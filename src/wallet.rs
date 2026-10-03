use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::str::FromStr;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Wallet {
    pub id: i64,
    pub name: String,
    /// Encrypted recovery phrase (v1 envelope, see `crypto`).
    pub encrypted_passphrase: String,
    pub created_at: String,
}

impl fmt::Debug for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wallet")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("encrypted_passphrase", &"[REDACTED]")
            .field("created_at", &self.created_at)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct Account {
    pub id: i64,
    pub wallet_id: i64,
    pub account_index: u32,
    /// Canonical BIP32 path (see `wallet_manager::derivation_path`).
    pub derivation_path: String,
    pub address: String,
    pub public_key: String,
    pub chain_type: ChainType,
    pub created_at: String,
}

impl fmt::Debug for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Account")
            .field("id", &self.id)
            .field("wallet_id", &self.wallet_id)
            .field("account_index", &self.account_index)
            .field("derivation_path", &self.derivation_path)
            .field("address", &self.address)
            .field("public_key", &self.public_key)
            .field("chain_type", &self.chain_type)
            .field("created_at", &self.created_at)
            .finish()
    }
}

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
