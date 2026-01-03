use crate::models::{Account, ChainType, Wallet};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
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

    /// Create a wallet with encrypted passphrase
    pub fn create_wallet(
        &self,
        key_id: i64,
        name: &str,
        encrypted_passphrase: &str,
    ) -> Result<Wallet> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO wallets (key_id, name, encrypted_passphrase) VALUES (?1, ?2, ?3)",
            params![key_id, name, encrypted_passphrase],
        )
        .context("Failed to insert wallet")?;

        let id = conn.last_insert_rowid();

        Ok(Wallet {
            id,
            name: name.to_string(),
            encrypted_passphrase: encrypted_passphrase.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Get wallet by name
    pub fn get_wallet(&self, key_id: i64, name: &str) -> Result<Option<Wallet>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT id, name, encrypted_passphrase, created_at FROM wallets WHERE key_id = ?1 AND name = ?2",
            )
            .context("Failed to prepare GET_WALLET statement")?;

        let wallet = stmt
            .query_row(params![key_id, name], |row| {
                Ok(Wallet {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    encrypted_passphrase: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .optional()
            .context("Failed to query wallet")?;

        Ok(wallet)
    }

    /// Get wallet by ID
    pub fn get_wallet_by_id(&self, key_id: i64, wallet_id: i64) -> Result<Option<Wallet>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT id, name, encrypted_passphrase, created_at FROM wallets WHERE key_id = ?1 AND id = ?2")
            .context("Failed to prepare GET_WALLET_BY_ID statement")?;

        let wallet = stmt
            .query_row(params![key_id, wallet_id], |row| {
                Ok(Wallet {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    encrypted_passphrase: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .optional()
            .context("Failed to query wallet")?;

        Ok(wallet)
    }

    /// Get all wallets
    pub fn list_wallets(&self, key_id: i64) -> Result<Vec<Wallet>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, encrypted_passphrase, created_at FROM wallets WHERE key_id = ?1 ORDER BY created_at DESC"
        ).context("Failed to prepare LIST_WALLETS statement")?;

        let wallets = stmt
            .query_map(params![key_id], |row| {
                Ok(Wallet {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    encrypted_passphrase: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .context("Failed to query wallets")?
            .collect::<Result<Vec<Wallet>, _>>()
            .context("Failed to collect wallets")?;

        Ok(wallets)
    }

    /// Create an account for a wallet with encrypted private key and public key
    pub fn create_account(
        &self,
        wallet_id: i64,
        account_index: i32,
        encrypted_private_key: &str,
        public_key: &str,
        chain_type: &ChainType,
    ) -> Result<Account> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO accounts (wallet_id, account_index, encrypted_private_key, public_key, chain_type) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![wallet_id, account_index, encrypted_private_key, public_key, chain_type.to_string()],
        )
        .context("Failed to insert account")?;

        let id = conn.last_insert_rowid();

        Ok(Account {
            id,
            wallet_id,
            account_index,
            encrypted_private_key: encrypted_private_key.to_string(),
            public_key: public_key.to_string(),
            chain_type: chain_type.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Get account by wallet_id, account_index, and chain_type
    pub fn get_account(
        &self,
        wallet_id: i64,
        account_index: i32,
        chain_type: &ChainType,
    ) -> Result<Option<Account>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT id, wallet_id, account_index, encrypted_private_key, public_key, chain_type, created_at 
                 FROM accounts WHERE wallet_id = ?1 AND account_index = ?2 AND chain_type = ?3",
            )
            .context("Failed to prepare GET_ACCOUNT statement")?;

        let account = stmt
            .query_row(
                params![wallet_id, account_index, chain_type.to_string()],
                |row| {
                    let chain_type: String = row.get(5)?;
                    let chain_type = ChainType::from_str(&chain_type);
                    Ok(Account {
                        id: row.get(0)?,
                        wallet_id: row.get(1)?,
                        account_index: row.get(2)?,
                        encrypted_private_key: row.get(3)?,
                        public_key: row.get(4)?,
                        chain_type: chain_type.unwrap(),
                        created_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .context("Failed to query account")?;
        Ok(account)
    }

    /// Create a new API key for a wallet
    pub fn create_api_key(&self, key_name: &str, key_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO api_keys (key_hash, key_name) VALUES (?1, ?2)",
            params![key_hash, key_name],
        )
        .context("Failed to create API key")?;
        Ok(())
    }

    /// Rotate an API key by updating its hash
    pub fn rotate_api_key(&self, key_id: i64, new_key_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE api_keys SET key_hash = ?1, is_revoked = 0 WHERE id = ?2",
            params![new_key_hash, key_id],
        )
        .context("Failed to rotate API key")?;
        Ok(())
    }

    /// List all API keys
    pub fn list_api_keys(&self) -> Result<Vec<(i64, String, bool)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, key_name, is_revoked FROM api_keys")
            .context("Failed to prepare LIST_API_KEYS statement")?;
        let api_keys = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .context("Failed to query API keys")?
            .collect::<Result<Vec<(i64, String, bool)>, _>>()
            .context("Failed to collect API keys")?;
        Ok(api_keys)
    }

    /// Validate an API key and return wallet_id and client_name if valid
    pub fn validate_api_key(&self, key_hash: &str) -> Result<Option<(i64, String)>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT id, key_name FROM api_keys
                 WHERE key_hash = ?1 AND is_revoked = 0",
            )
            .context("Failed to prepare VALIDATE_API_KEY statement")?;

        let result = stmt
            .query_row(params![key_hash], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()
            .context("Failed to query API key")?;

        Ok(result)
    }

    /// Revoke an API key
    pub fn revoke_api_key(&self, key_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE api_keys SET is_revoked = 1 WHERE id = ?1",
            params![key_id],
        )
        .context("Failed to revoke API key")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ChainType::Bitcoin;
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

        let db = Database::new(&db_path, "test_key").expect("Failed to create database");

        db.create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = db
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        let wallet = db
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

        let db = Database::new(&db_path, "test_key").expect("Failed to create database");

        db.create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = db
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        db.create_wallet(key_id, "TestWallet", "encrypted_passphrase")
            .expect("Failed to create wallet");

        let retrieved = db
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

        let db = Database::new(&db_path, "test_key").expect("Failed to create database");

        db.create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = db
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        db.create_wallet(key_id, "Wallet1", "pass1")
            .expect("Failed to create wallet 1");
        db.create_wallet(key_id, "Wallet2", "pass2")
            .expect("Failed to create wallet 2");

        let wallets = db.list_wallets(key_id).expect("Failed to list wallets");
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

        let db = Database::new(&db_path, "test_key").expect("Failed to create database");

        db.create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = db
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        let wallet = db
            .create_wallet(key_id, "WalletWithAccounts", "pass")
            .expect("Failed to create wallet");

        let account = db
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

        let db = Database::new(&db_path, "test_key").expect("Failed to create database");

        db.create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = db
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        let wallet = db
            .create_wallet(key_id, "TestWallet", "pass")
            .expect("Failed to create wallet");

        db.create_account(wallet.id, 0, "key1", "pubkey1", &Bitcoin)
            .expect("Failed to create account");

        let retrieved = db
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

        let db = Database::new(&db_path, "test_key").expect("Failed to create database");

        db.create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = db
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        let wallet = db
            .create_wallet(key_id, "TestWallet", "pass")
            .expect("Failed to create wallet");

        let retrieved = db
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

        let db = Database::new(&db_path, "test_key").expect("Failed to create database");

        db.create_api_key("TestKey", "hash123")
            .expect("Failed to create API key");
        let (key_id, _) = db
            .validate_api_key("hash123")
            .expect("Failed to validate API key")
            .unwrap();

        let retrieved = db
            .get_wallet_by_id(key_id, 999)
            .expect("Failed to query wallet");

        assert!(retrieved.is_none());
    }
}
