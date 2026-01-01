use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub enum AppError {
    WalletAlreadyExists(String),
    WalletNotFound(String),
    InvalidInput(String),
    DatabaseError(String),
    InternalError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::WalletAlreadyExists(msg) => write!(f, "Wallet already exists: {}", msg),
            AppError::WalletNotFound(msg) => write!(f, "Wallet not found: {}", msg),
            AppError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl From<AppError> for rmcp::ErrorData {
    fn from(err: AppError) -> Self {
        let message = err.to_string();
        rmcp::ErrorData::new(rmcp::model::ErrorCode(-1), message, None)
    }
}
