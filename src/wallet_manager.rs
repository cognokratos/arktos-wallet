use crate::wallet::ChainType;
use anyhow::{Result, anyhow};
use bip32::{DerivationPath, XPrv};
use bip39::Mnemonic;
use bitcoin::{
    Network, PublicKey,
    address::Address,
    secp256k1::{Secp256k1, XOnlyPublicKey},
};
use rand::RngCore;
use std::str::FromStr;
use tiny_keccak::{Hasher, Keccak};

/// Generate a BIP39 mnemonic (recovery passphrase) using 12 words
pub fn generate_recovery_passphrase() -> Result<String> {
    let mut entropy = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut entropy);
    let mnemonic = Mnemonic::from_entropy(&entropy)?;
    Ok(mnemonic.to_string())
}

pub struct AccountData {
    pub derivation_path: String,
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}

/// Derive an account's private/public keypair and address for a given chain.
///
/// Returns AccountData containing:
/// - `derivation_path` used for the account
/// - `private_key` is the 66-byte secp256k1 secret key (raw bytes)
/// - `public_key` is the compressed SEC1-encoded public key (raw bytes, 68 bytes)
/// - `address` is the derived address for the account
pub fn derive_account_keys(
    mnemonic: &str,
    account_index: u32,
    chain_type: &ChainType,
) -> Result<AccountData> {
    let mnemonic =
        Mnemonic::parse_normalized(mnemonic).map_err(|e| anyhow!("Invalid mnemonic: {}", e))?;

    let seed = mnemonic.to_seed("");
    let mut xprv = XPrv::new(seed)?;

    let derivation_path = match chain_type {
        ChainType::Bitcoin => format!("m/86'/0'/0'/0/{}", account_index),
        ChainType::Ethereum => format!("m/44'/60'/0'/0/{}", account_index),
    };

    let path = DerivationPath::from_str(&derivation_path)
        .map_err(|e| anyhow!("Failed to parse derivation path: {}", e))?;

    for child_num in path.into_iter() {
        xprv = xprv
            .derive_child(child_num)
            .map_err(|e| anyhow!("Failed to derive child key: {}", e))?;
    }

    let private_key = xprv.private_key().to_bytes().to_vec();
    let public_key = xprv.public_key().to_bytes().to_vec();

    let address = match chain_type {
        ChainType::Bitcoin => derive_bitcoin_address(&public_key)?,
        ChainType::Ethereum => derive_ethereum_address(&public_key)?,
    };

    let private_key = format!("0x{}", hex::encode(private_key));
    let public_key = format!("0x{}", hex::encode(public_key));

    Ok(AccountData {
        derivation_path,
        private_key,
        public_key,
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

    #[test]
    fn test_generate_recovery_passphrase_returns_string() {
        let result = generate_recovery_passphrase();
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_recovery_passphrase_returns_12_words() {
        let passphrase = generate_recovery_passphrase().expect("Should generate passphrase");
        let words: Vec<&str> = passphrase.split_whitespace().collect();
        assert_eq!(
            words.len(),
            12,
            "Passphrase should contain exactly 12 words"
        );
    }

    #[test]
    fn test_generate_recovery_passphrase_generates_unique_passphrases() {
        let passphrase1 = generate_recovery_passphrase().expect("Should generate passphrase 1");
        let passphrase2 = generate_recovery_passphrase().expect("Should generate passphrase 2");
        assert_ne!(
            passphrase1, passphrase2,
            "Two calls should generate different passphrases"
        );
    }

    #[test]
    fn test_generated_passphrase_is_valid_bip39() {
        let passphrase = generate_recovery_passphrase().expect("Should generate passphrase");
        let mnemonic = Mnemonic::parse_normalized(&passphrase);
        assert!(
            mnemonic.is_ok(),
            "Generated passphrase should be a valid BIP39 mnemonic"
        );
    }

    #[test]
    fn test_derive_account_keys_bitcoin() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let account =
            derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive Bitcoin keys");

        assert!(
            !account.private_key.is_empty(),
            "Private key should not be empty"
        );
        assert!(
            !account.public_key.is_empty(),
            "Public key should not be empty"
        );
        assert_eq!(
            account.private_key.len(),
            66,
            "Private key should be 66 bytes"
        );
        assert_eq!(
            account.public_key.len(),
            68,
            "Public key should be 68 bytes (compressed)"
        );
        assert!(
            account.address.starts_with("bc1"),
            "Bitcoin address should start with bc1"
        );
        assert_eq!(
            account.derivation_path, "m/86'/0'/0'/0/0",
            "Derivation path should match Bitcoin standard"
        );
    }

    #[test]
    fn test_derive_account_keys_ethereum() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let account =
            derive_account_keys(mnemonic, 0, &Ethereum).expect("Should derive Ethereum keys");

        assert!(
            !account.private_key.is_empty(),
            "Private key should not be empty"
        );
        assert!(
            !account.public_key.is_empty(),
            "Public key should not be empty"
        );
        assert_eq!(
            account.private_key.len(),
            66,
            "Private key should be 66 bytes"
        );
        assert_eq!(
            account.public_key.len(),
            68,
            "Public key should be 68 bytes (compressed)"
        );
        assert!(
            account.address.starts_with("0x"),
            "Ethereum address should start with 0x"
        );
        assert_eq!(
            account.address.len(),
            42,
            "Ethereum address should be 42 characters (0x + 40 hex)"
        );
        assert_eq!(
            account.derivation_path, "m/44'/60'/0'/0/0",
            "Derivation path should match Ethereum standard"
        );
    }

    #[test]
    fn test_derive_account_keys_different_indices() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let account_1 =
            derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive first account");
        let account_2 =
            derive_account_keys(mnemonic, 1, &Bitcoin).expect("Should derive second account");

        assert_ne!(
            account_1.private_key, account_2.private_key,
            "Different indices should produce different private keys"
        );
        assert_ne!(
            account_1.public_key, account_2.public_key,
            "Different indices should produce different public keys"
        );
        assert_ne!(
            account_1.address, account_2.address,
            "Different indices should produce different addresses"
        );
    }

    #[test]
    fn test_derive_account_keys_invalid_mnemonic() {
        let result = derive_account_keys("invalid mnemonic words", 0, &Bitcoin);
        assert!(result.is_err(), "Should reject invalid mnemonic");
    }

    #[test]
    fn test_derive_bitcoin_address_from_valid_public_key() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let account =
            derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive Bitcoin keys");

        // Bitcoin address should start with bc1 for Taproot (mainnet)
        assert!(
            account.address.starts_with("bc1"),
            "Bitcoin address should start with bc1"
        );
        assert!(
            !account.address.is_empty(),
            "Bitcoin address should not be empty"
        );
    }

    #[test]
    fn test_derive_bitcoin_address_consistent() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let account_1 =
            derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive Bitcoin keys");
        let account_2 =
            derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive Bitcoin keys");

        assert_eq!(
            account_1.address, account_2.address,
            "Same public key should produce same address"
        );
    }

    #[test]
    fn test_derive_bitcoin_address_different_keys_produce_different_addresses() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let account_1 = derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive key 1");
        let account_2 = derive_account_keys(mnemonic, 1, &Bitcoin).expect("Should derive key 2");

        assert_ne!(
            account_1.address, account_2.address,
            "Different public keys should produce different addresses"
        );
    }

    #[test]
    fn test_derive_ethereum_address_from_valid_public_key() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let account =
            derive_account_keys(mnemonic, 0, &Ethereum).expect("Should derive Ethereum keys");

        // Ethereum address should start with 0x and be 42 chars long (0x + 40 hex chars)
        assert!(
            account.address.starts_with("0x"),
            "Ethereum address should start with 0x"
        );
        assert_eq!(
            account.address.len(),
            42,
            "Ethereum address should be 42 characters (0x + 40 hex)"
        );
    }

    #[test]
    fn test_derive_ethereum_address_consistent() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let account_1 =
            derive_account_keys(mnemonic, 0, &Ethereum).expect("Should derive Ethereum keys");
        let account_2 =
            derive_account_keys(mnemonic, 0, &Ethereum).expect("Should derive Ethereum keys");

        assert_eq!(
            account_1.address, account_2.address,
            "Same public key should produce same Ethereum address"
        );
    }

    #[test]
    fn test_derive_ethereum_address_different_keys_produce_different_addresses() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let account_1 = derive_account_keys(mnemonic, 0, &Ethereum).expect("Should derive key 1");
        let account_2 = derive_account_keys(mnemonic, 1, &Ethereum).expect("Should derive key 2");

        assert_ne!(
            account_1.address, account_2.address,
            "Different public keys should produce different Ethereum addresses"
        );
    }
}
