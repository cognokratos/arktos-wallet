use crate::api_key::ApiKey;
use crate::wallet::{Account, ChainType};
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
            "Wallet Created: ID={}, Name=\"{}\", CreatedAt=\"{}\"",
            self.wallet_id, self.wallet_name, self.created_at
        )
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct GetBitcoinAddressRequest {
    #[schemars(description = "The wallet name to derive the Bitcoin address for.")]
    pub wallet_name: String,
    #[schemars(description = "The account index for derivation (default: 0).")]
    pub account_index: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitcoinAddressResponse {
    pub wallet_name: String,
    pub account_index: u32,
    pub bitcoin_address: String,
    pub created_at: String,
}

impl Display for BitcoinAddressResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BitcoinAddress: Wallet=\"{}\", Index={}, Address=\"{}\", CreatedAt=\"{}\"",
            self.wallet_name, self.account_index, self.bitcoin_address, self.created_at
        )
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct GetEthereumAddressRequest {
    #[schemars(description = "The wallet name to derive the Ethereum address for.")]
    pub wallet_name: String,
    #[schemars(description = "The account index for derivation (default: 0).")]
    pub account_index: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EthereumAddressResponse {
    pub wallet_name: String,
    pub account_index: u32,
    pub ethereum_address: String,
    pub created_at: String,
}

impl Display for EthereumAddressResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EthereumAddress: Wallet=\"{}\", Index={}, Address=\"{}\", CreatedAt=\"{}\"",
            self.wallet_name, self.account_index, self.ethereum_address, self.created_at
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

    /// Get or derive Bitcoin address for a wallet
    pub async fn get_bitcoin_address(
        &self,
        api_key: &ApiKey,
        req: GetBitcoinAddressRequest,
    ) -> Result<BitcoinAddressResponse, AppError> {
        let bitcoin_account = self
            .create_or_get_account(
                api_key,
                req.wallet_name.clone(),
                req.account_index.unwrap_or(0),
                ChainType::Bitcoin,
            )
            .await?;

        Ok(BitcoinAddressResponse {
            wallet_name: req.wallet_name,
            account_index: bitcoin_account.account_index,
            bitcoin_address: bitcoin_account.address,
            created_at: bitcoin_account.created_at,
        })
    }

    /// Get or derive Ethereum address for a wallet
    pub async fn get_ethereum_address(
        &self,
        api_key: &ApiKey,
        req: GetEthereumAddressRequest,
    ) -> Result<EthereumAddressResponse, AppError> {
        let ethereum_account = self
            .create_or_get_account(
                api_key,
                req.wallet_name.clone(),
                req.account_index.unwrap_or(0),
                ChainType::Ethereum,
            )
            .await?;

        Ok(EthereumAddressResponse {
            wallet_name: req.wallet_name,
            account_index: ethereum_account.account_index,
            ethereum_address: ethereum_account.address,
            created_at: ethereum_account.created_at,
        })
    }

    /// Create or retrieve an account for a wallet with derived keys
    async fn create_or_get_account(
        &self,
        api_key: &ApiKey,
        wallet_name: String,
        account_index: u32,
        chain_type: ChainType,
    ) -> Result<Account, AppError> {
        // Check if wallet exists
        let wallet = self
            .store
            .get_wallet(api_key.id, &wallet_name)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::WalletNotFound(format!("Wallet Name: {}", wallet_name)))?;

        // Check if account already exists
        if let Ok(Some(account)) = self
            .store
            .get_account(wallet.id, account_index, &chain_type)
        {
            return Ok(account);
        }

        // Decrypt the wallet's passphrase
        let decrypted_passphrase =
            crypto::decrypt_secret(&wallet.encrypted_passphrase, &self.secret_key).map_err(
                |e| AppError::InternalError(format!("Failed to decrypt passphrase: {}", e)),
            )?;

        let account_data =
            wallet_manager::derive_account_keys(&decrypted_passphrase, account_index, &chain_type)
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to derive account keys: {}", e))
                })?;

        // Encrypt the derived private key
        let encrypted_private_key =
            crypto::encrypt_secret(&account_data.private_key, &self.secret_key).map_err(|e| {
                AppError::InternalError(format!("Failed to encrypt private key: {}", e))
            })?;

        // Store account in database
        let account = self
            .store
            .create_account(
                wallet.id,
                account_index,
                &account_data.address,
                &account_data.public_key,
                &encrypted_private_key,
                &chain_type,
            )
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(account)
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

        let account_resp = services
            .create_or_get_account(&api_key, "TestWallet".to_string(), 0, Bitcoin)
            .await;
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

        services.create_wallet(&api_key, wallet_req).await.unwrap();

        let acc1 = services
            .create_or_get_account(&api_key, "TestWallet".to_string(), 0, Bitcoin)
            .await
            .unwrap();

        let acc2 = services
            .create_or_get_account(&api_key, "TestWallet".to_string(), 0, Bitcoin)
            .await
            .unwrap();

        // Both should have the same public key (they're the same account)
        assert_eq!(acc1.public_key, acc2.public_key);
        assert_eq!(acc1.id, acc2.id);
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

        let result = services
            .create_or_get_account(&api_key, "MissingWallet".to_string(), 0, Bitcoin)
            .await;
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

        services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Create Ethereum account
        let acc = services
            .create_or_get_account(&api_key, "EthWallet".to_string(), 0, Ethereum)
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
            wallet_name: "BitcoinWallet".to_string(),
            account_index: Some(0),
        };

        let addr_resp = services
            .get_bitcoin_address(&api_key, addr_req)
            .await
            .expect("Should get Bitcoin address");

        assert_eq!(addr_resp.wallet_name, wallet_resp.wallet_name);
        assert_eq!(addr_resp.account_index, 0);
        assert!(
            addr_resp.bitcoin_address.starts_with("bc1"),
            "Bitcoin address should start with bc1"
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
            wallet_name: wallet_resp.wallet_name.clone(),
            account_index: Some(0),
        };
        let addr1 = services
            .get_bitcoin_address(&api_key, addr_req1)
            .await
            .unwrap();

        // Get Bitcoin address again
        let addr_req2 = GetBitcoinAddressRequest {
            wallet_name: wallet_resp.wallet_name.clone(),
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
            wallet_name: "MissingWallet".to_string(),
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
            wallet_name: wallet_resp.wallet_name.clone(),
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
        assert!(addr_resp.bitcoin_address.starts_with("bc1"));
    }

    #[tokio::test]
    async fn test_get_ethereum_address_creates_address() {
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
            wallet_name: "EthereumWallet".to_string(),
        };
        let wallet_resp = services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Get Ethereum address
        let addr_req = GetEthereumAddressRequest {
            wallet_name: wallet_resp.wallet_name.clone(),
            account_index: Some(0),
        };

        let addr_resp = services
            .get_ethereum_address(&api_key, addr_req)
            .await
            .expect("Should get Ethereum address");

        assert_eq!(addr_resp.wallet_name, wallet_resp.wallet_name);
        assert_eq!(addr_resp.account_index, 0);
        assert!(addr_resp.ethereum_address.starts_with("0x"));
        assert_eq!(addr_resp.ethereum_address.len(), 42); // 0x + 40 hex chars
    }

    #[tokio::test]
    async fn test_get_ethereum_address_returns_existing() {
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
            wallet_name: "EthereumWallet2".to_string(),
        };
        let wallet_resp = services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Get Ethereum address twice
        let addr_req1 = GetEthereumAddressRequest {
            wallet_name: wallet_resp.wallet_name.clone(),
            account_index: Some(0),
        };

        let addr_resp1 = services
            .get_ethereum_address(&api_key, addr_req1)
            .await
            .expect("Should get Ethereum address");

        let addr_req2 = GetEthereumAddressRequest {
            wallet_name: wallet_resp.wallet_name.clone(),
            account_index: Some(0),
        };

        let addr_resp2 = services
            .get_ethereum_address(&api_key, addr_req2)
            .await
            .expect("Should get same Ethereum address");

        // Addresses should be identical (deterministic derivation)
        assert_eq!(addr_resp1.ethereum_address, addr_resp2.ethereum_address);
    }

    #[tokio::test]
    async fn test_get_ethereum_address_fails_for_nonexistent_wallet() {
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
        let addr_req = GetEthereumAddressRequest {
            wallet_name: "NonexistentWallet".to_string(),
            account_index: Some(0),
        };

        let result = services.get_ethereum_address(&api_key, addr_req).await;
        assert!(result.is_err(), "Should fail for nonexistent wallet");
    }

    #[tokio::test]
    async fn test_get_ethereum_address_with_default_account_index() {
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
            wallet_name: "EthereumWallet3".to_string(),
        };
        let wallet_resp = services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Get Ethereum address without specifying account index
        let addr_req = GetEthereumAddressRequest {
            wallet_name: wallet_resp.wallet_name.clone(),
            account_index: None,
        };

        let addr_resp = services
            .get_ethereum_address(&api_key, addr_req)
            .await
            .expect("Should get Ethereum address with default index");

        assert_eq!(
            addr_resp.account_index, 0,
            "Should default to account index 0"
        );
        assert!(addr_resp.ethereum_address.starts_with("0x"));
        assert_eq!(addr_resp.ethereum_address.len(), 42);
    }

    #[tokio::test]
    async fn test_get_ethereum_address_performance_nfr2() {
        use std::time::Instant;

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
            wallet_name: "PerfTestWallet".to_string(),
        };
        let wallet_resp = services.create_wallet(&api_key, wallet_req).await.unwrap();

        // Warm-up call
        let addr_req = GetEthereumAddressRequest {
            wallet_name: wallet_resp.wallet_name.clone(),
            account_index: Some(0),
        };
        let _ = services
            .get_ethereum_address(&api_key, addr_req.clone())
            .await;

        // Performance test: measure 100 consecutive calls
        let mut times = Vec::new();
        for _ in 0..100 {
            let start = Instant::now();
            let _ = services
                .get_ethereum_address(&api_key, addr_req.clone())
                .await;
            let elapsed = start.elapsed().as_millis();
            times.push(elapsed as u64);
        }

        times.sort();
        let p95_idx = std::cmp::min(95, times.len() - 1);
        let p95 = times[p95_idx];

        println!("get_ethereum_address Performance (NFR2):");
        println!("  Min: {}ms", times[0]);
        println!("  P95: {}ms", p95);
        println!("  Max: {}ms", times[times.len() - 1]);

        assert!(
            p95 < 100,
            "P95 latency {} ms exceeds 100ms NFR2 threshold",
            p95
        );
    }
}
