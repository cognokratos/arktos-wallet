//! Chain, network and validated input types shared by the API, services and
//! persistence.
//!
//! Derivation (BIP39 mnemonic → BIP32 HD keys):
//!
//! | Network | Path | Standard |
//! |---------|------|----------|
//! | Bitcoin mainnet | `m/86'/0'/0'/0/{index}` | BIP86 (Taproot, P2TR) |
//! | Bitcoin testnet / signet / regtest | `m/86'/1'/0'/0/{index}` | BIP86, coin type 1' for test networks |
//! | Ethereum (any chain ID) | `m/44'/60'/0'/0/{index}` | BIP44 path, coin type 60' |
//!
//! `{index}` is the non-hardened *address index* (the last path component);
//! the public API calls it `account_index` for compatibility.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Supported blockchains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Bitcoin,
    Ethereum,
}

impl Chain {
    /// Value stored in `accounts.chain_type`.
    pub fn as_storage_str(self) -> &'static str {
        match self {
            Chain::Bitcoin => "Bitcoin",
            Chain::Ethereum => "Ethereum",
        }
    }

    pub fn from_storage_str(value: &str) -> Option<Self> {
        match value {
            "Bitcoin" => Some(Chain::Bitcoin),
            "Ethereum" => Some(Chain::Ethereum),
            _ => None,
        }
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Chain::Bitcoin => "bitcoin",
            Chain::Ethereum => "ethereum",
        })
    }
}

/// Bitcoin network used for address encoding (`BITCOIN_NETWORK`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BitcoinNetwork {
    #[default]
    Mainnet,
    Testnet,
    Signet,
    Regtest,
}

impl BitcoinNetwork {
    pub const ALL: [BitcoinNetwork; 4] = [
        BitcoinNetwork::Mainnet,
        BitcoinNetwork::Testnet,
        BitcoinNetwork::Signet,
        BitcoinNetwork::Regtest,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            BitcoinNetwork::Mainnet => "mainnet",
            BitcoinNetwork::Testnet => "testnet",
            BitcoinNetwork::Signet => "signet",
            BitcoinNetwork::Regtest => "regtest",
        }
    }

    /// BIP44/BIP86 coin type: 0' on mainnet, 1' on every test network.
    pub fn coin_type(self) -> u32 {
        match self {
            BitcoinNetwork::Mainnet => 0,
            _ => 1,
        }
    }

    pub fn to_bitcoin(self) -> bitcoin::Network {
        match self {
            BitcoinNetwork::Mainnet => bitcoin::Network::Bitcoin,
            BitcoinNetwork::Testnet => bitcoin::Network::Testnet,
            BitcoinNetwork::Signet => bitcoin::Network::Signet,
            BitcoinNetwork::Regtest => bitcoin::Network::Regtest,
        }
    }
}

impl fmt::Display for BitcoinNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BitcoinNetwork {
    type Err = ();

    /// Accepts exactly `mainnet`, `testnet`, `signet` or `regtest`.
    fn from_str(value: &str) -> Result<Self, ()> {
        BitcoinNetwork::ALL
            .into_iter()
            .find(|n| n.as_str() == value)
            .ok_or(())
    }
}

/// EIP-155 chain ID (`ETHEREUM_CHAIN_ID`). Address derivation does not depend
/// on it; it is reported so future transactions have an explicit target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EthereumChainId(u64);

impl EthereumChainId {
    pub const MAINNET: EthereumChainId = EthereumChainId(1);

    /// Any positive chain ID.
    pub fn new(id: u64) -> Option<Self> {
        (id > 0).then_some(Self(id))
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl Default for EthereumChainId {
    fn default() -> Self {
        Self::MAINNET
    }
}

/// The address space an account belongs to; determines the derivation path
/// and address encoding, and is part of an account's persisted identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Network {
    Bitcoin(BitcoinNetwork),
    /// Ethereum addresses are identical on every EVM chain ID.
    Ethereum,
}

impl Network {
    pub fn chain(self) -> Chain {
        match self {
            Network::Bitcoin(_) => Chain::Bitcoin,
            Network::Ethereum => Chain::Ethereum,
        }
    }

    /// Value stored in `accounts.network`.
    pub fn as_storage_str(self) -> &'static str {
        match self {
            Network::Bitcoin(network) => network.as_str(),
            Network::Ethereum => "evm",
        }
    }

    pub fn from_storage(chain: Chain, network: &str) -> Option<Self> {
        match chain {
            Chain::Bitcoin => network.parse().ok().map(Network::Bitcoin),
            Chain::Ethereum => (network == "evm").then_some(Network::Ethereum),
        }
    }

    /// The single source of truth for derivation paths. The `accounts` table
    /// enforces the same format with a CHECK constraint.
    pub fn derivation_path(self, index: DerivationIndex) -> String {
        match self {
            Network::Bitcoin(network) => {
                format!("m/86'/{}'/0'/0/{}", network.coin_type(), index.get())
            }
            Network::Ethereum => format!("m/44'/60'/0'/0/{}", index.get()),
        }
    }
}

/// Why an input value was rejected. Messages are safe to show to clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    Empty,
    TooLong { max_chars: usize },
    SurroundingWhitespace,
    ForbiddenCharacter,
    OutOfRange { max: u32 },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::Empty => write!(f, "must not be empty"),
            ValidationError::TooLong { max_chars } => {
                write!(f, "must be at most {max_chars} characters")
            }
            ValidationError::SurroundingWhitespace => {
                write!(f, "must not start or end with whitespace")
            }
            ValidationError::ForbiddenCharacter => write!(
                f,
                "must not contain control, bidirectional-override or zero-width characters"
            ),
            ValidationError::OutOfRange { max } => write!(f, "must be between 0 and {max}"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Non-hardened BIP32 child index (`0 ..= 2^31 - 1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DerivationIndex(u32);

impl DerivationIndex {
    pub const MAX: u32 = (1 << 31) - 1;

    pub fn new(index: u32) -> Result<Self, ValidationError> {
        if index <= Self::MAX {
            Ok(Self(index))
        } else {
            Err(ValidationError::OutOfRange { max: Self::MAX })
        }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

/// Maximum length of wallet and API-key names, in Unicode scalar values.
pub const MAX_NAME_CHARS: usize = 255;

/// Shared name policy: 1–255 characters of any script, no surrounding
/// whitespace (rejected, not trimmed), no control characters and no invisible
/// bidi/zero-width characters that enable look-alike names. Names are compared
/// exactly as given.
fn validate_name(value: &str) -> Result<(), ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::Empty);
    }
    if value.chars().count() > MAX_NAME_CHARS {
        return Err(ValidationError::TooLong {
            max_chars: MAX_NAME_CHARS,
        });
    }
    if value.trim() != value {
        return Err(ValidationError::SurroundingWhitespace);
    }
    if value.chars().any(is_forbidden_char) {
        return Err(ValidationError::ForbiddenCharacter);
    }
    Ok(())
}

fn is_forbidden_char(c: char) -> bool {
    c.is_control()
        || matches!(c,
            '\u{200B}'..='\u{200F}'   // zero-width space/joiners, LRM/RLM
            | '\u{2028}' | '\u{2029}' // line / paragraph separator
            | '\u{202A}'..='\u{202E}' // bidi embeddings and overrides
            | '\u{2060}'..='\u{2064}' // word joiner, invisible operators
            | '\u{2066}'..='\u{2069}' // bidi isolates
            | '\u{FEFF}') // zero-width no-break space / BOM
}

macro_rules! validated_name {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: &str) -> Result<Self, ValidationError> {
                validate_name(value).map(|()| Self(value.to_owned()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

validated_name!(
    /// A wallet name, unique per owning API key.
    WalletName
);
validated_name!(
    /// An admin-assigned label for an API key.
    ApiKeyName
);

#[cfg(test)]
mod tests {
    use super::*;

    fn index(i: u32) -> DerivationIndex {
        DerivationIndex::new(i).unwrap()
    }

    #[test]
    fn derivation_paths_follow_bip86_and_bip44() {
        assert_eq!(
            Network::Bitcoin(BitcoinNetwork::Mainnet).derivation_path(index(0)),
            "m/86'/0'/0'/0/0"
        );
        for test_network in [
            BitcoinNetwork::Testnet,
            BitcoinNetwork::Signet,
            BitcoinNetwork::Regtest,
        ] {
            assert_eq!(
                Network::Bitcoin(test_network).derivation_path(index(7)),
                "m/86'/1'/0'/0/7"
            );
        }
        assert_eq!(
            Network::Ethereum.derivation_path(index(DerivationIndex::MAX)),
            "m/44'/60'/0'/0/2147483647"
        );
    }

    #[test]
    fn bitcoin_network_parsing_is_strict() {
        for network in BitcoinNetwork::ALL {
            assert_eq!(network.as_str().parse(), Ok(network));
        }
        for bad in ["Mainnet", "bitcoin", "test", " mainnet", "", "testnet4"] {
            assert!(bad.parse::<BitcoinNetwork>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn storage_labels_round_trip() {
        for network in BitcoinNetwork::ALL
            .map(Network::Bitcoin)
            .into_iter()
            .chain([Network::Ethereum])
        {
            let chain = Chain::from_storage_str(network.chain().as_storage_str()).unwrap();
            assert_eq!(
                Network::from_storage(chain, network.as_storage_str()),
                Some(network)
            );
        }
        assert_eq!(Network::from_storage(Chain::Ethereum, "mainnet"), None);
        assert_eq!(Network::from_storage(Chain::Bitcoin, "evm"), None);
    }

    #[test]
    fn chain_id_must_be_positive() {
        assert_eq!(EthereumChainId::new(0), None);
        assert_eq!(EthereumChainId::new(11155111).unwrap().get(), 11155111);
        assert_eq!(EthereumChainId::default().get(), 1);
    }

    #[test]
    fn derivation_index_range() {
        assert!(DerivationIndex::new(0).is_ok());
        assert!(DerivationIndex::new(DerivationIndex::MAX).is_ok());
        assert_eq!(
            DerivationIndex::new(DerivationIndex::MAX + 1),
            Err(ValidationError::OutOfRange {
                max: DerivationIndex::MAX
            })
        );
    }

    #[test]
    fn names_accept_any_script() {
        for ok in [
            "main",
            "Épargne 2026",
            "財布",
            "кошелёк",
            "a b",
            "x".repeat(255).as_str(),
        ] {
            assert!(WalletName::parse(ok).is_ok(), "{ok:?}");
        }
    }

    #[test]
    fn names_reject_invalid_values() {
        let cases = [
            ("", ValidationError::Empty),
            (" main", ValidationError::SurroundingWhitespace),
            ("main\t", ValidationError::SurroundingWhitespace),
            ("ma\nin", ValidationError::ForbiddenCharacter),
            ("ma\u{0}in", ValidationError::ForbiddenCharacter),
            ("ma\u{202E}in", ValidationError::ForbiddenCharacter),
            ("ma\u{200B}in", ValidationError::ForbiddenCharacter),
        ];
        for (input, expected) in cases {
            assert_eq!(WalletName::parse(input), Err(expected.clone()), "{input:?}");
            assert_eq!(ApiKeyName::parse(input), Err(expected), "{input:?}");
        }
        assert_eq!(
            WalletName::parse(&"x".repeat(256)),
            Err(ValidationError::TooLong { max_chars: 255 })
        );
    }
}
