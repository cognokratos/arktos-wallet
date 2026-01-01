use anyhow::Result;
use tiny_keccak::{Hasher, Keccak};

/// Simple encryption using Keccak hash-based XOR cipher for demonstration
/// Production systems should use proper AES-256 encryption
pub fn encrypt_passphrase(passphrase: &str, key: &str) -> Result<String> {
    let mut hasher = Keccak::v256();
    hasher.update(key.as_bytes());
    let mut key_hash = [0u8; 32];
    hasher.finalize(&mut key_hash);

    let passphrase_bytes = passphrase.as_bytes();
    let mut encrypted = Vec::new();

    for (i, byte) in passphrase_bytes.iter().enumerate() {
        encrypted.push(byte ^ key_hash[i % 32]);
    }

    Ok(hex::encode(&encrypted))
}

/// Decrypt a passphrase encrypted with the corresponding key
pub fn decrypt_passphrase(encrypted: &str, key: &str) -> Result<String> {
    let mut hasher = Keccak::v256();
    hasher.update(key.as_bytes());
    let mut key_hash = [0u8; 32];
    hasher.finalize(&mut key_hash);

    let encrypted_bytes = hex::decode(encrypted)?;
    let mut decrypted = Vec::new();

    for (i, byte) in encrypted_bytes.iter().enumerate() {
        decrypted.push(byte ^ key_hash[i % 32]);
    }

    Ok(String::from_utf8(decrypted)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_passphrase() {
        let original = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let key = "test_key";

        let encrypted = encrypt_passphrase(original, key).expect("Should encrypt");
        let decrypted = decrypt_passphrase(&encrypted, key).expect("Should decrypt");

        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_encryption_produces_different_output_with_different_key() {
        let passphrase = "test_passphrase";
        let key1 = "key1";
        let key2 = "key2";

        let encrypted1 = encrypt_passphrase(passphrase, key1).expect("Should encrypt with key1");
        let encrypted2 = encrypt_passphrase(passphrase, key2).expect("Should encrypt with key2");

        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_wrong_key_produces_garbage_on_decrypt() {
        let passphrase = "test_passphrase";
        let encrypted = encrypt_passphrase(passphrase, "key1").expect("Should encrypt");
        let decrypted_result = decrypt_passphrase(&encrypted, "wrong_key");

        // With a wrong key, decryption may produce invalid UTF-8 or wrong content
        if let Ok(decrypted) = decrypted_result {
            assert_ne!(
                passphrase, decrypted,
                "Decrypted with wrong key should not match"
            );
        }
    }
}
