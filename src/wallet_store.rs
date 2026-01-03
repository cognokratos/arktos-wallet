use crate::database::Database;
use crate::wallet::{Account, ChainType, Wallet};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

pub struct WalletStore {
    conn: Arc<Mutex<Connection>>,
}

impl WalletStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            conn: db.conn.clone(),
        }
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
}
