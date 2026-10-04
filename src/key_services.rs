use crate::api_key::ApiKey;
use crate::database::{Database, StoreError};
use crate::domain::ApiKeyName;
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
        let name = ApiKeyName::parse(name).map_err(|e| anyhow::anyhow!("API key name {e}"))?;
        let name = name.as_str();
        let api_key = ApiKey::generate()?;
        self.store
            .create_api_key(name, &self.keys.hmac.hash(&api_key))
            .await?;
        Ok(api_key)
    }

    /// Rotate an existing API key by its ID; `None` if the ID is unknown.
    pub async fn rotate(&self, key_id: i64) -> anyhow::Result<Option<String>> {
        let new_api_key = ApiKey::generate()?;
        let rotated = self
            .store
            .rotate_api_key(key_id, &self.keys.hmac.hash(&new_api_key))
            .await?;
        Ok(rotated.then_some(new_api_key))
    }

    /// List all API keys (for admin purposes)
    pub async fn list(&self) -> Result<Vec<ApiKey>, StoreError> {
        let api_keys = self.store.list_api_keys().await?;
        Ok(api_keys.into_iter().map(ApiKey::read).collect())
    }

    /// Look up an API key: `Ok(None)` if it is unknown or revoked, `Err` if
    /// the database could not be queried.
    ///
    /// Keys are looked up by HMAC, so the database comparison only ever sees
    /// keyed hashes.
    pub async fn lookup(&self, api_key: &str) -> Result<Option<ApiKey>, StoreError> {
        Ok(self
            .store
            .validate_api_key(&self.keys.hmac.hash(api_key))
            .await?
            .map(|(id, name)| ApiKey::new(id, name)))
    }

    /// Validate an API key and return its details if valid.
    pub async fn validate(&self, api_key: &str) -> anyhow::Result<ApiKey> {
        match self.lookup(api_key).await? {
            None => anyhow::bail!("Invalid API key"),
            Some(key) => Ok(key),
        }
    }

    /// Revoke an API key by its ID; `false` if the ID is unknown.
    pub async fn revoke(&self, key_id: i64) -> Result<bool, StoreError> {
        self.store.revoke_api_key(key_id).await
    }
}
