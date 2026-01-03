use crate::api_key::ApiKey;
use crate::db::Database;
use std::sync::Arc;

pub struct KeyStore {
    db: Arc<Database>,
}

impl KeyStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Create a new API key entry in the database
    pub async fn create(&self, name: &str) -> anyhow::Result<String> {
        let api_key = ApiKey::generate();
        let hashed_key = ApiKey::hash(&api_key);
        self.db.create_api_key(name, &hashed_key)?;
        Ok(api_key)
    }

    /// Rotate an existing API key by its ID
    pub async fn rotate(&self, key_id: i64) -> anyhow::Result<String> {
        let new_api_key = ApiKey::generate();
        let hashed_key = ApiKey::hash(&new_api_key);
        self.db.rotate_api_key(key_id, &hashed_key)?;
        Ok(new_api_key)
    }

    /// List all API keys (for admin purposes)
    pub async fn list(&self) -> anyhow::Result<Vec<ApiKey>> {
        let api_keys = self.db.list_api_keys()?;
        let api_keys = api_keys.into_iter().map(ApiKey::read).collect();
        Ok(api_keys)
    }

    /// Validate an API key and return its details if valid
    pub async fn validate(&self, api_key: &str) -> anyhow::Result<ApiKey> {
        let hashed_key = ApiKey::hash(api_key);
        match self.db.validate_api_key(&hashed_key)? {
            None => anyhow::bail!("Invalid API key"),
            Some((id, name)) => Ok(ApiKey::new(id, name)),
        }
    }

    /// Revoke an API key by its hashed value
    pub async fn revoke(&self, key_id: i64) -> anyhow::Result<()> {
        self.db.revoke_api_key(key_id)?;
        Ok(())
    }
}
