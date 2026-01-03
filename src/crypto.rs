use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use rand::RngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

const NONCE_LEN: usize = 12;

fn derive_aes256_key(key: &str) -> [u8; 32] {
    let digest = Sha256::digest(key.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Encrypts `secret` using AES-256-GCM.
/// Returns base64( nonce(12) || ciphertext+tag ).
pub fn encrypt_secret(secret: &str, key: &str) -> Result<String> {
    let key_bytes = derive_aes256_key(key);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).expect("AES-256 key must be 32 bytes");

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let mut ct_and_tag = cipher
        .encrypt(&nonce, secret.as_bytes())
        .map_err(|e| anyhow!("encrypt failed: {e:?}"))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ct_and_tag.len());
    out.extend_from_slice(&nonce_bytes);
    out.append(&mut ct_and_tag);

    Ok(B64.encode(out))
}

/// Decrypts base64( nonce || ciphertext+tag ) using AES-256-GCM.
pub fn decrypt_secret(encrypted_b64: &str, key: &str) -> Result<String> {
    let blob = B64
        .decode(encrypted_b64.trim())
        .context("base64 decode failed")?;

    if blob.len() < NONCE_LEN + 16 {
        return Err(anyhow!("ciphertext too short"));
    }

    let (nonce_bytes, ct_and_tag) = blob.split_at(NONCE_LEN);

    let key_bytes = derive_aes256_key(key);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).expect("AES-256 key must be 32 bytes");
    let nonce = Nonce::try_from(nonce_bytes).expect("nonce length must be 12 bytes");

    let pt = cipher
        .decrypt(&nonce, ct_and_tag)
        .map_err(|_| anyhow!("decrypt failed (wrong key or tampered ciphertext)"))?;

    String::from_utf8(pt).context("utf8 decode failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_passphrase() {
        let original = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let key = "test_key";

        let encrypted = encrypt_secret(original, key).expect("Should encrypt");
        let decrypted = decrypt_secret(&encrypted, key).expect("Should decrypt");

        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_encryption_produces_different_output_with_different_key() {
        let passphrase = "test_passphrase";
        let key1 = "key1";
        let key2 = "key2";

        let encrypted1 = encrypt_secret(passphrase, key1).expect("Should encrypt with key1");
        let encrypted2 = encrypt_secret(passphrase, key2).expect("Should encrypt with key2");

        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_wrong_key_produces_garbage_on_decrypt() {
        let passphrase = "test_passphrase";
        let encrypted = encrypt_secret(passphrase, "key1").expect("Should encrypt");
        let decrypted_result = decrypt_secret(&encrypted, "wrong_key");

        // With a wrong key, decryption may produce invalid UTF-8 or wrong content
        if let Ok(decrypted) = decrypted_result {
            assert_ne!(
                passphrase, decrypted,
                "Decrypted with wrong key should not match"
            );
        }
    }
}
