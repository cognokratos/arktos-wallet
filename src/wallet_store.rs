//! Wallet and account persistence. All queries are scoped by the owning API
//! key (`key_id`), so one owner can never read another owner's wallets.

use crate::database::{Database, StoreError};
use crate::wallet::{Account, ChainType, Wallet};
use rusqlite::{OptionalExtension, Row, params};
use std::str::FromStr;
use std::sync::Arc;

/// Public data of a newly derived account.
pub struct NewAccount {
    pub wallet_id: i64,
    pub chain_type: ChainType,
    pub account_index: u32,
    pub derivation_path: String,
    pub public_key: String,
    pub address: String,
}

pub struct WalletStore {
    db: Arc<Database>,
}

const WALLET_COLUMNS: &str = "id, name, encrypted_passphrase, created_at";
const ACCOUNT_COLUMNS: &str =
    "id, wallet_id, account_index, derivation_path, address, public_key, chain_type, created_at";

fn wallet_from_row(row: &Row<'_>) -> rusqlite::Result<Wallet> {
    Ok(Wallet {
        id: row.get(0)?,
        name: row.get(1)?,
        encrypted_passphrase: row.get(2)?,
        created_at: row.get(3)?,
    })
}

/// Map an account row; an unknown chain type is reported as corrupt data.
fn account_from_row(row: &Row<'_>) -> rusqlite::Result<Result<Account, StoreError>> {
    let chain: String = row.get(6)?;
    let Ok(chain_type) = ChainType::from_str(&chain) else {
        return Ok(Err(StoreError::CorruptData(format!(
            "unknown chain type in accounts row {}",
            row.get::<_, i64>(0)?
        ))));
    };
    Ok(Ok(Account {
        id: row.get(0)?,
        wallet_id: row.get(1)?,
        account_index: row.get(2)?,
        derivation_path: row.get(3)?,
        address: row.get(4)?,
        public_key: row.get(5)?,
        chain_type,
        created_at: row.get(7)?,
    }))
}

impl WalletStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Insert a wallet. Fails with [`StoreError::AlreadyExists`] if the owner
    /// already has a wallet with this name, and [`StoreError::ForeignKeyViolation`]
    /// if `key_id` does not exist.
    pub async fn create_wallet(
        &self,
        key_id: i64,
        name: &str,
        encrypted_passphrase: &str,
    ) -> Result<Wallet, StoreError> {
        let (name, encrypted_passphrase) = (name.to_owned(), encrypted_passphrase.to_owned());
        self.db
            .write(move |tx| {
                Ok(tx.query_row(
                    &format!(
                        "INSERT INTO wallets (key_id, name, encrypted_passphrase) VALUES (?1, ?2, ?3)
                         RETURNING {WALLET_COLUMNS}"
                    ),
                    params![key_id, name, encrypted_passphrase],
                    wallet_from_row,
                )?)
            })
            .await
    }

    /// The owner's wallet with this name.
    pub async fn get_wallet(&self, key_id: i64, name: &str) -> Result<Option<Wallet>, StoreError> {
        let name = name.to_owned();
        self.db
            .read(move |conn| {
                Ok(conn
                    .query_row(
                        &format!(
                            "SELECT {WALLET_COLUMNS} FROM wallets WHERE key_id = ?1 AND name = ?2"
                        ),
                        params![key_id, name],
                        wallet_from_row,
                    )
                    .optional()?)
            })
            .await
    }

    /// The owner's wallet with this id.
    pub async fn get_wallet_by_id(
        &self,
        key_id: i64,
        wallet_id: i64,
    ) -> Result<Option<Wallet>, StoreError> {
        self.db
            .read(move |conn| {
                Ok(conn
                    .query_row(
                        &format!(
                            "SELECT {WALLET_COLUMNS} FROM wallets WHERE key_id = ?1 AND id = ?2"
                        ),
                        params![key_id, wallet_id],
                        wallet_from_row,
                    )
                    .optional()?)
            })
            .await
    }

    /// All of the owner's wallets, newest first.
    pub async fn list_wallets(&self, key_id: i64) -> Result<Vec<Wallet>, StoreError> {
        self.db
            .read(move |conn| {
                let mut stmt = conn.prepare(&format!(
                    "SELECT {WALLET_COLUMNS} FROM wallets WHERE key_id = ?1 ORDER BY created_at DESC, id DESC"
                ))?;
                let wallets = stmt
                    .query_map(params![key_id], wallet_from_row)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(wallets)
            })
            .await
    }

    /// The account for wallet + chain + index, if it was derived before.
    pub async fn get_account(
        &self,
        wallet_id: i64,
        account_index: u32,
        chain_type: &ChainType,
    ) -> Result<Option<Account>, StoreError> {
        let chain = chain_type.to_string();
        self.db
            .read(move |conn| {
                conn.query_row(
                    &format!(
                        "SELECT {ACCOUNT_COLUMNS} FROM accounts
                         WHERE wallet_id = ?1 AND account_index = ?2 AND chain_type = ?3"
                    ),
                    params![wallet_id, account_index, chain],
                    account_from_row,
                )
                .optional()?
                .transpose()
            })
            .await
    }

    /// Insert an account, or return the existing one if a concurrent request
    /// inserted it first. Derivation is deterministic, so both rows are equal;
    /// the UNIQUE constraint makes this race-safe.
    pub async fn insert_account(&self, account: NewAccount) -> Result<Account, StoreError> {
        self.db
            .write(move |tx| {
                let chain = account.chain_type.to_string();
                tx.execute(
                    "INSERT INTO accounts
                         (wallet_id, chain_type, account_index, derivation_path, public_key, address)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT (wallet_id, chain_type, account_index) DO NOTHING",
                    params![
                        account.wallet_id,
                        chain,
                        account.account_index,
                        account.derivation_path,
                        account.public_key,
                        account.address
                    ],
                )?;
                let stored = tx
                    .query_row(
                        &format!(
                            "SELECT {ACCOUNT_COLUMNS} FROM accounts
                             WHERE wallet_id = ?1 AND account_index = ?2 AND chain_type = ?3"
                        ),
                        params![account.wallet_id, account.account_index, chain],
                        account_from_row,
                    )??;
                if stored.address != account.address || stored.public_key != account.public_key {
                    return Err(StoreError::CorruptData(format!(
                        "stored account {} does not match its derivation",
                        stored.id
                    )));
                }
                Ok(stored)
            })
            .await
    }
}
