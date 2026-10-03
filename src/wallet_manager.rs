//! BIP39 mnemonic generation and BIP32 account derivation.
//!
//! The mnemonic is held in [`RecoveryPhrase`], which zeroizes on drop and
//! redacts `Debug`. Seeds are zeroized as soon as the extended key is built,
//! and derived private keys never leave this module: only the public key and
//! address are returned.
//! Library-internal copies (e.g. inside `bip39::Mnemonic` or `bip32::XPrv`
//! chain codes) are outside our control.

use crate::wallet::ChainType;
use anyhow::{Result, anyhow};
use bip32::{DerivationPath, XPrv};
use bip39::Mnemonic;
use bitcoin::{
    Network, PublicKey,
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

/// Highest non-hardened BIP32 child index; larger indices are rejected.
pub const MAX_ACCOUNT_INDEX: u32 = (1 << 31) - 1;

/// Canonical derivation path for a chain and account index. The `accounts`
/// table enforces the same format with a CHECK constraint.
pub fn derivation_path(chain_type: &ChainType, account_index: u32) -> String {
    match chain_type {
        ChainType::Bitcoin => format!("m/86'/0'/0'/0/{account_index}"),
        ChainType::Ethereum => format!("m/44'/60'/0'/0/{account_index}"),
    }
}

/// Derive an account's public key and address for a given chain.
///
/// Paths: Bitcoin `m/86'/0'/0'/0/{index}` (Taproot, BIP86),
/// Ethereum `m/44'/60'/0'/0/{index}`.
pub fn derive_account_keys(
    phrase: &RecoveryPhrase,
    account_index: u32,
    chain_type: &ChainType,
) -> Result<AccountData> {
    let seed = {
        let mnemonic =
            Mnemonic::parse_normalized(phrase.expose()).map_err(|_| anyhow!("Invalid mnemonic"))?;
        Zeroizing::new(mnemonic.to_seed(""))
    };
    let mut xprv = XPrv::new(seed.as_slice())?;
    drop(seed);

    if account_index > MAX_ACCOUNT_INDEX {
        return Err(anyhow!("account index must be at most {MAX_ACCOUNT_INDEX}"));
    }
    let derivation_path = derivation_path(chain_type, account_index);

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

    let address = match chain_type {
        ChainType::Bitcoin => derive_bitcoin_address(&public_key)?,
        ChainType::Ethereum => derive_ethereum_address(&public_key)?,
    };

    Ok(AccountData {
        derivation_path,
        public_key: format!("0x{}", hex::encode(public_key)),
        address,
    })
}

/// Derive a Bitcoin address (Taproot) from a (compressed) secp256k1 public key.
fn derive_bitcoin_address(public_key: &[u8]) -> Result<String> {
    let pk = PublicKey::from_slice(public_key)
        .map_err(|e| anyhow!("Invalid public key format: {}", e))?;

    let xonly = XOnlyPublicKey::from(pk.inner);
    let secp = Secp256k1::verification_only();
    let address = Address::p2tr(&secp, xonly, None, Network::Bitcoin);

    Ok(address.to_string())
}

/// Derive an Ethereum address from a (compressed) secp256k1 public key.
///
/// Ethereum uses Keccak-256 of the uncompressed public key (without the 0x04 prefix),
/// then takes the last 20 bytes.
fn derive_ethereum_address(public_key: &[u8]) -> Result<String> {
    let public_key = PublicKey::from_slice(public_key)
        .map_err(|e| anyhow!("Invalid public key format: {}", e))?;

    // Uncompressed public key bytes include the 0x04 prefix
    let uncompressed = public_key.inner.serialize_uncompressed();

    let key_material = if uncompressed.len() == 65 && uncompressed[0] == 0x04 {
        &uncompressed[1..]
    } else {
        &uncompressed[..]
    };

    let mut hasher = Keccak::v256();
    hasher.update(key_material);
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    let address_bytes = &hash[12..]; // last 20 bytes
    Ok(format!("0x{}", hex::encode(address_bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::ChainType::{Bitcoin, Ethereum};

    /// Public BIP39 test mnemonic (never use for real funds).
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn phrase(words: &str) -> RecoveryPhrase {
        RecoveryPhrase::from_utf8(Zeroizing::new(words.as_bytes().to_vec())).unwrap()
    }

    // ---- Known derivation vectors (must never change) ----------------------

    #[test]
    fn bitcoin_bip86_test_vectors() {
        // BIP86 reference vectors for the test mnemonic.
        let a0 = derive_account_keys(&phrase(TEST_MNEMONIC), 0, &Bitcoin).unwrap();
        assert_eq!(a0.derivation_path, "m/86'/0'/0'/0/0");
        assert_eq!(
            a0.address,
            "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
        );
        assert_eq!(
            a0.public_key,
            "0x03cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115"
        );
        let a1 = derive_account_keys(&phrase(TEST_MNEMONIC), 1, &Bitcoin).unwrap();
        assert_eq!(
            a1.address,
            "bc1p4qhjn9zdvkux4e44uhx8tc55attvtyu358kutcqkudyccelu0was9fqzwh"
        );
    }

    #[test]
    fn ethereum_bip44_test_vectors() {
        // Widely published m/44'/60'/0'/0/{0,1} addresses for the test mnemonic
        // (lowercase; Arktos does not apply EIP-55 checksums).
        let a0 = derive_account_keys(&phrase(TEST_MNEMONIC), 0, &Ethereum).unwrap();
        assert_eq!(a0.derivation_path, "m/44'/60'/0'/0/0");
        assert_eq!(a0.address, "0x9858effd232b4033e47d90003d41ec34ecaeda94");
        assert_eq!(
            a0.public_key,
            "0x0237b0bb7a8288d38ed49a524b5dc98cff3eb5ca824c9f9dc0dfdb3d9cd600f299"
        );
        let a1 = derive_account_keys(&phrase(TEST_MNEMONIC), 1, &Ethereum).unwrap();
        assert_eq!(a1.address, "0x6fac4d18c912343bf86fa7049364dd4e424ab9c0");
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
        let a = derive_account_keys(&phrase(TEST_MNEMONIC), 0, &Bitcoin).unwrap();
        let b = derive_account_keys(&phrase(TEST_MNEMONIC), 1, &Bitcoin).unwrap();
        assert_ne!(a.public_key, b.public_key);
        assert_ne!(a.address, b.address);
    }

    #[test]
    fn derivation_is_deterministic() {
        for chain in [Bitcoin, Ethereum] {
            let a = derive_account_keys(&phrase(TEST_MNEMONIC), 3, &chain).unwrap();
            let b = derive_account_keys(&phrase(TEST_MNEMONIC), 3, &chain).unwrap();
            assert_eq!(a.address, b.address);
            assert_eq!(a.public_key, b.public_key);
        }
    }

    #[test]
    fn invalid_mnemonic_is_rejected_without_echoing_it() {
        let err = derive_account_keys(&phrase("invalid mnemonic words"), 0, &Bitcoin).unwrap_err();
        assert!(!err.to_string().contains("invalid mnemonic words"));
    }

    #[test]
    fn invalid_utf8_phrase_is_rejected() {
        assert!(RecoveryPhrase::from_utf8(Zeroizing::new(vec![0xff, 0xfe])).is_err());
    }

    // ---- Secret hygiene -----------------------------------------------------

    #[test]
    fn debug_output_redacts_secrets() {
        let account = derive_account_keys(&phrase(TEST_MNEMONIC), 0, &Ethereum).unwrap();
        let rendered = format!("{account:?} {:?}", phrase(TEST_MNEMONIC));
        // Known private key of m/44'/60'/0'/0/0 must not be reachable anywhere.
        assert!(
            !rendered.contains("1ab42cc412b618bdea3a599e3c9bae199ebf030895b039e9db1e30dafb12b727")
        );
        assert!(!rendered.contains("abandon"));
        assert!(rendered.contains("REDACTED"));
        // Public data stays visible for diagnostics.
        assert!(rendered.contains(&account.address));
    }
}
