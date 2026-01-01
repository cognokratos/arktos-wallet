use anyhow::Result;
use bip39::Mnemonic;
use rand::RngCore;

/// Generate a BIP39 mnemonic (recovery passphrase) using 12 words
pub fn generate_recovery_passphrase() -> Result<String> {
    let mut entropy = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut entropy);
    let mnemonic = Mnemonic::from_entropy(&entropy)?;
    Ok(mnemonic.to_string())
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
}
