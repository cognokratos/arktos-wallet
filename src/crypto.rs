//! Field-level authenticated encryption (AES-256-GCM) for secrets stored in
//! the database.
//!
//! Values are written as a versioned JSON envelope:
//!
//! ```json
//! {"v":1,"alg":"A256GCM","nonce":"<base64url 12 bytes>","ct":"<base64url ciphertext‖tag>"}
//! ```
//!
//! The associated data binds the version, algorithm and key purpose, so a
//! value encrypted for one purpose cannot be opened as another.

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit, Nonce, Payload};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

/// AES-256 key length in bytes.
pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const ENVELOPE_VERSION: u32 = 1;
const ALG_A256GCM: &str = "A256GCM";

/// Controlled, non-secret description of a cryptographic failure.
///
/// Authentication failures (wrong key, tampered nonce, ciphertext or tag)
/// are deliberately indistinguishable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Not a decodable envelope (bad JSON, bad base64, truncated, ...).
    MalformedEnvelope,
    /// Well-formed envelope with an unknown version.
    UnsupportedVersion(u32),
    /// Well-formed envelope with an unknown algorithm.
    UnsupportedAlgorithm,
    /// Nonce is not 12 bytes.
    InvalidNonce,
    /// Authentication failed.
    DecryptionFailed,
    EncryptionFailed,
    /// The OS random number generator failed.
    RandomnessUnavailable,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::MalformedEnvelope => write!(f, "invalid encrypted-secret envelope"),
            CryptoError::UnsupportedVersion(v) => {
                write!(f, "unsupported encrypted-secret version {v}")
            }
            CryptoError::UnsupportedAlgorithm => {
                write!(f, "unsupported encrypted-secret algorithm")
            }
            CryptoError::InvalidNonce => write!(f, "invalid encrypted-secret nonce"),
            CryptoError::DecryptionFailed => write!(f, "failed to decrypt secret"),
            CryptoError::EncryptionFailed => write!(f, "failed to encrypt secret"),
            CryptoError::RandomnessUnavailable => write!(f, "OS random number generator failed"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// A 256-bit AES-GCM key bound to one purpose. Zeroized on drop.
pub struct AeadKey {
    key: Zeroizing<[u8; KEY_LEN]>,
    /// Bound into the associated data.
    purpose: &'static str,
}

impl AeadKey {
    pub(crate) fn new(key: Zeroizing<[u8; KEY_LEN]>, purpose: &'static str) -> Self {
        Self { key, purpose }
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new(self.key.as_slice().try_into().expect("32-byte key"))
    }

    fn aad(&self) -> Vec<u8> {
        format!("arktos:v{ENVELOPE_VERSION}:{ALG_A256GCM}:{}", self.purpose).into_bytes()
    }
}

impl fmt::Debug for AeadKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AeadKey")
            .field("purpose", &self.purpose)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

/// Stored representation of an encrypted secret (version 1).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvelopeV1 {
    v: u32,
    alg: String,
    nonce: String,
    ct: String,
}

/// Only the version, read first so unknown versions fail explicitly even if
/// their other fields differ.
#[derive(Deserialize)]
struct EnvelopeVersion {
    v: u32,
}

/// Encrypt `plaintext` with a fresh random nonce and return a v1 envelope.
pub fn seal(key: &AeadKey, plaintext: &[u8]) -> Result<String, CryptoError> {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|_| CryptoError::RandomnessUnavailable)?;
    let aad = key.aad();
    let ciphertext = key
        .cipher()
        .encrypt(
            &Nonce::<Aes256Gcm>::from(nonce),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::EncryptionFailed)?;
    let envelope = EnvelopeV1 {
        v: ENVELOPE_VERSION,
        alg: ALG_A256GCM.to_string(),
        nonce: B64.encode(nonce),
        ct: B64.encode(ciphertext),
    };
    serde_json::to_string(&envelope).map_err(|_| CryptoError::EncryptionFailed)
}

/// Decrypt a stored envelope.
pub fn open(key: &AeadKey, stored: &str) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let EnvelopeVersion { v } =
        serde_json::from_str(stored).map_err(|_| CryptoError::MalformedEnvelope)?;
    if v != ENVELOPE_VERSION {
        return Err(CryptoError::UnsupportedVersion(v));
    }
    let envelope: EnvelopeV1 =
        serde_json::from_str(stored).map_err(|_| CryptoError::MalformedEnvelope)?;
    if envelope.alg != ALG_A256GCM {
        return Err(CryptoError::UnsupportedAlgorithm);
    }
    let nonce: [u8; NONCE_LEN] = B64
        .decode(&envelope.nonce)
        .map_err(|_| CryptoError::MalformedEnvelope)?
        .try_into()
        .map_err(|_| CryptoError::InvalidNonce)?;
    let ciphertext = B64
        .decode(&envelope.ct)
        .map_err(|_| CryptoError::MalformedEnvelope)?;
    if ciphertext.len() < TAG_LEN {
        return Err(CryptoError::MalformedEnvelope);
    }
    let aad = key.aad();
    key.cipher()
        .decrypt(
            &Nonce::<Aes256Gcm>::from(nonce),
            Payload {
                msg: &ciphertext,
                aad: &aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| CryptoError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn key(byte: u8, purpose: &'static str) -> AeadKey {
        AeadKey::new(Zeroizing::new([byte; KEY_LEN]), purpose)
    }

    fn tamper(envelope: &str, field: &str, f: impl FnOnce(&mut Vec<u8>)) -> String {
        let mut value: Value = serde_json::from_str(envelope).unwrap();
        let mut bytes = B64.decode(value[field].as_str().unwrap()).unwrap();
        f(&mut bytes);
        value[field] = json!(B64.encode(bytes));
        value.to_string()
    }

    #[test]
    fn round_trip() {
        let k = key(1, "wallet-seed");
        let sealed = seal(&k, MNEMONIC.as_bytes()).unwrap();
        assert_eq!(open(&k, &sealed).unwrap().as_slice(), MNEMONIC.as_bytes());
    }

    #[test]
    fn envelope_is_versioned_and_does_not_contain_plaintext() {
        let sealed = seal(&key(1, "wallet-seed"), MNEMONIC.as_bytes()).unwrap();
        let value: Value = serde_json::from_str(&sealed).unwrap();
        assert_eq!(value["v"], 1);
        assert_eq!(value["alg"], "A256GCM");
        assert_eq!(
            B64.decode(value["nonce"].as_str().unwrap()).unwrap().len(),
            12
        );
        assert!(!sealed.contains("abandon"));
    }

    #[test]
    fn nonces_and_ciphertexts_are_unique() {
        let k = key(1, "wallet-seed");
        let sealed: Vec<Value> = (0..32)
            .map(|_| serde_json::from_str(&seal(&k, b"same plaintext").unwrap()).unwrap())
            .collect();
        for (i, a) in sealed.iter().enumerate() {
            for b in &sealed[i + 1..] {
                assert_ne!(a["nonce"], b["nonce"]);
                assert_ne!(a["ct"], b["ct"]);
            }
        }
    }

    #[test]
    fn wrong_key_fails() {
        let sealed = seal(&key(1, "wallet-seed"), b"secret").unwrap();
        assert_eq!(
            open(&key(2, "wallet-seed"), &sealed).unwrap_err(),
            CryptoError::DecryptionFailed
        );
    }

    #[test]
    fn purpose_is_bound_to_ciphertext() {
        // Same key bytes, different purpose: associated data no longer matches.
        let sealed = seal(&key(1, "wallet-seed"), b"secret").unwrap();
        assert_eq!(
            open(&key(1, "other-purpose"), &sealed).unwrap_err(),
            CryptoError::DecryptionFailed
        );
    }

    #[test]
    fn modified_ciphertext_fails() {
        let k = key(1, "wallet-seed");
        let sealed = seal(&k, b"secret").unwrap();
        for index in [0usize, 3, 6 + TAG_LEN - 1] {
            let tampered = tamper(&sealed, "ct", |ct| ct[index] ^= 0x01);
            assert_eq!(
                open(&k, &tampered).unwrap_err(),
                CryptoError::DecryptionFailed
            );
        }
    }

    #[test]
    fn modified_nonce_fails() {
        let k = key(1, "wallet-seed");
        let sealed = seal(&k, b"secret").unwrap();
        let tampered = tamper(&sealed, "nonce", |n| n[0] ^= 0x80);
        assert_eq!(
            open(&k, &tampered).unwrap_err(),
            CryptoError::DecryptionFailed
        );
    }

    #[test]
    fn wrong_nonce_length_fails() {
        let k = key(1, "wallet-seed");
        let sealed = seal(&k, b"secret").unwrap();
        let short = tamper(&sealed, "nonce", |n| n.truncate(8));
        assert_eq!(open(&k, &short).unwrap_err(), CryptoError::InvalidNonce);
    }

    #[test]
    fn truncated_envelopes_fail_cleanly() {
        let k = key(1, "wallet-seed");
        let sealed = seal(&k, b"secret").unwrap();
        let no_tag = tamper(&sealed, "ct", |ct| ct.truncate(TAG_LEN - 1));
        assert_eq!(
            open(&k, &no_tag).unwrap_err(),
            CryptoError::MalformedEnvelope
        );
        let cut = &sealed[..sealed.len() / 2];
        assert_eq!(open(&k, cut).unwrap_err(), CryptoError::MalformedEnvelope);
        assert_eq!(
            open(&k, "{\"v\":1}").unwrap_err(),
            CryptoError::MalformedEnvelope
        );
    }

    #[test]
    fn invalid_encoding_fails_cleanly() {
        let k = key(1, "wallet-seed");
        let bad_b64 = json!({"v":1,"alg":"A256GCM","nonce":"!!!","ct":"AAAA"}).to_string();
        assert_eq!(
            open(&k, &bad_b64).unwrap_err(),
            CryptoError::MalformedEnvelope
        );
        assert_eq!(
            open(&k, "not*json").unwrap_err(),
            CryptoError::MalformedEnvelope
        );
        assert_eq!(
            open(&k, "AAECAwQFBgcICQoL").unwrap_err(),
            CryptoError::MalformedEnvelope,
            "bare base64 without an envelope is rejected"
        );
        let extra = json!({"v":1,"alg":"A256GCM","nonce":"","ct":"","x":1}).to_string();
        assert_eq!(
            open(&k, &extra).unwrap_err(),
            CryptoError::MalformedEnvelope
        );
    }

    #[test]
    fn unknown_version_fails_explicitly() {
        let k = key(1, "wallet-seed");
        let v2 = json!({"v":2,"alg":"XCHACHA","payload":"..."}).to_string();
        assert_eq!(
            open(&k, &v2).unwrap_err(),
            CryptoError::UnsupportedVersion(2)
        );
    }

    #[test]
    fn unknown_algorithm_fails_explicitly() {
        let k = key(1, "wallet-seed");
        let sealed = seal(&k, b"secret").unwrap();
        let mut value: Value = serde_json::from_str(&sealed).unwrap();
        value["alg"] = json!("A128GCM");
        assert_eq!(
            open(&k, &value.to_string()).unwrap_err(),
            CryptoError::UnsupportedAlgorithm
        );
    }

    #[test]
    fn errors_and_debug_do_not_leak_secrets() {
        let k = key(0xab, "wallet-seed");
        let sealed = seal(&k, MNEMONIC.as_bytes()).unwrap();
        let err = open(&key(0xcd, "wallet-seed"), &sealed).unwrap_err();
        let rendered = format!("{err} {err:?} {k:?}");
        assert!(!rendered.contains("abandon"));
        assert!(!rendered.contains("abab"));
        assert!(!rendered.contains("171"), "no raw key bytes");
        assert!(rendered.contains("REDACTED"));
    }
}
