//! API-key persistence. Only HMAC hashes of API keys are stored.

use crate::database::{Database, StoreError};
use rusqlite::{OptionalExtension, params};
use std::sync::Arc;

pub struct KeyStore {
    db: Arc<Database>,
}

impl KeyStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Store a new API-key hash and return its id.
    pub async fn create_api_key(&self, key_name: &str, key_hash: &str) -> Result<i64, StoreError> {
        let (key_name, key_hash) = (key_name.to_owned(), key_hash.to_owned());
        self.db
            .write(move |tx| {
                Ok(tx.query_row(
                    "INSERT INTO api_keys (key_hash, key_name) VALUES (?1, ?2) RETURNING id",
                    params![key_hash, key_name],
                    |row| row.get(0),
                )?)
            })
            .await
    }

    /// Replace a key's hash and un-revoke it. Returns `false` if the id is unknown.
    pub async fn rotate_api_key(
        &self,
        key_id: i64,
        new_key_hash: &str,
    ) -> Result<bool, StoreError> {
        let new_key_hash = new_key_hash.to_owned();
        self.db
            .write(move |tx| {
                Ok(tx.execute(
                    "UPDATE api_keys SET key_hash = ?1, is_revoked = 0 WHERE id = ?2",
                    params![new_key_hash, key_id],
                )? == 1)
            })
            .await
    }

    /// All API keys as `(id, name, is_revoked)`.
    pub async fn list_api_keys(&self) -> Result<Vec<(i64, String, bool)>, StoreError> {
        self.db
            .read(|conn| {
                let mut stmt =
                    conn.prepare("SELECT id, key_name, is_revoked FROM api_keys ORDER BY id")?;
                let keys = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(keys)
            })
            .await
    }

    /// `(id, name)` of the non-revoked key with this hash.
    pub async fn validate_api_key(
        &self,
        key_hash: &str,
    ) -> Result<Option<(i64, String)>, StoreError> {
        let key_hash = key_hash.to_owned();
        self.db
            .read(move |conn| {
                Ok(conn
                    .query_row(
                        "SELECT id, key_name FROM api_keys WHERE key_hash = ?1 AND is_revoked = 0",
                        params![key_hash],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()?)
            })
            .await
    }

    /// Revoke a key. Returns `false` if the id is unknown.
    pub async fn revoke_api_key(&self, key_id: i64) -> Result<bool, StoreError> {
        self.db
            .write(move |tx| {
                Ok(tx.execute(
                    "UPDATE api_keys SET is_revoked = 1 WHERE id = ?1",
                    params![key_id],
                )? == 1)
            })
            .await
    }
}
