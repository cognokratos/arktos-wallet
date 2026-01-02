use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
pub enum ChainType {
    Bitcoin,
    Ethereum,
}

impl Display for ChainType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainType::Bitcoin => write!(f, "Bitcoin"),
            ChainType::Ethereum => write!(f, "Ethereum"),
        }
    }
}

impl FromStr for ChainType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Bitcoin" => Ok(ChainType::Bitcoin),
            "Ethereum" => Ok(ChainType::Ethereum),
            _ => Err(format!("Unknown chain type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Wallet {
    pub id: i64,
    pub name: String,
    pub encrypted_passphrase: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct Account {
    pub id: i64,
    pub wallet_id: i64,
    pub account_index: i32,
    pub encrypted_private_key: String,
    pub public_key: String,
    pub chain_type: ChainType,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApiKey {
    pub id: i64,
    pub wallet_id: i64,
    pub key: String,
    pub key_hash: String,
    pub client_name: String,
    pub created_at: String,
    pub is_revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApiKeyResponse {
    pub client_name: String,
    pub created_at: String,
    pub is_revoked: bool,
}
