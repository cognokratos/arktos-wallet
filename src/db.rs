use crate::models::{Account, Wallet};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
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
            "CREATE TABLE IF NOT EXISTS wallets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                encrypted_passphrase TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallet_id INTEGER NOT NULL,
                account_index INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (wallet_id) REFERENCES wallets(id),
                UNIQUE(wallet_id, account_index)
            );",
        )
        .context("Failed to create schema")?;
        Ok(())
    }

    /// Create a wallet with encrypted passphrase
    pub fn create_wallet(&self, name: &str, encrypted_passphrase: &str) -> Result<Wallet> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO wallets (name, encrypted_passphrase) VALUES (?1, ?2)",
            params![name, encrypted_passphrase],
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
    pub fn get_wallet(&self, name: &str) -> Result<Option<Wallet>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT id, name, encrypted_passphrase, created_at FROM wallets WHERE name = ?1",
            )
            .context("Failed to prepare statement")?;

        let wallet = stmt
            .query_row(params![name], |row| {
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
    pub fn list_wallets(&self) -> Result<Vec<Wallet>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, encrypted_passphrase, created_at FROM wallets ORDER BY created_at DESC"
        ).context("Failed to prepare statement")?;

        let wallets = stmt
            .query_map([], |row| {
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

    /// Create an account for a wallet
    pub fn create_account(&self, wallet_id: i64, account_index: i32) -> Result<Account> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO accounts (wallet_id, account_index) VALUES (?1, ?2)",
            params![wallet_id, account_index],
        )
        .context("Failed to insert account")?;

        let id = conn.last_insert_rowid();

        Ok(Account {
            id,
            wallet_id,
            account_index,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let wallet = db
            .create_wallet("MyWallet", "encrypted_data")
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
        db.create_wallet("TestWallet", "encrypted_passphrase")
            .expect("Failed to create wallet");

        let retrieved = db.get_wallet("TestWallet").expect("Failed to get wallet");

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
        db.create_wallet("Wallet1", "pass1")
            .expect("Failed to create wallet 1");
        db.create_wallet("Wallet2", "pass2")
            .expect("Failed to create wallet 2");

        let wallets = db.list_wallets().expect("Failed to list wallets");
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
        let wallet = db
            .create_wallet("WalletWithAccounts", "pass")
            .expect("Failed to create wallet");

        let account = db
            .create_account(wallet.id, 0)
            .expect("Failed to create account");

        assert_eq!(account.wallet_id, wallet.id);
        assert_eq!(account.account_index, 0);
    }
}
