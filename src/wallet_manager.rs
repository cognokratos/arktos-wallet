use crate::wallet::ChainType;
use anyhow::{Result, anyhow};
use bip32::{DerivationPath, XPrv};
use bip39::Mnemonic;
use bitcoin::Network;
use bitcoin::PublicKey;
use bitcoin::address::Address;
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

/// Derive account keys using BIP32/BIP44 standard
/// Takes a mnemonic and account index, returns (private_key_hex, public_key_hex, derivation_path)
pub fn derive_account_keys(
    mnemonic: &str,
    account_index: u32,
    chain_type: &ChainType,
) -> Result<(String, String, String)> {
    let mnemonic =
        Mnemonic::parse_normalized(mnemonic).map_err(|e| anyhow!("Invalid mnemonic: {}", e))?;

    let seed = mnemonic.to_seed("");

    let mut xprv = XPrv::new(seed)?;

    let derivation_path = match chain_type {
        ChainType::Bitcoin => format!("m/44'/0'/0'/0/{}", account_index),
        ChainType::Ethereum => format!("m/44'/60'/0'/0/{}", account_index),
    };

    let path = DerivationPath::from_str(&derivation_path)
        .map_err(|e| anyhow!("Failed to parse derivation path: {}", e))?;

    for child_num in path.into_iter() {
        xprv = xprv
            .derive_child(child_num)
            .map_err(|e| anyhow!("Failed to derive child key: {}", e))?;
    }

    let private_key_bytes = xprv.private_key().to_bytes();
    let private_key_hex = hex::encode(private_key_bytes);

    let public_key_bytes = xprv.public_key().to_bytes();
    let public_key_hex = hex::encode(public_key_bytes);

    Ok((private_key_hex, public_key_hex, derivation_path))
}

/// Derive a Bitcoin address from a public key (hex string)
/// Returns a valid Bitcoin address for mainnet
pub fn derive_bitcoin_address(public_key_hex: &str) -> Result<String> {
    let public_key_bytes =
        hex::decode(public_key_hex).map_err(|e| anyhow!("Failed to decode public key: {}", e))?;

    let public_key = PublicKey::from_slice(&public_key_bytes)
        .map_err(|e| anyhow!("Invalid public key format: {}", e))?;

    let address = Address::p2pkh(public_key, Network::Bitcoin);
    Ok(address.to_string())
}

/// Derive an Ethereum address from a public key (hex string)
/// Ethereum uses Keccak-256 hash of the uncompressed public key (without 0x04 prefix)
/// Takes the last 20 bytes and returns with 0x prefix
pub fn derive_ethereum_address(public_key_hex: &str) -> Result<String> {
    let public_key_bytes =
        hex::decode(public_key_hex).map_err(|e| anyhow!("Failed to decode public key: {}", e))?;

    let public_key = PublicKey::from_slice(&public_key_bytes)
        .map_err(|e| anyhow!("Invalid public key format: {}", e))?;

    // Get uncompressed public key bytes (includes the 0x04 prefix)
    let uncompressed_bytes = public_key.inner.serialize_uncompressed().to_vec();

    // Remove the 0x04 prefix (first byte)
    let key_material = if uncompressed_bytes.len() == 65 && uncompressed_bytes[0] == 0x04 {
        &uncompressed_bytes[1..]
    } else {
        &uncompressed_bytes
    };

    // Hash with Keccak-256
    let mut hasher = Keccak::v256();
    hasher.update(key_material);
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    // Take last 20 bytes
    let address_bytes = &hash[12..];
    let address_hex = hex::encode(address_bytes);

    Ok(format!("0x{}", address_hex))
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
        let (private_key, public_key, path) =
            derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive Bitcoin keys");

        assert!(!private_key.is_empty(), "Private key should not be empty");
        assert!(!public_key.is_empty(), "Public key should not be empty");
        assert_eq!(
            path, "m/44'/0'/0'/0/0",
            "Derivation path should match Bitcoin standard"
        );
    }

    #[test]
    fn test_derive_account_keys_ethereum() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (private_key, public_key, path) =
            derive_account_keys(mnemonic, 0, &Ethereum).expect("Should derive Ethereum keys");

        assert!(!private_key.is_empty(), "Private key should not be empty");
        assert!(!public_key.is_empty(), "Public key should not be empty");
        assert_eq!(
            path, "m/44'/60'/0'/0/0",
            "Derivation path should match Ethereum standard"
        );
    }

    #[test]
    fn test_derive_account_keys_different_indices() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (priv1, pub1, _) =
            derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive first account");
        let (priv2, pub2, _) =
            derive_account_keys(mnemonic, 1, &Bitcoin).expect("Should derive second account");

        assert_ne!(
            priv1, priv2,
            "Different indices should produce different private keys"
        );
        assert_ne!(
            pub1, pub2,
            "Different indices should produce different public keys"
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
        let (_, public_key_hex, _) =
            derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive Bitcoin keys");

        let address = derive_bitcoin_address(&public_key_hex).expect("Should derive address");

        // Bitcoin address should start with 1 for P2PKH (mainnet)
        assert!(
            address.starts_with("1"),
            "Bitcoin address should start with 1"
        );
        assert!(!address.is_empty(), "Bitcoin address should not be empty");
    }

    #[test]
    fn test_derive_bitcoin_address_consistent() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (_, public_key_hex, _) =
            derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive Bitcoin keys");

        let address1 = derive_bitcoin_address(&public_key_hex).expect("Should derive address 1");
        let address2 = derive_bitcoin_address(&public_key_hex).expect("Should derive address 2");

        assert_eq!(
            address1, address2,
            "Same public key should produce same address"
        );
    }

    #[test]
    fn test_derive_bitcoin_address_different_keys_produce_different_addresses() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (_, public_key_hex1, _) =
            derive_account_keys(mnemonic, 0, &Bitcoin).expect("Should derive key 1");
        let (_, public_key_hex2, _) =
            derive_account_keys(mnemonic, 1, &Bitcoin).expect("Should derive key 2");

        let address1 = derive_bitcoin_address(&public_key_hex1).expect("Should derive address 1");
        let address2 = derive_bitcoin_address(&public_key_hex2).expect("Should derive address 2");

        assert_ne!(
            address1, address2,
            "Different public keys should produce different addresses"
        );
    }

    #[test]
    fn test_derive_ethereum_address_from_valid_public_key() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (_, public_key_hex, _) =
            derive_account_keys(mnemonic, 0, &Ethereum).expect("Should derive Ethereum keys");

        let address = derive_ethereum_address(&public_key_hex).expect("Should derive address");

        // Ethereum address should start with 0x and be 42 chars long (0x + 40 hex chars)
        assert!(
            address.starts_with("0x"),
            "Ethereum address should start with 0x"
        );
        assert_eq!(
            address.len(),
            42,
            "Ethereum address should be 42 characters (0x + 40 hex)"
        );
    }

    #[test]
    fn test_derive_ethereum_address_consistent() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (_, public_key_hex, _) =
            derive_account_keys(mnemonic, 0, &Ethereum).expect("Should derive Ethereum keys");

        let address1 = derive_ethereum_address(&public_key_hex).expect("Should derive address 1");
        let address2 = derive_ethereum_address(&public_key_hex).expect("Should derive address 2");

        assert_eq!(
            address1, address2,
            "Same public key should produce same Ethereum address"
        );
    }

    #[test]
    fn test_derive_ethereum_address_different_keys_produce_different_addresses() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let (_, public_key_hex1, _) =
            derive_account_keys(mnemonic, 0, &Ethereum).expect("Should derive key 1");
        let (_, public_key_hex2, _) =
            derive_account_keys(mnemonic, 1, &Ethereum).expect("Should derive key 2");

        let address1 = derive_ethereum_address(&public_key_hex1).expect("Should derive address 1");
        let address2 = derive_ethereum_address(&public_key_hex2).expect("Should derive address 2");

        assert_ne!(
            address1, address2,
            "Different public keys should produce different Ethereum addresses"
        );
    }
}
