use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    pub chain_type: String,
    pub created_at: String,
}
