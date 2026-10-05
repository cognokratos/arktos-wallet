//! Wallet use cases and their public request/response types.
//!
//! The request and response structs below are the single definition of the
//! MCP tool contract: their JSON Schemas (input and output) are generated from
//! these types by `rmcp`, and responses are serialized as structured content.
//! They contain public data only.

use crate::api_key::ApiKey;
use crate::crypto;
use crate::database::{Database, StoreError};
use crate::domain::{BitcoinNetwork, Chain, DerivationIndex, EthereumChainId, Network, WalletName};
use crate::error::AppError;
use crate::keys::WalletKeys;
use crate::wallet::Account;
use crate::wallet_manager::{self, RecoveryPhrase};
use crate::wallet_store::{NewAccount, WalletStore};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct CreateWalletRequest {
    /// Wallet name, unique per API key: 1-255 characters, no leading or
    /// trailing whitespace, no control characters.
    pub wallet_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct GetBitcoinAddressRequest {
    /// Name of a wallet owned by the caller.
    pub wallet_name: String,
    /// Address index: the last, non-hardened component of the BIP86 path
    /// m/86'/coin'/0'/0/{account_index}. Defaults to 0.
    #[serde(default)]
    #[schemars(range(min = 0, max = 2147483647))]
    pub account_index: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct GetEthereumAddressRequest {
    /// Name of a wallet owned by the caller.
    pub wallet_name: String,
    /// Address index: the last, non-hardened component of the path
    /// m/44'/60'/0'/0/{account_index}. Defaults to 0.
    #[serde(default)]
    #[schemars(range(min = 0, max = 2147483647))]
    pub account_index: Option<u32>,
}

// ---------------------------------------------------------------------------
// Responses (public data only)
// ---------------------------------------------------------------------------

// PUBLIC-ONLY: these types are every successful result a model can receive.
// They have no secret-bearing fields, and `deny_unknown_fields` makes clients
// reject any field beyond the contract.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateWalletResponse {
    /// Server-assigned wallet identifier.
    pub wallet_id: i64,
    pub wallet_name: String,
    /// Creation time, RFC 3339 UTC.
    pub created_at: String,
}

/// Bitcoin address type returned by Arktos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BitcoinAddressType {
    /// Pay-to-Taproot (BIP86 key-path-only, bech32m).
    P2tr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BitcoinAddressResponse {
    pub wallet_name: String,
    /// Address index used in the derivation path.
    pub account_index: u32,
    /// Always `bitcoin`.
    pub chain: Chain,
    /// Bitcoin network the address is encoded for (server configuration).
    pub network: BitcoinNetwork,
    pub address_type: BitcoinAddressType,
    /// BIP86 path, e.g. m/86'/0'/0'/0/0 (coin type 1' on test networks).
    pub derivation_path: String,
    /// Taproot address (bc1p…, tb1p… or bcrt1p…).
    pub address: String,
    /// Compressed SEC1 secp256k1 public key, 0x-hex: the BIP86 internal key
    /// (before the Taproot tweak).
    pub public_key_hex: String,
    /// When the account was first derived, RFC 3339 UTC.
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EthereumAddressResponse {
    pub wallet_name: String,
    /// Address index used in the derivation path.
    pub account_index: u32,
    /// Always `ethereum`.
    pub chain: Chain,
    /// Configured EIP-155 chain ID. The address is the same on every chain ID.
    pub chain_id: u64,
    /// BIP44 path m/44'/60'/0'/0/{account_index}.
    pub derivation_path: String,
    /// EIP-55 checksummed address.
    pub address: String,
    /// Compressed SEC1 secp256k1 public key, 0x-hex.
    pub public_key_hex: String,
    /// When the account was first derived, RFC 3339 UTC.
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Chain settings from configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChainConfig {
    pub bitcoin_network: BitcoinNetwork,
    pub ethereum_chain_id: EthereumChainId,
}

/// Log an unexpected persistence failure and convert it to a domain error.
fn storage_failure(context: &'static str, error: StoreError) -> AppError {
    warn!(error = %error, "{context}");
    AppError::Storage(error)
}

fn wallet_name(value: &str) -> Result<WalletName, AppError> {
    WalletName::parse(value).map_err(|e| AppError::invalid("wallet_name", e))
}

fn derivation_index(value: Option<u32>) -> Result<DerivationIndex, AppError> {
    DerivationIndex::new(value.unwrap_or(0)).map_err(|e| AppError::invalid("account_index", e))
}

pub struct WalletServices {
    store: WalletStore,
    keys: WalletKeys,
    chains: ChainConfig,
}

impl WalletServices {
    pub fn new(db: Arc<Database>, keys: WalletKeys, chains: ChainConfig) -> Self {
        Self {
            store: WalletStore::new(db),
            keys,
            chains,
        }
    }

    pub fn chain_config(&self) -> ChainConfig {
        self.chains
    }

    /// Create a wallet with a new 12-word BIP39 recovery phrase, encrypted
    /// before it is stored. The phrase is never returned.
    pub async fn create_wallet(
        &self,
        api_key: &ApiKey,
        req: CreateWalletRequest,
    ) -> Result<CreateWalletResponse, AppError> {
        let name = wallet_name(&req.wallet_name)?;
        info!(wallet_name = %name, api_key_id = api_key.id, "Creating wallet");

        // Friendly early exit; the UNIQUE (key_id, name) constraint is the
        // authoritative, race-safe check.
        if self
            .store
            .get_wallet(api_key.id, name.as_str())
            .await
            .map_err(|e| storage_failure("Database error checking wallet", e))?
            .is_some()
        {
            return Err(AppError::WalletAlreadyExists {
                wallet_name: name.into_string(),
            });
        }

        // SECRET-BOUNDARY: the only place a new recovery phrase exists in
        // plaintext. It is sealed and zeroized when `phrase` goes out of scope.
        let encrypted_passphrase = {
            let phrase = wallet_manager::generate_recovery_passphrase()
                .map_err(|e| AppError::Internal(format!("mnemonic generation: {e}")))?;
            crypto::seal(self.keys.seed.aead(), phrase.expose().as_bytes())
                .map_err(|e| AppError::Crypto(e.to_string()))?
        };

        let wallet = self
            .store
            .create_wallet(api_key.id, name.as_str(), &encrypted_passphrase)
            .await
            .map_err(|e| match e {
                StoreError::AlreadyExists => AppError::WalletAlreadyExists {
                    wallet_name: name.to_string(),
                },
                e => storage_failure("Database error creating wallet", e),
            })?;

        info!(wallet_id = wallet.id, wallet_name = %wallet.name, "Wallet created");
        Ok(CreateWalletResponse {
            wallet_id: wallet.id,
            wallet_name: wallet.name,
            created_at: wallet.created_at,
        })
    }

    /// Get (deriving on first use) the Taproot address at `account_index` on
    /// the configured Bitcoin network.
    pub async fn get_bitcoin_address(
        &self,
        api_key: &ApiKey,
        req: GetBitcoinAddressRequest,
    ) -> Result<BitcoinAddressResponse, AppError> {
        let name = wallet_name(&req.wallet_name)?;
        let index = derivation_index(req.account_index)?;
        let network = self.chains.bitcoin_network;
        let account = self
            .account(api_key, &name, Network::Bitcoin(network), index)
            .await?;
        Ok(BitcoinAddressResponse {
            wallet_name: name.into_string(),
            account_index: account.account_index,
            chain: Chain::Bitcoin,
            network,
            address_type: BitcoinAddressType::P2tr,
            derivation_path: account.derivation_path,
            address: account.address,
            public_key_hex: account.public_key,
            created_at: account.created_at,
        })
    }

    /// Get (deriving on first use) the Ethereum address at `account_index`.
    pub async fn get_ethereum_address(
        &self,
        api_key: &ApiKey,
        req: GetEthereumAddressRequest,
    ) -> Result<EthereumAddressResponse, AppError> {
        let name = wallet_name(&req.wallet_name)?;
        let index = derivation_index(req.account_index)?;
        let account = self
            .account(api_key, &name, Network::Ethereum, index)
            .await?;
        let address = wallet_manager::eip55_checksum(&account.address).map_err(|e| {
            AppError::Storage(StoreError::CorruptData(format!(
                "account {}: {e}",
                account.id
            )))
        })?;
        Ok(EthereumAddressResponse {
            wallet_name: name.into_string(),
            account_index: account.account_index,
            chain: Chain::Ethereum,
            chain_id: self.chains.ethereum_chain_id.get(),
            derivation_path: account.derivation_path,
            address,
            public_key_hex: account.public_key,
            created_at: account.created_at,
        })
    }

    /// Return the stored account, or derive and store it on first use.
    async fn account(
        &self,
        api_key: &ApiKey,
        name: &WalletName,
        network: Network,
        index: DerivationIndex,
    ) -> Result<Account, AppError> {
        // Scoped to the caller's API key: other owners' wallets are invisible.
        let wallet = self
            .store
            .get_wallet(api_key.id, name.as_str())
            .await
            .map_err(|e| storage_failure("Database error retrieving wallet", e))?
            .ok_or_else(|| AppError::WalletNotFound {
                wallet_name: name.to_string(),
            })?;

        if let Some(account) = self
            .store
            .get_account(wallet.id, network, index)
            .await
            .map_err(|e| storage_failure("Database error retrieving account", e))?
        {
            return Ok(account);
        }

        info!(
            wallet_id = wallet.id,
            chain = %network.chain(),
            network = network.as_storage_str(),
            account_index = index.get(),
            "Deriving new account"
        );

        // SECRET-BOUNDARY: the phrase, seed and extended private keys exist
        // only inside this block and are zeroized when it ends; only public
        // data leaves it. Private keys are never extracted or stored.
        let derived = {
            let phrase = crypto::open(self.keys.seed.aead(), &wallet.encrypted_passphrase)
                .map_err(|e| AppError::Crypto(format!("wallet {}: {e}", wallet.id)))
                .and_then(|bytes| {
                    RecoveryPhrase::from_utf8(bytes)
                        .map_err(|e| AppError::Crypto(format!("wallet {}: {e}", wallet.id)))
                })?;
            wallet_manager::derive_account_keys(&phrase, network, index)
                .map_err(|e| AppError::DerivationFailed(format!("wallet {}: {e}", wallet.id)))?
        };

        // Returns the existing row if a concurrent request stored it first.
        let account = self
            .store
            .insert_account(NewAccount {
                wallet_id: wallet.id,
                network,
                account_index: index,
                derivation_path: derived.derivation_path,
                public_key: derived.public_key,
                address: derived.address,
            })
            .await
            .map_err(|e| storage_failure("Database error storing account", e))?;
        info!(wallet_id = wallet.id, address = %account.address, "Account stored");
        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_services::KeyServices;
    use crate::keys::{Keyring, MasterKey};
    use tempfile::TempDir;

    struct Fixture {
        services: WalletServices,
        owner: ApiKey,
        _dir: TempDir,
    }

    async fn fixture(chains: ChainConfig) -> Fixture {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("services.db");
        let db = Arc::new(Database::new(path.to_str().unwrap(), "db-key").unwrap());
        let keyring = || Keyring::new(&MasterKey::from_bytes([0x42; 32]));
        let keys = KeyServices::new(db.clone(), keyring().api_keys);
        let raw = keys.create("owner").await.unwrap();
        let owner = keys.validate(&raw).await.unwrap();
        Fixture {
            services: WalletServices::new(db, keyring().wallet, chains),
            owner,
            _dir: dir,
        }
    }

    impl Fixture {
        async fn create(&self, name: &str) -> Result<CreateWalletResponse, AppError> {
            self.services
                .create_wallet(
                    &self.owner,
                    CreateWalletRequest {
                        wallet_name: name.into(),
                    },
                )
                .await
        }

        async fn btc(
            &self,
            name: &str,
            index: Option<u32>,
        ) -> Result<BitcoinAddressResponse, AppError> {
            self.services
                .get_bitcoin_address(
                    &self.owner,
                    GetBitcoinAddressRequest {
                        wallet_name: name.into(),
                        account_index: index,
                    },
                )
                .await
        }

        async fn eth(
            &self,
            name: &str,
            index: Option<u32>,
        ) -> Result<EthereumAddressResponse, AppError> {
            self.services
                .get_ethereum_address(
                    &self.owner,
                    GetEthereumAddressRequest {
                        wallet_name: name.into(),
                        account_index: index,
                    },
                )
                .await
        }
    }

    #[tokio::test]
    async fn create_wallet_returns_typed_response() {
        let fx = fixture(ChainConfig::default()).await;
        let created = fx.create("main").await.unwrap();
        assert_eq!(created.wallet_name, "main");
        assert!(created.wallet_id > 0);
        assert!(created.created_at.ends_with('Z'));
    }

    #[tokio::test]
    async fn create_wallet_validates_and_rejects_duplicates() {
        let fx = fixture(ChainConfig::default()).await;
        assert!(matches!(
            fx.create(" padded").await.unwrap_err(),
            AppError::InvalidArgument {
                field: "wallet_name",
                ..
            }
        ));
        fx.create("main").await.unwrap();
        assert_eq!(
            fx.create("main").await.unwrap_err(),
            AppError::WalletAlreadyExists {
                wallet_name: "main".into()
            }
        );
    }

    #[tokio::test]
    async fn bitcoin_address_defaults_to_index_zero_and_is_stable() {
        let fx = fixture(ChainConfig::default()).await;
        fx.create("main").await.unwrap();
        let first = fx.btc("main", None).await.unwrap();
        assert_eq!(first.account_index, 0);
        assert_eq!(first.network, BitcoinNetwork::Mainnet);
        assert_eq!(first.derivation_path, "m/86'/0'/0'/0/0");
        assert!(first.address.starts_with("bc1p"));
        assert_eq!(fx.btc("main", Some(0)).await.unwrap(), first);
        assert_ne!(
            fx.btc("main", Some(1)).await.unwrap().address,
            first.address
        );
    }

    #[tokio::test]
    async fn bitcoin_network_comes_from_configuration() {
        let fx = fixture(ChainConfig {
            bitcoin_network: BitcoinNetwork::Regtest,
            ..ChainConfig::default()
        })
        .await;
        fx.create("main").await.unwrap();
        let resp = fx.btc("main", None).await.unwrap();
        assert_eq!(resp.network, BitcoinNetwork::Regtest);
        assert_eq!(resp.derivation_path, "m/86'/1'/0'/0/0");
        assert!(resp.address.starts_with("bcrt1p"));
    }

    #[tokio::test]
    async fn ethereum_address_is_checksummed_with_chain_id() {
        let fx = fixture(ChainConfig {
            ethereum_chain_id: EthereumChainId::new(11155111).unwrap(),
            ..ChainConfig::default()
        })
        .await;
        fx.create("main").await.unwrap();
        let resp = fx.eth("main", None).await.unwrap();
        assert_eq!(resp.chain_id, 11155111);
        assert_eq!(resp.derivation_path, "m/44'/60'/0'/0/0");
        assert_eq!(
            wallet_manager::eip55_checksum(&resp.address).unwrap(),
            resp.address
        );
    }

    #[tokio::test]
    async fn unknown_wallet_and_bad_index_are_client_errors() {
        let fx = fixture(ChainConfig::default()).await;
        assert_eq!(
            fx.btc("missing", None).await.unwrap_err(),
            AppError::WalletNotFound {
                wallet_name: "missing".into()
            }
        );
        fx.create("main").await.unwrap();
        assert!(matches!(
            fx.eth("main", Some(1 << 31)).await.unwrap_err(),
            AppError::InvalidArgument {
                field: "account_index",
                ..
            }
        ));
    }
}
