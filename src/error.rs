//! Domain errors and their mapping to MCP.
//!
//! ```text
//! client errors  (invalid_argument | not_found | already_exists)
//!     → tool execution error: CallToolResult { isError: true,
//!       content: [text: {"error":{"code":"…","message":"…"}}] }
//!       so the calling model can read it and correct its call
//! server faults  (storage | crypto | derivation | internal)
//!     → JSON-RPC error -32603 "internal error"; details are only logged
//! ```

use crate::database::StoreError;
use crate::domain::ValidationError;
use rmcp::handler::server::tool::IntoCallToolResult;
use rmcp::model::{CallToolResponse, CallToolResult, ContentBlock};
use serde_json::json;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// A request field failed validation.
    InvalidArgument {
        field: &'static str,
        error: ValidationError,
    },
    /// The caller has no wallet with this name.
    WalletNotFound {
        wallet_name: String,
    },
    /// The caller already has a wallet with this name.
    WalletAlreadyExists {
        wallet_name: String,
    },
    /// Unexpected persistence failure.
    Storage(StoreError),
    /// Encrypting or decrypting a wallet secret failed.
    Crypto(String),
    /// Key derivation failed (e.g. invalid stored mnemonic).
    DerivationFailed(String),
    Internal(String),
}

impl AppError {
    pub fn invalid(field: &'static str, error: ValidationError) -> Self {
        AppError::InvalidArgument { field, error }
    }

    /// Stable machine-readable code returned to clients.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::InvalidArgument { .. } => "invalid_argument",
            AppError::WalletNotFound { .. } => "not_found",
            AppError::WalletAlreadyExists { .. } => "already_exists",
            _ => "internal",
        }
    }

    /// Whether the caller can fix the request (vs. a server fault).
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            AppError::InvalidArgument { .. }
                | AppError::WalletNotFound { .. }
                | AppError::WalletAlreadyExists { .. }
        )
    }

    /// Message safe to show to clients (no internal details).
    pub fn client_message(&self) -> String {
        match self {
            AppError::InvalidArgument { field, error } => format!("{field} {error}"),
            AppError::WalletNotFound { wallet_name } => {
                format!("wallet '{wallet_name}' not found")
            }
            AppError::WalletAlreadyExists { wallet_name } => {
                format!("wallet '{wallet_name}' already exists")
            }
            _ => "internal error".to_string(),
        }
    }
}

/// Full description for logs (never contains secrets).
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Storage(e) => write!(f, "storage failure: {e}"),
            AppError::Crypto(e) => write!(f, "crypto failure: {e}"),
            AppError::DerivationFailed(e) => write!(f, "account derivation failed: {e}"),
            AppError::Internal(e) => write!(f, "internal error: {e}"),
            client => f.write_str(&client.client_message()),
        }
    }
}

impl std::error::Error for AppError {}

impl From<StoreError> for AppError {
    fn from(err: StoreError) -> Self {
        AppError::Storage(err)
    }
}

impl IntoCallToolResult for AppError {
    fn into_call_tool_result(self) -> Result<CallToolResponse, rmcp::ErrorData> {
        if self.is_client_error() {
            let body =
                json!({ "error": { "code": self.code(), "message": self.client_message() } });
            Ok(CallToolResult::error(vec![ContentBlock::text(body.to_string())]).into())
        } else {
            Err(rmcp::ErrorData::internal_error("internal error", None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_errors_become_tool_errors_with_codes() {
        let cases = [
            (
                AppError::invalid("wallet_name", ValidationError::Empty),
                "invalid_argument",
                "wallet_name must not be empty",
            ),
            (
                AppError::WalletNotFound {
                    wallet_name: "main".into(),
                },
                "not_found",
                "wallet 'main' not found",
            ),
            (
                AppError::WalletAlreadyExists {
                    wallet_name: "main".into(),
                },
                "already_exists",
                "wallet 'main' already exists",
            ),
        ];
        for (error, code, message) in cases {
            let CallToolResponse::Complete(result) = error.into_call_tool_result().unwrap() else {
                panic!("expected a complete result");
            };
            assert_eq!(result.is_error, Some(true));
            let text = &result.content[0].as_text().unwrap().text;
            let body: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(body["error"]["code"], code);
            assert_eq!(body["error"]["message"], message);
        }
    }

    #[test]
    fn server_faults_hide_details() {
        for error in [
            AppError::Storage(StoreError::Internal(
                "UNIQUE constraint failed: accounts".into(),
            )),
            AppError::Crypto("failed to decrypt secret".into()),
            AppError::DerivationFailed("Invalid mnemonic".into()),
            AppError::Internal("boom".into()),
        ] {
            let err = error.into_call_tool_result().unwrap_err();
            assert_eq!(err.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
            assert_eq!(err.message, "internal error");
            assert!(err.data.is_none());
        }
    }
}
