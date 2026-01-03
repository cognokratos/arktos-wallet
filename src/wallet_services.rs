use crate::api_key::ApiKey;
use crate::wallet::ChainType;
use crate::wallet_store::WalletStore;
use crate::{crypto, database::Database, error::AppError, wallet_manager};
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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetAccountRequest {
    pub wallet_id: i64,
    pub account_index: u32,
    pub chain_type: ChainType,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct GetBitcoinAddressRequest {
    #[schemars(description = "The wallet ID to derive the Bitcoin address for.")]
    pub wallet_id: i64,
    #[schemars(description = "The account index for derivation (default: 0).")]
    pub account_index: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitcoinAddressResponse {
    pub wallet_id: i64,
    pub account_index: u32,
    pub bitcoin_address: String,
    pub created_at: String,
}

impl Display for BitcoinAddressResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BitcoinAddress: WalletID={}, Index={}, Address={}, CreatedAt={}",
            self.wallet_id, self.account_index, self.bitcoin_address, self.created_at
        )
    }
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

impl Display for AccountResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Account: ID={}, WalletID={}, Index={}, ChainType={}, PublicKey={}, CreatedAt={}",
            self.account_id,
            self.wallet_id,
            self.account_index,
            self.chain_type,
            self.public_key,
            self.created_at
        )
    }
}

pub struct WalletServices {
    store: WalletStore,
    secret_key: String,
}

impl WalletServices {
    pub fn new(db: Arc<Database>, secret_key: String) -> Self {
        Self {
            store: WalletStore::new(db),
            secret_key,
        }
    }

    /// Create a new wallet with encrypted recovery passphrase
    pub async fn create_wallet(
        &self,
        api_key: &ApiKey,
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
            .store
            .get_wallet(api_key.id, &req.wallet_name)
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
        let encrypted_passphrase = crypto::encrypt_secret(&recovery_passphrase, &self.secret_key)
            .map_err(|e| {
            AppError::InternalError(format!("Failed to encrypt passphrase: {}", e))
        })?;

        // Create wallet in database
        let wallet = self
            .store
            .create_wallet(api_key.id, &req.wallet_name, &encrypted_passphrase)
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
        api_key: &ApiKey,
        req: GetAccountRequest,
    ) -> Result<AccountResponse, AppError> {
        // Check if wallet exists
        let wallet = self
            .store
            .get_wallet_by_id(api_key.id, req.wallet_id)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::WalletNotFound(format!("Wallet ID: {}", req.wallet_id)))?;

        // Check if account already exists
        if let Ok(Some(account)) =
            self.store
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
            crypto::decrypt_secret(&wallet.encrypted_passphrase, &self.secret_key).map_err(
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
        let encrypted_private_key = crypto::encrypt_secret(&private_key_hex, &self.secret_key)
            .map_err(|e| {
                AppError::InternalError(format!("Failed to encrypt private key: {}", e))
            })?;

        // Store account in database
        let account = self
            .store
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

    /// Get or derive Bitcoin address for a wallet
    pub async fn get_bitcoin_address(
        &self,
        api_key: &ApiKey,
        req: GetBitcoinAddressRequest,
    ) -> Result<BitcoinAddressResponse, AppError> {
        let account_index = req.account_index.unwrap_or(0);

        // Check if wallet exists
        let wallet = self
            .store
            .get_wallet_by_id(api_key.id, req.wallet_id)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::WalletNotFound(format!("Wallet ID: {}", req.wallet_id)))?;

        // Check if Bitcoin account already exists
        if let Ok(Some(account)) =
            self.store
                .get_account(req.wallet_id, account_index as i32, &ChainType::Bitcoin)
        {
            // Derive address from existing public key
            let bitcoin_address = wallet_manager::derive_bitcoin_address(&account.public_key)
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to derive Bitcoin address: {}", e))
                })?;

            return Ok(BitcoinAddressResponse {
                wallet_id: req.wallet_id,
                account_index,
                bitcoin_address,
                created_at: account.created_at,
            });
        }

        // Decrypt the wallet's passphrase
        let decrypted_passphrase =
            crypto::decrypt_secret(&wallet.encrypted_passphrase, &self.secret_key).map_err(
                |e| AppError::InternalError(format!("Failed to decrypt passphrase: {}", e)),
            )?;

        // Derive account keys using BIP32/BIP44 for Bitcoin
        let (private_key_hex, public_key_hex, _) = wallet_manager::derive_account_keys(
            &decrypted_passphrase,
            account_index,
            &ChainType::Bitcoin,
        )
        .map_err(|e| AppError::InternalError(format!("Failed to derive account keys: {}", e)))?;

        // Encrypt the derived private key
        let encrypted_private_key = crypto::encrypt_secret(&private_key_hex, &self.secret_key)
            .map_err(|e| {
                AppError::InternalError(format!("Failed to encrypt private key: {}", e))
            })?;

        // Store account in database
        let account = self
            .store
            .create_account(
                req.wallet_id,
                account_index as i32,
                &encrypted_private_key,
                &public_key_hex,
                &ChainType::Bitcoin,
            )
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Derive Bitcoin address from public key
        let bitcoin_address =
            wallet_manager::derive_bitcoin_address(&account.public_key).map_err(|e| {
                AppError::InternalError(format!("Failed to derive Bitcoin address: {}", e))
            })?;

        Ok(BitcoinAddressResponse {
            wallet_id: req.wallet_id,
            account_index,
            bitcoin_address,
            created_at: account.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_services::KeyServices;
    use crate::wallet::ChainType::{Bitcoin, Ethereum};
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

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);
        let req = CreateWalletRequest {
            wallet_name: "MyWallet".to_string(),
        };
        let response = services.create_wallet(&api_key, req).await;
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

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);
        let req = CreateWalletRequest {
            wallet_name: "".to_string(),
        };
        let response = services.create_wallet(&api_key, req).await;
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

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);

        let req1 = CreateWalletRequest {
            wallet_name: "MyWallet".to_string(),
        };

        let req2 = CreateWalletRequest {
            wallet_name: "MyWallet".to_string(),
        };

        // First creation should succeed
        let response1 = services.create_wallet(&api_key, req1).await;
        assert!(response1.is_ok());

        // Second creation with same name should fail
        let response2 = services.create_wallet(&api_key, req2).await;
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

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);

        // Create a wallet first
        let wallet_req = CreateWalletRequest {
            wallet_name: "TestWallet".to_string(),
        };
        let wallet_resp = services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Now create an account
        let account_req = GetAccountRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: 0,
            chain_type: Bitcoin,
        };

        let account_resp = services.create_or_get_account(&api_key, account_req).await;
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

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);

        // Create a wallet
        let wallet_req = CreateWalletRequest {
            wallet_name: "TestWallet".to_string(),
        };
        let wallet_resp = services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Create account first time
        let account_req1 = GetAccountRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: 0,
            chain_type: Bitcoin,
        };
        let acc1 = services
            .create_or_get_account(&api_key, account_req1)
            .await
            .unwrap();

        // Try to create same account again
        let account_req2 = GetAccountRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: 0,
            chain_type: Bitcoin,
        };
        let acc2 = services
            .create_or_get_account(&api_key, account_req2)
            .await
            .unwrap();

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

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);

        // Try to create account for non-existent wallet
        let account_req = GetAccountRequest {
            wallet_id: 999,
            account_index: 0,
            chain_type: Bitcoin,
        };

        let result = services.create_or_get_account(&api_key, account_req).await;
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

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);

        // Create a wallet
        let wallet_req = CreateWalletRequest {
            wallet_name: "EthWallet".to_string(),
        };
        let wallet_resp = services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Create Ethereum account
        let account_req = GetAccountRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: 0,
            chain_type: Ethereum,
        };

        let acc = services
            .create_or_get_account(&api_key, account_req)
            .await
            .unwrap();
        assert_eq!(acc.chain_type, Ethereum);
        assert!(!acc.public_key.is_empty());
    }

    #[tokio::test]
    async fn test_get_bitcoin_address_creates_address() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);

        // Create a wallet
        let wallet_req = CreateWalletRequest {
            wallet_name: "BitcoinWallet".to_string(),
        };
        let wallet_resp = services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Get Bitcoin address
        let addr_req = GetBitcoinAddressRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: Some(0),
        };

        let addr_resp = services
            .get_bitcoin_address(&api_key, addr_req)
            .await
            .expect("Should get Bitcoin address");

        assert_eq!(addr_resp.wallet_id, wallet_resp.wallet_id);
        assert_eq!(addr_resp.account_index, 0);
        assert!(
            addr_resp.bitcoin_address.starts_with("1"),
            "Bitcoin address should start with 1"
        );
        assert!(!addr_resp.bitcoin_address.is_empty());
    }

    #[tokio::test]
    async fn test_get_bitcoin_address_returns_existing() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);

        // Create a wallet
        let wallet_req = CreateWalletRequest {
            wallet_name: "BitcoinWallet".to_string(),
        };
        let wallet_resp = services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Get Bitcoin address first time
        let addr_req1 = GetBitcoinAddressRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: Some(0),
        };
        let addr1 = services
            .get_bitcoin_address(&api_key, addr_req1)
            .await
            .unwrap();

        // Get Bitcoin address again
        let addr_req2 = GetBitcoinAddressRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: Some(0),
        };
        let addr2 = services
            .get_bitcoin_address(&api_key, addr_req2)
            .await
            .unwrap();

        // Both should return the same address
        assert_eq!(addr1.bitcoin_address, addr2.bitcoin_address);
    }

    #[tokio::test]
    async fn test_get_bitcoin_address_fails_for_nonexistent_wallet() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);

        // Try to get address for non-existent wallet
        let addr_req = GetBitcoinAddressRequest {
            wallet_id: 999,
            account_index: Some(0),
        };

        let result = services.get_bitcoin_address(&api_key, addr_req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_bitcoin_address_with_default_account_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let secret_key = "test_cipher_key".to_string();

        let db = Arc::new(Database::new(&db_path, &secret_key).expect("Failed to create database"));

        let key_store = KeyServices::new(db.clone(), secret_key.clone());
        let api_key = key_store
            .create("TestClient")
            .await
            .expect("Failed to create API key");
        let api_key = key_store
            .validate(&api_key)
            .await
            .expect("Failed to get API key");

        let services = WalletServices::new(db, secret_key);

        // Create a wallet
        let wallet_req = CreateWalletRequest {
            wallet_name: "BitcoinWallet".to_string(),
        };
        let wallet_resp = services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Get Bitcoin address without specifying account index
        let addr_req = GetBitcoinAddressRequest {
            wallet_id: wallet_resp.wallet_id,
            account_index: None,
        };

        let addr_resp = services
            .get_bitcoin_address(&api_key, addr_req)
            .await
            .expect("Should get Bitcoin address with default index");

        assert_eq!(
            addr_resp.account_index, 0,
            "Should default to account index 0"
        );
        assert!(addr_resp.bitcoin_address.starts_with("1"));
    }
}
