//! BIP39 mnemonic generation and BIP32 account derivation.
//!
//! The mnemonic is held in [`RecoveryPhrase`], which zeroizes on drop and
//! redacts `Debug`. Seeds are zeroized as soon as the extended key is built,
//! and derived private keys never leave this module: only the public key and
//! address are returned.
//! Library-internal copies (e.g. inside `bip39::Mnemonic` or `bip32::XPrv`
//! chain codes) are outside our control.

use crate::domain::{BitcoinNetwork, DerivationIndex, Network};
use anyhow::{Result, anyhow};
use bip32::{DerivationPath, XPrv};
use bip39::Mnemonic;
use bitcoin::{
    PublicKey,
    address::Address,
    secp256k1::{Secp256k1, XOnlyPublicKey},
};
use std::fmt::{self, Write as _};
use std::str::FromStr;
use tiny_keccak::{Hasher, Keccak};
use zeroize::{Zeroize, Zeroizing};

/// Longest 12-word English mnemonic: 12 × 8 letters + 11 spaces.
const MAX_PHRASE_LEN: usize = 12 * 8 + 11;

/// A BIP39 recovery phrase. Zeroized on drop, never printed.
pub struct RecoveryPhrase(Zeroizing<String>);

impl RecoveryPhrase {
    /// Take ownership of a decrypted phrase without copying it.
    pub fn from_utf8(bytes: Zeroizing<Vec<u8>>) -> Result<Self> {
        let mut bytes = bytes;
        match String::from_utf8(std::mem::take(&mut *bytes)) {
            Ok(phrase) => Ok(Self(Zeroizing::new(phrase))),
            Err(err) => {
                err.into_bytes().zeroize();
                Err(anyhow!("recovery phrase is not valid UTF-8"))
            }
        }
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RecoveryPhrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecoveryPhrase([REDACTED])")
    }
}

/// Generate a 12-word BIP39 mnemonic from 128 bits of OS randomness.
pub fn generate_recovery_passphrase() -> Result<RecoveryPhrase> {
    let mut entropy = Zeroizing::new([0u8; 16]);
    getrandom::fill(entropy.as_mut_slice())
        .map_err(|_| anyhow!("OS random number generator failed"))?;
    let mnemonic = Mnemonic::from_entropy(entropy.as_slice())
        .map_err(|_| anyhow!("failed to build mnemonic from entropy"))?;
    // Pre-sized so formatting never reallocates and leaves a stray copy.
    let mut phrase = Zeroizing::new(String::with_capacity(MAX_PHRASE_LEN));
    write!(phrase, "{mnemonic}").map_err(|_| anyhow!("failed to format mnemonic"))?;
    Ok(RecoveryPhrase(phrase))
}

/// Public result of deriving one account. Contains no secret material.
#[derive(Debug)]
pub struct AccountData {
    pub derivation_path: String,
    /// Compressed SEC1 public key, `0x`-prefixed hex.
    pub public_key: String,
    pub address: String,
}

/// Derive an account's public key and address.
///
/// BIP39 seed → BIP32 key at [`Network::derivation_path`]:
/// Bitcoin uses the BIP86 Taproot path and encodes a P2TR address for the
/// given network; Ethereum uses the BIP44 path `m/44'/60'/0'/0/{index}`.
/// Ethereum addresses are returned in canonical lowercase hex; apply
/// [`eip55_checksum`] for display.
pub fn derive_account_keys(
    phrase: &RecoveryPhrase,
    network: Network,
    index: DerivationIndex,
) -> Result<AccountData> {
    let seed = {
        let mnemonic =
            Mnemonic::parse_normalized(phrase.expose()).map_err(|_| anyhow!("Invalid mnemonic"))?;
        Zeroizing::new(mnemonic.to_seed(""))
    };
    let mut xprv = XPrv::new(seed.as_slice())?;
    drop(seed);

    let derivation_path = network.derivation_path(index);
    let path = DerivationPath::from_str(&derivation_path)
        .map_err(|e| anyhow!("Failed to parse derivation path: {}", e))?;
    for child_num in path.into_iter() {
        xprv = xprv
            .derive_child(child_num)
            .map_err(|e| anyhow!("Failed to derive child key: {}", e))?;
    }

    let public_key = xprv.public_key().to_bytes().to_vec();
    // The extended private key (and its secp256k1 scalar) is zeroized on drop.
    drop(xprv);

    let address = match network {
        Network::Bitcoin(bitcoin_network) => derive_bitcoin_address(&public_key, bitcoin_network)?,
        Network::Ethereum => derive_ethereum_address(&public_key)?,
    };

    Ok(AccountData {
        derivation_path,
        public_key: format!("0x{}", hex::encode(public_key)),
        address,
    })
}

/// BIP86 key-path-only Taproot (P2TR) address: the derived key is the internal
/// key, tweaked with no script tree (BIP341).
fn derive_bitcoin_address(public_key: &[u8], network: BitcoinNetwork) -> Result<String> {
    let pk = PublicKey::from_slice(public_key)
        .map_err(|e| anyhow!("Invalid public key format: {}", e))?;
    let xonly = XOnlyPublicKey::from(pk.inner);
    let secp = Secp256k1::verification_only();
    Ok(Address::p2tr(&secp, xonly, None, network.to_bitcoin()).to_string())
}

/// Ethereum address: last 20 bytes of Keccak-256 over the uncompressed public
/// key without its 0x04 prefix, as lowercase hex.
fn derive_ethereum_address(public_key: &[u8]) -> Result<String> {
    let public_key = PublicKey::from_slice(public_key)
        .map_err(|e| anyhow!("Invalid public key format: {}", e))?;
    let uncompressed = public_key.inner.serialize_uncompressed();
    let hash = keccak256(&uncompressed[1..]);
    Ok(format!("0x{}", hex::encode(&hash[12..])))
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(data);
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    hash
}

/// EIP-55 mixed-case checksum encoding of a `0x`-prefixed 20-byte hex
/// address (input in any case).
pub fn eip55_checksum(address: &str) -> Result<String> {
    let hex_part = address
        .strip_prefix("0x")
        .filter(|h| h.len() == 40 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| anyhow!("not a 0x-prefixed 20-byte hex address"))?
        .to_ascii_lowercase();
    let hash = keccak256(hex_part.as_bytes());
    let checksummed: String = hex_part
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let nibble = (hash[i / 2] >> if i % 2 == 0 { 4 } else { 0 }) & 0x0f;
            if c.is_ascii_alphabetic() && nibble >= 8 {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect();
    Ok(format!("0x{checksummed}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Public BIP39 test mnemonic (never use for real funds).
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn phrase(words: &str) -> RecoveryPhrase {
        RecoveryPhrase::from_utf8(Zeroizing::new(words.as_bytes().to_vec())).unwrap()
    }

    fn index(i: u32) -> DerivationIndex {
        DerivationIndex::new(i).unwrap()
    }

    fn derive(network: Network, i: u32) -> AccountData {
        derive_account_keys(&phrase(TEST_MNEMONIC), network, index(i)).unwrap()
    }

    const MAINNET: Network = Network::Bitcoin(BitcoinNetwork::Mainnet);

    // ---- Known derivation vectors (must never change) ----------------------
    // Mainnet values are the BIP86 reference vectors; test-network values were
    // generated independently with Python `bip_utils` (Bip86 coins).

    #[test]
    fn bitcoin_mainnet_bip86_vectors() {
        let a0 = derive(MAINNET, 0);
        assert_eq!(a0.derivation_path, "m/86'/0'/0'/0/0");
        assert_eq!(
            a0.address,
            "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
        );
        assert_eq!(
            a0.public_key,
            "0x03cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115"
        );
        assert_eq!(
            derive(MAINNET, 1).address,
            "bc1p4qhjn9zdvkux4e44uhx8tc55attvtyu358kutcqkudyccelu0was9fqzwh"
        );
    }

    #[test]
    fn bitcoin_test_network_bip86_vectors() {
        for network in [BitcoinNetwork::Testnet, BitcoinNetwork::Signet] {
            let a0 = derive(Network::Bitcoin(network), 0);
            assert_eq!(a0.derivation_path, "m/86'/1'/0'/0/0", "{network}");
            assert_eq!(
                a0.address, "tb1p8wpt9v4frpf3tkn0srd97pksgsxc5hs52lafxwru9kgeephvs7rqlqt9zj",
                "{network}"
            );
            assert_eq!(
                a0.public_key,
                "0x0255355ca83c973f1d97ce0e3843c85d78905af16b4dc531bc488e57212d230116"
            );
        }
        let regtest = derive(Network::Bitcoin(BitcoinNetwork::Regtest), 1);
        assert_eq!(regtest.derivation_path, "m/86'/1'/0'/0/1");
        assert_eq!(
            regtest.address,
            "bcrt1p90h6z3p36n9hrzy7580h5l429uwchyg8uc9sz4jwzhdtuhqdl5eqkcyx0f"
        );
    }

    #[test]
    fn bitcoin_addresses_parse_for_their_network_only() {
        use bitcoin::address::NetworkUnchecked;
        for network in BitcoinNetwork::ALL {
            let address = derive(Network::Bitcoin(network), 0).address;
            let parsed: Address<NetworkUnchecked> = address.parse().unwrap();
            assert!(
                parsed.is_valid_for_network(network.to_bitcoin()),
                "{network}"
            );
            if network != BitcoinNetwork::Mainnet {
                assert!(!parsed.is_valid_for_network(bitcoin::Network::Bitcoin));
            }
        }
    }

    #[test]
    fn ethereum_bip44_vectors() {
        let a0 = derive(Network::Ethereum, 0);
        assert_eq!(a0.derivation_path, "m/44'/60'/0'/0/0");
        assert_eq!(a0.address, "0x9858effd232b4033e47d90003d41ec34ecaeda94");
        assert_eq!(
            eip55_checksum(&a0.address).unwrap(),
            "0x9858EfFD232B4033E47d90003D41EC34EcaEda94"
        );
        assert_eq!(
            a0.public_key,
            "0x0237b0bb7a8288d38ed49a524b5dc98cff3eb5ca824c9f9dc0dfdb3d9cd600f299"
        );
        assert_eq!(
            eip55_checksum(&derive(Network::Ethereum, 1).address).unwrap(),
            "0x6Fac4D18c912343BF86fa7049364Dd4E424Ab9C0"
        );
    }

    #[test]
    fn eip55_reference_vectors() {
        // Test vectors from the EIP-55 specification.
        for expected in [
            "0x52908400098527886E0F7030069857D2E4169EE7",
            "0x8617E340B3D01FA5F11F306F4090FD50E238070D",
            "0xde709f2102306220921060314715629080e2fb77",
            "0x27b1fdb04752bbc536007a920d24acb045561c26",
            "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
            "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359",
            "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB",
            "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb",
        ] {
            let hex = &expected[2..];
            let lower = format!("0x{}", hex.to_ascii_lowercase());
            let upper = format!("0x{}", hex.to_ascii_uppercase());
            assert_eq!(eip55_checksum(&lower).unwrap(), expected);
            assert_eq!(eip55_checksum(&upper).unwrap(), expected);
        }
    }

    #[test]
    fn eip55_rejects_malformed_addresses() {
        for bad in [
            "",
            "0x",
            "9858effd232b4033e47d90003d41ec34ecaeda94",
            "0x9858effd",
            "0xzz58effd232b4033e47d90003d41ec34ecaeda94",
        ] {
            assert!(eip55_checksum(bad).is_err(), "{bad:?}");
        }
    }

    // ---- Mnemonic generation ------------------------------------------------

    #[test]
    fn generated_phrase_is_valid_12_word_bip39() {
        let generated = generate_recovery_passphrase().expect("generate");
        assert_eq!(generated.expose().split_whitespace().count(), 12);
        Mnemonic::parse_normalized(generated.expose()).expect("valid BIP39");
        assert!(generated.0.capacity() >= generated.expose().len());
    }

    #[test]
    fn generated_phrases_are_unique() {
        let a = generate_recovery_passphrase().unwrap();
        let b = generate_recovery_passphrase().unwrap();
        assert_ne!(a.expose(), b.expose());
    }

    // ---- Derivation properties ---------------------------------------------

    #[test]
    fn different_indices_give_different_keys() {
        let (a, b) = (derive(MAINNET, 0), derive(MAINNET, 1));
        assert_ne!(a.public_key, b.public_key);
        assert_ne!(a.address, b.address);
    }

    #[test]
    fn derivation_is_deterministic() {
        for network in [MAINNET, Network::Ethereum] {
            let (a, b) = (derive(network, 3), derive(network, 3));
            assert_eq!(a.address, b.address);
            assert_eq!(a.public_key, b.public_key);
        }
    }

    #[test]
    fn invalid_mnemonic_is_rejected_without_echoing_it() {
        let err =
            derive_account_keys(&phrase("invalid mnemonic words"), MAINNET, index(0)).unwrap_err();
        assert!(!err.to_string().contains("invalid mnemonic words"));
    }

    #[test]
    fn invalid_utf8_phrase_is_rejected() {
        assert!(RecoveryPhrase::from_utf8(Zeroizing::new(vec![0xff, 0xfe])).is_err());
    }

    // ---- Secret hygiene -----------------------------------------------------

    #[test]
    fn debug_output_redacts_secrets() {
        let account = derive(Network::Ethereum, 0);
        let rendered = format!("{account:?} {:?}", phrase(TEST_MNEMONIC));
        // Known private key of m/44'/60'/0'/0/0 must not be reachable anywhere.
        assert!(
            !rendered.contains("1ab42cc412b618bdea3a599e3c9bae199ebf030895b039e9db1e30dafb12b727")
        );
        assert!(!rendered.contains("abandon"));
        assert!(rendered.contains("REDACTED"));
        assert!(rendered.contains(&account.address));
    }
}
