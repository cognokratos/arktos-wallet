use crate::keys::MasterKey;
use secrecy::{ExposeSecret, SecretString};
use std::env;
use std::fmt;

/// Hosts accepted on `/mcp` when `MCP_ALLOWED_HOSTS` is not set (loopback only).
pub const DEFAULT_MCP_ALLOWED_HOSTS: &[&str] = &["localhost", "127.0.0.1", "::1"];

pub const MASTER_KEY_VAR: &str = "MASTER_KEY";

/// Settings needed to open the database (used by `migrate` / `db-info`).
pub struct DatabaseConfig {
    pub db_path: String,
    /// SQLCipher key; independent of the application key hierarchy.
    pub db_key: SecretString,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| env::var(name).ok())
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let get = |name: &str| lookup(name).filter(|v| !v.trim().is_empty());
        Ok(Self {
            db_key: get("DATABASE_KEY")
                .map(SecretString::from)
                .ok_or_else(|| ConfigError("DATABASE_KEY must be set".into()))?,
            db_path: get("DATABASE_PATH").unwrap_or_else(|| DEFAULT_DATABASE_PATH.to_string()),
        })
    }
}

/// Relative to the working directory; the container image sets an absolute path.
pub const DEFAULT_DATABASE_PATH: &str = "data/arktos.db";

pub struct Config {
    pub admin_key: SecretString,
    /// SQLCipher key; independent of the application key hierarchy.
    pub db_key: SecretString,
    pub db_path: String,
    /// Root of the application key hierarchy (see `keys`).
    pub master_key: MasterKey,
    /// `Host` header values accepted on `/mcp` (DNS-rebinding protection).
    pub mcp_allowed_hosts: Vec<String>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("admin_key", &"[REDACTED]")
            .field("db_key", &"[REDACTED]")
            .field("db_path", &self.db_path)
            .field("master_key", &self.master_key)
            .field("mcp_allowed_hosts", &self.mcp_allowed_hosts)
            .finish()
    }
}

/// Configuration error. Messages name the variable, never its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| env::var(name).ok())
    }

    /// Build the configuration from a variable lookup (testable without
    /// touching the process environment). Empty values count as unset.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let get = |name: &str| lookup(name).filter(|v| !v.trim().is_empty());
        let required = |name: &str| {
            get(name)
                .map(SecretString::from)
                .ok_or_else(|| ConfigError(format!("{name} must be set")))
        };

        let admin_key = required("ADMIN_API_KEY")?;
        let db_key = required("DATABASE_KEY")?;

        let master_value = get(MASTER_KEY_VAR).map(SecretString::from).ok_or_else(|| {
            ConfigError(format!(
                "{MASTER_KEY_VAR} is not set: expected a 32-byte base64 value \
                 (generate one with `make secret`)"
            ))
        })?;
        let master_key = MasterKey::from_base64(master_value.expose_secret())
            .map_err(|e| ConfigError(format!("{MASTER_KEY_VAR} is invalid: {e}")))?;

        if db_key.expose_secret() == master_value.expose_secret() {
            return Err(ConfigError(format!(
                "DATABASE_KEY must be generated independently of {MASTER_KEY_VAR}"
            )));
        }

        Ok(Self {
            admin_key,
            db_key,
            db_path: get("DATABASE_PATH").unwrap_or_else(|| DEFAULT_DATABASE_PATH.to_string()),
            master_key,
            mcp_allowed_hosts: parse_allowed_hosts(get("MCP_ALLOWED_HOSTS")),
        })
    }
}

/// Parse a comma-separated host list, falling back to loopback hosts.
fn parse_allowed_hosts(value: Option<String>) -> Vec<String> {
    let hosts: Vec<String> = value
        .iter()
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .map(String::from)
        .collect();
    if hosts.is_empty() {
        DEFAULT_MCP_ALLOWED_HOSTS
            .iter()
            .map(|h| h.to_string())
            .collect()
    } else {
        hosts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const MASTER: &str = "q83vEjRWeJCrze8SNFZ4kKvN7xI0VniQq83vEjRWeJA=";

    fn config(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|name| vars.get(name).cloned())
    }

    fn base() -> Vec<(&'static str, &'static str)> {
        vec![
            ("ADMIN_API_KEY", "admin-secret-value"),
            ("DATABASE_KEY", "database-secret-value"),
            (MASTER_KEY_VAR, MASTER),
        ]
    }

    #[test]
    fn valid_configuration_loads() {
        let config = config(&base()).expect("valid");
        assert_eq!(config.db_path, "data/arktos.db");
    }

    #[test]
    fn missing_master_key_is_explained() {
        let mut vars = base();
        vars.retain(|(k, _)| *k != MASTER_KEY_VAR);
        let err = config(&vars).unwrap_err().to_string();
        assert!(err.contains("MASTER_KEY is not set"), "{err}");
    }

    #[test]
    fn malformed_master_key_is_rejected_without_echoing_it() {
        for bad in ["short-password", "dG9vIHNob3J0", "%%%not-base64%%%"] {
            let mut vars = base();
            vars.retain(|(k, _)| *k != MASTER_KEY_VAR);
            vars.push((MASTER_KEY_VAR, bad));
            let err = config(&vars).unwrap_err().to_string();
            assert!(err.starts_with("MASTER_KEY is invalid"), "{err}");
            assert!(err.contains("32-byte base64 value"), "{err}");
            assert!(!err.contains(bad), "{err}");
        }
    }

    #[test]
    fn empty_values_count_as_missing() {
        let mut vars = base();
        vars.retain(|(k, _)| *k != "ADMIN_API_KEY");
        vars.push(("ADMIN_API_KEY", "  "));
        assert_eq!(
            config(&vars).unwrap_err().to_string(),
            "ADMIN_API_KEY must be set"
        );
    }

    #[test]
    fn database_key_must_differ_from_master_key() {
        let mut vars = base();
        vars.retain(|(k, _)| *k != "DATABASE_KEY");
        vars.push(("DATABASE_KEY", MASTER));
        let err = config(&vars).unwrap_err().to_string();
        assert!(err.contains("independently"), "{err}");
    }

    #[test]
    fn debug_redacts_secrets() {
        let rendered = format!("{:?}", config(&base()).unwrap());
        for secret in ["admin-secret-value", "database-secret-value", MASTER] {
            assert!(!rendered.contains(secret), "{rendered}");
        }
    }

    #[test]
    fn allowed_hosts_default_to_loopback() {
        assert_eq!(parse_allowed_hosts(None), DEFAULT_MCP_ALLOWED_HOSTS);
        assert_eq!(
            parse_allowed_hosts(Some(" , ".into())),
            DEFAULT_MCP_ALLOWED_HOSTS
        );
    }

    #[test]
    fn allowed_hosts_are_parsed_from_comma_separated_list() {
        assert_eq!(
            parse_allowed_hosts(Some("wallet.example.com, localhost:8080".into())),
            vec!["wallet.example.com", "localhost:8080"]
        );
    }
}
