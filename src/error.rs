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

impl From<crate::database::StoreError> for AppError {
    /// Generic mapping for unexpected persistence failures. Details are logged
    /// by the caller; the client only sees a safe category. Expected cases
    /// (duplicates, missing records) are mapped explicitly by the services.
    fn from(err: crate::database::StoreError) -> Self {
        use crate::database::StoreError;
        match err {
            StoreError::Unavailable(_) => AppError::DatabaseError("database unavailable".into()),
            StoreError::CorruptData(_) => AppError::DatabaseError("stored data is invalid".into()),
            _ => AppError::DatabaseError("database error".into()),
        }
    }
}

impl From<AppError> for rmcp::ErrorData {
    fn from(err: AppError) -> Self {
        let message = err.to_string();
        rmcp::ErrorData::new(rmcp::model::ErrorCode(-1), message, None)
    }
}
