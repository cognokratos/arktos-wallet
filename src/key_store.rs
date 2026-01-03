use crate::database::Database;
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::{Arc, Mutex};

pub struct KeyStore {
    conn: Arc<Mutex<Connection>>,
}

impl KeyStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            conn: db.conn.clone(),
        }
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
