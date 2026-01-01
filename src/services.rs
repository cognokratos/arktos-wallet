use crate::{crypto, db::Database, error::AppError, wallet_manager};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateWalletRequest {
    #[schemars(description = "The name of the wallet to be created.")]
    pub wallet_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWalletResponse {
    pub wallet_id: i64,
    pub wallet_name: String,
    pub created_at: String,
}

impl Display for CreateWalletResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Wallet Created: ID={}, Name={}, CreatedAt={}",
            self.wallet_id, self.wallet_name, self.created_at
        )
    }
}

pub struct Services {
    db: Arc<Database>,
    cipher_key: String,
}

impl Services {
    pub fn new(db: Arc<Database>, cipher_key: String) -> Self {
        Self { db, cipher_key }
    }

    /// Create a new wallet with encrypted recovery passphrase
    pub async fn create_wallet(
        &self,
        req: CreateWalletRequest,
    ) -> Result<CreateWalletResponse, AppError> {
        // Validate input
        if req.wallet_name.is_empty() {
            return Err(AppError::InvalidInput(
                "Wallet name cannot be empty".to_string(),
            ));
        }

        if req.wallet_name.len() > 255 {
            return Err(AppError::InvalidInput(
                "Wallet name exceeds maximum length of 255 characters".to_string(),
            ));
        }

        // Check if wallet already exists
        if self
            .db
            .get_wallet(&req.wallet_name)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .is_some()
        {
            return Err(AppError::WalletAlreadyExists(req.wallet_name));
        }

        // Generate recovery passphrase
        let recovery_passphrase = wallet_manager::generate_recovery_passphrase().map_err(|e| {
            AppError::InternalError(format!("Failed to generate passphrase: {}", e))
        })?;

        // Encrypt the passphrase
        let encrypted_passphrase =
            crypto::encrypt_passphrase(&recovery_passphrase, &self.cipher_key).map_err(|e| {
                AppError::InternalError(format!("Failed to encrypt passphrase: {}", e))
            })?;

        // Create wallet in database
        let wallet = self
            .db
            .create_wallet(&req.wallet_name, &encrypted_passphrase)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(CreateWalletResponse {
            wallet_id: wallet.id,
            wallet_name: wallet.name,
            created_at: wallet.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_wallet_successfully() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let db = Arc::new(
            Database::new(&db_path, "test_cipher_key").expect("Failed to create database"),
        );

        let handler = Services::new(db, "test_cipher_key".to_string());
        let req = CreateWalletRequest {
            wallet_name: "MyWallet".to_string(),
        };

        let response = handler.create_wallet(req).await;
        assert!(response.is_ok());

        let resp = response.unwrap();
        assert_eq!(resp.wallet_name, "MyWallet");
    }

    #[tokio::test]
    async fn test_create_wallet_empty_name_fails() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let db = Arc::new(
            Database::new(&db_path, "test_cipher_key").expect("Failed to create database"),
        );

        let handler = Services::new(db, "test_cipher_key".to_string());
        let req = CreateWalletRequest {
            wallet_name: "".to_string(),
        };

        let response = handler.create_wallet(req).await;
        assert!(response.is_err());
    }

    #[tokio::test]
    async fn test_create_wallet_duplicate_fails() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let db = Arc::new(
            Database::new(&db_path, "test_cipher_key").expect("Failed to create database"),
        );

        let handler = Services::new(db, "test_cipher_key".to_string());

        let req1 = CreateWalletRequest {
            wallet_name: "MyWallet".to_string(),
        };

        let req2 = CreateWalletRequest {
            wallet_name: "MyWallet".to_string(),
        };

        // First creation should succeed
        let response1 = handler.create_wallet(req1).await;
        assert!(response1.is_ok());

        // Second creation with same name should fail
        let response2 = handler.create_wallet(req2).await;
        assert!(response2.is_err());
    }
}
