use crate::api_key::ApiKey;
use crate::database::Database;
use crate::key_store::KeyStore;
use crate::keys::ApiKeyKeys;
use std::sync::Arc;

pub struct KeyServices {
    store: KeyStore,
    keys: ApiKeyKeys,
}

impl KeyServices {
    pub fn new(db: Arc<Database>, keys: ApiKeyKeys) -> Self {
        Self {
            store: KeyStore::new(db),
            keys,
        }
    }

    /// Create a new API key entry in the database
    pub async fn create(&self, name: &str) -> anyhow::Result<String> {
        let api_key = ApiKey::generate()?;
        self.store
            .create_api_key(name, &self.keys.hmac.hash(&api_key))?;
        Ok(api_key)
    }

    /// Rotate an existing API key by its ID
    pub async fn rotate(&self, key_id: i64) -> anyhow::Result<String> {
        let new_api_key = ApiKey::generate()?;
        self.store
            .rotate_api_key(key_id, &self.keys.hmac.hash(&new_api_key))?;
        Ok(new_api_key)
    }

    /// List all API keys (for admin purposes)
    pub async fn list(&self) -> anyhow::Result<Vec<ApiKey>> {
        let api_keys = self.store.list_api_keys()?;
        let api_keys = api_keys.into_iter().map(ApiKey::read).collect();
        Ok(api_keys)
    }

    /// Validate an API key and return its details if valid.
    ///
    /// Keys are looked up by HMAC, so the database comparison only ever sees
    /// keyed hashes.
    pub async fn validate(&self, api_key: &str) -> anyhow::Result<ApiKey> {
        match self.store.validate_api_key(&self.keys.hmac.hash(api_key))? {
            None => anyhow::bail!("Invalid API key"),
            Some((id, name)) => Ok(ApiKey::new(id, name)),
        }
    }

    /// Revoke an API key by its hashed value
    pub async fn revoke(&self, key_id: i64) -> anyhow::Result<()> {
        self.store.revoke_api_key(key_id)?;
        Ok(())
    }
}
