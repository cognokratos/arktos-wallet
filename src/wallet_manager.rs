use anyhow::{Result, anyhow};
use bip32::{DerivationPath, XPrv};
use bip39::Mnemonic;
use rand::RngCore;
use std::str::FromStr;

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
    chain_type: &str,
) -> Result<(String, String, String)> {
    let mnemonic =
        Mnemonic::parse_normalized(mnemonic).map_err(|e| anyhow!("Invalid mnemonic: {}", e))?;

    let seed = mnemonic.to_seed("");

    let mut xprv = XPrv::new(seed)?;

    let derivation_path = match chain_type {
        "Bitcoin" => format!("m/44'/0'/0'/0/{}", account_index),
        "Ethereum" => format!("m/44'/60'/0'/0/{}", account_index),
        other => return Err(anyhow!("Unsupported chain type: {}", other)),
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

#[cfg(test)]
mod tests {
    use super::*;

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
            derive_account_keys(mnemonic, 0, "Bitcoin").expect("Should derive Bitcoin keys");

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
            derive_account_keys(mnemonic, 0, "Ethereum").expect("Should derive Ethereum keys");

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
            derive_account_keys(mnemonic, 0, "Bitcoin").expect("Should derive first account");
        let (priv2, pub2, _) =
            derive_account_keys(mnemonic, 1, "Bitcoin").expect("Should derive second account");

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
        let result = derive_account_keys("invalid mnemonic words", 0, "Bitcoin");
        assert!(result.is_err(), "Should reject invalid mnemonic");
    }

    #[test]
    fn test_derive_account_keys_unsupported_chain() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let result = derive_account_keys(mnemonic, 0, "UnsupportedChain");
        assert!(result.is_err(), "Should reject unsupported chain type");
    }
}
