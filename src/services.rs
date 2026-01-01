use crate::models::ChainType;
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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateAccountRequest {
    pub wallet_id: i64,
    pub account_index: u32,
    pub chain_type: ChainType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountResponse {
    pub account_id: i64,
    pub wallet_id: i64,
    pub account_index: u32,
    pub public_key: String,
    pub chain_type: ChainType,
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

    /// Create or retrieve an account for a wallet with derived keys
    pub async fn create_or_get_account(
        &self,
        req: CreateAccountRequest,
    ) -> Result<AccountResponse, AppError> {
        // Check if wallet exists
        let wallet = self
            .db
            .get_wallet_by_id(req.wallet_id)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::WalletNotFound(format!("Wallet ID: {}", req.wallet_id)))?;

        // Check if account already exists
        if let Ok(Some(account)) =
            self.db
                .get_account(req.wallet_id, req.account_index as i32, &req.chain_type)
        {
            return Ok(AccountResponse {
                account_id: account.id,
                wallet_id: account.wallet_id,
                account_index: req.account_index,
                public_key: account.public_key,
                chain_type: account.chain_type,
                created_at: account.created_at,
            });
        }

        // Decrypt the wallet's passphrase
        let decrypted_passphrase =
            crypto::decrypt_passphrase(&wallet.encrypted_passphrase, &self.cipher_key).map_err(
                |e| AppError::InternalError(format!("Failed to decrypt passphrase: {}", e)),
            )?;

        // Derive account keys using BIP32/BIP44
        let (private_key_hex, public_key_hex, _) = wallet_manager::derive_account_keys(
            &decrypted_passphrase,
            req.account_index,
            &req.chain_type,
        )
        .map_err(|e| AppError::InternalError(format!("Failed to derive account keys: {}", e)))?;

        // Encrypt the derived private key
        let encrypted_private_key = crypto::encrypt_passphrase(&private_key_hex, &self.cipher_key)
            .map_err(|e| {
                AppError::InternalError(format!("Failed to encrypt private key: {}", e))
            })?;

        // Store account in database
        let account = self
            .db
            .create_account(
                req.wallet_id,
                req.account_index as i32,
                &encrypted_private_key,
                &public_key_hex,
                &req.chain_type,
            )
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(AccountResponse {
            account_id: account.id,
            wallet_id: account.wallet_id,
            account_index: req.account_index,
            public_key: account.public_key,
            chain_type: account.chain_type,
            created_at: account.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ChainType::{Bitcoin, Ethereum};
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

    #[tokio::test]
    async fn test_create_or_get_account_creates_account() {
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

        let handler = Services::new(db.clone(), "test_cipher_key".to_string());

        // Create a wallet first
        let wallet_req = CreateWalletRequest {
            wallet_name: "TestWallet".to_string(),
        };
        let wallet_resp = handler.create_wallet(wallet_req).await.unwrap();

        // Now create an account
        let account_req = CreateAccountRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: 0,
            chain_type: Bitcoin,
        };

        let account_resp = handler.create_or_get_account(account_req).await;
        assert!(account_resp.is_ok());

        let acc = account_resp.unwrap();
        assert_eq!(acc.wallet_id, wallet_resp.wallet_id);
        assert_eq!(acc.account_index, 0);
        assert_eq!(acc.chain_type, Bitcoin);
        assert!(!acc.public_key.is_empty());
    }

    #[tokio::test]
    async fn test_create_or_get_account_returns_existing() {
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

        let handler = Services::new(db.clone(), "test_cipher_key".to_string());

        // Create a wallet
        let wallet_req = CreateWalletRequest {
            wallet_name: "TestWallet".to_string(),
        };
        let wallet_resp = handler.create_wallet(wallet_req).await.unwrap();

        // Create account first time
        let account_req1 = CreateAccountRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: 0,
            chain_type: Bitcoin,
        };
        let acc1 = handler.create_or_get_account(account_req1).await.unwrap();

        // Try to create same account again
        let account_req2 = CreateAccountRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: 0,
            chain_type: Bitcoin,
        };
        let acc2 = handler.create_or_get_account(account_req2).await.unwrap();

        // Both should have the same public key (they're the same account)
        assert_eq!(acc1.public_key, acc2.public_key);
        assert_eq!(acc1.account_id, acc2.account_id);
    }

    #[tokio::test]
    async fn test_create_or_get_account_fails_for_nonexistent_wallet() {
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

        // Try to create account for non-existent wallet
        let account_req = CreateAccountRequest {
            wallet_id: 999,
            account_index: 0,
            chain_type: Bitcoin,
        };

        let result = handler.create_or_get_account(account_req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_or_get_account_ethereum_derivation() {
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

        let handler = Services::new(db.clone(), "test_cipher_key".to_string());

        // Create a wallet
        let wallet_req = CreateWalletRequest {
            wallet_name: "EthWallet".to_string(),
        };
        let wallet_resp = handler.create_wallet(wallet_req).await.unwrap();

        // Create Ethereum account
        let account_req = CreateAccountRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: 0,
            chain_type: Ethereum,
        };

        let acc = handler.create_or_get_account(account_req).await.unwrap();
        assert_eq!(acc.chain_type, Ethereum);
        assert!(!acc.public_key.is_empty());
    }
}
