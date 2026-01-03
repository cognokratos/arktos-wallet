use std::env;

pub struct Config {
    pub admin_key: String,
    pub secret_key: String,
    pub db_key: String,
    pub db_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        let admin_key = env::var("ADMIN_API_KEY").expect("ADMIN_API_KEY must be set");
        let secret_key = env::var("SECRET_KEY").expect("SECRET_KEY must be set");
        let db_key = env::var("DATABASE_KEY").expect("DATABASE_KEY must be set");
        let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "data/arktos.db".to_string());

        Self {
            admin_key,
            secret_key,
            db_key,
            db_path,
        }
    }
}
