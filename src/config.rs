use std::env;

pub struct Config {
    pub cipher_key: String,
    pub db_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        let cipher_key =
            env::var("DATABASE_CIPHER_KEY").unwrap_or_else(|_| "default_cipher_key".to_string());
        let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "data/arktos.db".to_string());

        Self {
            cipher_key,
            db_path,
        }
    }
}
