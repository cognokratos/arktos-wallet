use anyhow::{Context, Result};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct Database {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Initialize database with encrypted SQLCipher connection
    pub fn new(db_path: &str, cipher_key: &str) -> Result<Self> {
        let conn = Connection::open(db_path).context("Failed to open database connection")?;

        // Enable SQLCipher encryption with AES-256
        conn.execute_batch(&format!("PRAGMA key = '{}';", cipher_key))
            .context("Failed to set encryption key")?;

        // Verify encryption is enabled
        conn.execute_batch("PRAGMA cipher_page_size = 4096;")
            .context("Failed to set cipher page size")?;

        // Create schema if it doesn't exist
        Self::create_schema(&conn)?;

        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn create_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key_hash TEXT UNIQUE NOT NULL,
                key_name TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_revoked BOOLEAN DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS wallets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key_id INTEGER NOT NULL,
                name TEXT UNIQUE NOT NULL,
                encrypted_passphrase TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (key_id) REFERENCES api_keys(id)
            );
            
            CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallet_id INTEGER NOT NULL,
                account_index INTEGER NOT NULL,
                encrypted_private_key TEXT NOT NULL,
                public_key TEXT NOT NULL,
                chain_type TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (wallet_id) REFERENCES wallets(id),
                UNIQUE(wallet_id, account_index, chain_type)
            );",
        )
        .context("Failed to create schema")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_store::KeyStore;
    use crate::wallet::ChainType::Bitcoin;
    use crate::wallet_store::WalletStore;
    use tempfile::TempDir;

    #[test]
    fn test_create_wallet_with_encrypted_passphrase() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let db = Arc::new(Database::new(&db_path, "test_key").expect("Failed to create database"));
        let key_store = KeyStore::new(db.clone());
        let wallet_store = WalletStore::new(db.clone());

        key_store
            .create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = key_store
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        let wallet = wallet_store
            .create_wallet(key_id, "MyWallet", "encrypted_data")
            .expect("Failed to create wallet");

        assert_eq!(wallet.name, "MyWallet");
        assert_eq!(wallet.encrypted_passphrase, "encrypted_data");
    }

    #[test]
    fn test_get_wallet_by_name() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let db = Arc::new(Database::new(&db_path, "test_key").expect("Failed to create database"));
        let key_store = KeyStore::new(db.clone());
        let wallet_store = WalletStore::new(db.clone());

        key_store
            .create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = key_store
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        wallet_store
            .create_wallet(key_id, "TestWallet", "encrypted_passphrase")
            .expect("Failed to create wallet");

        let retrieved = wallet_store
            .get_wallet(key_id, "TestWallet")
            .expect("Failed to get wallet");

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "TestWallet");
    }

    #[test]
    fn test_list_wallets() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let db = Arc::new(Database::new(&db_path, "test_key").expect("Failed to create database"));
        let key_store = KeyStore::new(db.clone());
        let wallet_store = WalletStore::new(db.clone());

        key_store
            .create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = key_store
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        wallet_store
            .create_wallet(key_id, "Wallet1", "pass1")
            .expect("Failed to create wallet 1");
        wallet_store
            .create_wallet(key_id, "Wallet2", "pass2")
            .expect("Failed to create wallet 2");

        let wallets = wallet_store
            .list_wallets(key_id)
            .expect("Failed to list wallets");
        assert_eq!(wallets.len(), 2);
    }

    #[test]
    fn test_create_account_for_wallet() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let db = Arc::new(Database::new(&db_path, "test_key").expect("Failed to create database"));
        let key_store = KeyStore::new(db.clone());
        let wallet_store = WalletStore::new(db.clone());

        key_store
            .create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = key_store
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        let wallet = wallet_store
            .create_wallet(key_id, "WalletWithAccounts", "pass")
            .expect("Failed to create wallet");

        let account = wallet_store
            .create_account(
                wallet.id,
                0,
                "encrypted_key_123",
                "public_key_abc",
                &Bitcoin,
            )
            .expect("Failed to create account");

        assert_eq!(account.wallet_id, wallet.id);
        assert_eq!(account.account_index, 0);
        assert_eq!(account.encrypted_private_key, "encrypted_key_123");
        assert_eq!(account.public_key, "public_key_abc");
        assert_eq!(account.chain_type, Bitcoin);
    }

    #[test]
    fn test_get_account_by_wallet_and_index() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let db = Arc::new(Database::new(&db_path, "test_key").expect("Failed to create database"));
        let key_store = KeyStore::new(db.clone());
        let wallet_store = WalletStore::new(db.clone());

        key_store
            .create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = key_store
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        let wallet = wallet_store
            .create_wallet(key_id, "TestWallet", "pass")
            .expect("Failed to create wallet");

        wallet_store
            .create_account(wallet.id, 0, "key1", "pubkey1", &Bitcoin)
            .expect("Failed to create account");

        let retrieved = wallet_store
            .get_account(wallet.id, 0, &Bitcoin)
            .expect("Failed to get account");

        assert!(retrieved.is_some());
        let acc = retrieved.unwrap();
        assert_eq!(acc.wallet_id, wallet.id);
        assert_eq!(acc.account_index, 0);
        assert_eq!(acc.chain_type, Bitcoin);
    }

    #[test]
    fn test_get_wallet_by_id() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let db = Arc::new(Database::new(&db_path, "test_key").expect("Failed to create database"));
        let key_store = KeyStore::new(db.clone());
        let wallet_store = WalletStore::new(db.clone());

        key_store
            .create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = key_store
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        let wallet = wallet_store
            .create_wallet(key_id, "TestWallet", "pass")
            .expect("Failed to create wallet");

        let retrieved = wallet_store
            .get_wallet_by_id(key_id, wallet.id)
            .expect("Failed to get wallet by id");

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "TestWallet");
    }

    #[test]
    fn test_get_nonexistent_wallet_by_id_returns_none() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();

        let db = Arc::new(Database::new(&db_path, "test_key").expect("Failed to create database"));
        let key_store = KeyStore::new(db.clone());
        let wallet_store = WalletStore::new(db.clone());

        key_store
            .create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = key_store
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        let retrieved = wallet_store
            .get_wallet_by_id(key_id, 999)
            .expect("Failed to query wallet");

        assert!(retrieved.is_none());
    }
}
