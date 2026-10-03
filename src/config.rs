use std::env;

/// Hosts accepted on `/mcp` when `MCP_ALLOWED_HOSTS` is not set (loopback only).
pub const DEFAULT_MCP_ALLOWED_HOSTS: &[&str] = &["localhost", "127.0.0.1", "::1"];

pub struct Config {
    pub admin_key: String,
    pub secret_key: String,
    pub db_key: String,
    pub db_path: String,
    /// `Host` header values accepted on `/mcp` (DNS-rebinding protection).
    pub mcp_allowed_hosts: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let admin_key = env::var("ADMIN_API_KEY").expect("ADMIN_API_KEY must be set");
        let secret_key = env::var("SECRET_KEY").expect("SECRET_KEY must be set");
        let db_key = env::var("DATABASE_KEY").expect("DATABASE_KEY must be set");
        let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "data/arktos.db".to_string());
        let mcp_allowed_hosts = parse_allowed_hosts(env::var("MCP_ALLOWED_HOSTS").ok());

        Self {
            admin_key,
            secret_key,
            db_key,
            db_path,
            mcp_allowed_hosts,
        }
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
