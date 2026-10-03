//! Application key hierarchy.
//!
//! ```text
//! MASTER_KEY (32 random bytes, base64)
//!  └─ HKDF-SHA256 (no salt, purpose label as `info`)
//!      ├─ "arktos/api-key-hmac/v1"            → ApiKeyHmacKey
//!      └─ "arktos/wallet-seed-encryption/v1"  → WalletSeedKey
//! ```
//!
//! Each derived key has its own type, so one purpose's key cannot be passed
//! where another is expected. All key material is zeroized on drop and is
//! never printed by `Debug`. The SQLCipher `DATABASE_KEY` is deliberately
//! independent of this hierarchy.

use crate::crypto::{AeadKey, KEY_LEN};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use hkdf::Hkdf;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::fmt;
use zeroize::Zeroizing;

const API_KEY_HMAC_INFO: &[u8] = b"arktos/api-key-hmac/v1";
const WALLET_SEED_INFO: &[u8] = b"arktos/wallet-seed-encryption/v1";

/// Why a configured key could not be loaded. Never contains the key value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// Not valid base64 (standard or URL-safe alphabet).
    InvalidEncoding,
    /// Decoded to the wrong number of bytes.
    InvalidLength,
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyError::InvalidEncoding => {
                write!(
                    f,
                    "not valid base64; expected a {KEY_LEN}-byte base64 value"
                )
            }
            KeyError::InvalidLength => {
                write!(f, "wrong length; expected a {KEY_LEN}-byte base64 value")
            }
        }
    }
}

impl std::error::Error for KeyError {}

/// Root application secret. Only used to derive purpose-specific subkeys.
pub struct MasterKey(Zeroizing<[u8; KEY_LEN]>);

impl MasterKey {
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// Parse 32 bytes encoded as base64 (standard or URL-safe, padding optional).
    pub fn from_base64(encoded: &str) -> Result<Self, KeyError> {
        let decoded = Zeroizing::new(decode_base64(encoded.trim())?);
        let bytes: [u8; KEY_LEN] = decoded
            .as_slice()
            .try_into()
            .map_err(|_| KeyError::InvalidLength)?;
        Ok(Self::from_bytes(bytes))
    }

    /// HKDF-SHA256 expand for one purpose label.
    fn derive(&self, info: &[u8]) -> Zeroizing<[u8; KEY_LEN]> {
        let hkdf = Hkdf::<Sha256>::new(None, self.0.as_slice());
        let mut okm = Zeroizing::new([0u8; KEY_LEN]);
        hkdf.expand(info, okm.as_mut_slice())
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        okm
    }
}

impl fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MasterKey([REDACTED])")
    }
}

fn decode_base64(encoded: &str) -> Result<Vec<u8>, KeyError> {
    const INDIFFERENT: GeneralPurposeConfig =
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent);
    let standard = GeneralPurpose::new(&base64::alphabet::STANDARD, INDIFFERENT);
    let url_safe = GeneralPurpose::new(&base64::alphabet::URL_SAFE, INDIFFERENT);
    standard
        .decode(encoded)
        .or_else(|_| url_safe.decode(encoded))
        .map_err(|_| KeyError::InvalidEncoding)
}

/// Generate a new random master key, base64-encoded (for operators/tooling).
pub fn generate_master_key_base64() -> Result<String, getrandom::Error> {
    let mut bytes = Zeroizing::new([0u8; KEY_LEN]);
    getrandom::fill(bytes.as_mut_slice())?;
    Ok(STANDARD.encode(bytes.as_slice()))
}

/// HMAC-SHA256 key for hashing client API keys.
pub struct ApiKeyHmacKey(Zeroizing<[u8; KEY_LEN]>);

impl ApiKeyHmacKey {
    /// Hex HMAC-SHA256 of an API key, as stored in `api_keys.key_hash`.
    pub fn hash(&self, api_key: &str) -> String {
        hmac_sha256_hex(self.0.as_slice(), api_key)
    }
}

impl fmt::Debug for ApiKeyHmacKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKeyHmacKey([REDACTED])")
    }
}

/// AES-256-GCM key for wallet recovery phrases (`wallets.encrypted_passphrase`).
#[derive(Debug)]
pub struct WalletSeedKey(AeadKey);

impl WalletSeedKey {
    pub fn aead(&self) -> &AeadKey {
        &self.0
    }
}

fn hmac_sha256_hex(key: &[u8], message: &str) -> String {
    let mut mac =
        <Hmac<Sha256> as KeyInit>::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Keys used by [`KeyServices`](crate::key_services::KeyServices).
#[derive(Debug)]
pub struct ApiKeyKeys {
    pub hmac: ApiKeyHmacKey,
}

/// Keys used by [`WalletServices`](crate::wallet_services::WalletServices).
#[derive(Debug)]
pub struct WalletKeys {
    pub seed: WalletSeedKey,
}

/// All purpose-specific keys derived from one master key.
#[derive(Debug)]
pub struct Keyring {
    pub api_keys: ApiKeyKeys,
    pub wallet: WalletKeys,
}

impl Keyring {
    pub fn new(master: &MasterKey) -> Self {
        Self {
            api_keys: ApiKeyKeys {
                hmac: ApiKeyHmacKey(master.derive(API_KEY_HMAC_INFO)),
            },
            wallet: WalletKeys {
                seed: WalletSeedKey(AeadKey::new(master.derive(WALLET_SEED_INFO), "wallet-seed")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE;

    const MASTER_A: [u8; 32] = [0x11; 32];
    const MASTER_B: [u8; 32] = [0x22; 32];

    fn subkeys(master: &MasterKey) -> [Zeroizing<[u8; 32]>; 2] {
        [
            master.derive(API_KEY_HMAC_INFO),
            master.derive(WALLET_SEED_INFO),
        ]
    }

    #[test]
    fn hkdf_matches_rfc5869_test_case_3() {
        // RFC 5869 A.3: SHA-256, 22-byte IKM, no salt, empty info, L = 42.
        let ikm = [0x0b; 22];
        let mut okm = [0u8; 42];
        Hkdf::<Sha256>::new(None, &ikm)
            .expand(&[], &mut okm)
            .unwrap();
        assert_eq!(
            hex::encode(okm),
            "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d\
             9d201395faa4b61a96c8"
        );
    }

    #[test]
    fn derivation_is_deterministic() {
        let a1 = subkeys(&MasterKey::from_bytes(MASTER_A));
        let a2 = subkeys(&MasterKey::from_bytes(MASTER_A));
        for (x, y) in a1.iter().zip(a2.iter()) {
            assert_eq!(x.as_slice(), y.as_slice());
        }
    }

    #[test]
    fn purposes_derive_independent_keys() {
        let [hmac, seed] = subkeys(&MasterKey::from_bytes(MASTER_A));
        assert_ne!(hmac.as_slice(), seed.as_slice());
        assert_ne!(hmac.as_slice(), &MASTER_A, "subkey must not equal master");
        assert_ne!(seed.as_slice(), &MASTER_A, "subkey must not equal master");
    }

    #[test]
    fn different_context_or_master_gives_different_key() {
        let master = MasterKey::from_bytes(MASTER_A);
        assert_ne!(
            master.derive(b"arktos/other/v1").as_slice(),
            master.derive(WALLET_SEED_INFO).as_slice()
        );
        assert_ne!(
            MasterKey::from_bytes(MASTER_B)
                .derive(WALLET_SEED_INFO)
                .as_slice(),
            master.derive(WALLET_SEED_INFO).as_slice()
        );
    }

    #[test]
    fn derived_subkeys_are_pinned() {
        // Changing labels or the KDF would make stored data unreadable.
        let [hmac, seed] = subkeys(&MasterKey::from_bytes(MASTER_A));
        let mut okm = [0u8; 32];
        Hkdf::<Sha256>::new(None, &MASTER_A)
            .expand(b"arktos/wallet-seed-encryption/v1", &mut okm)
            .unwrap();
        assert_eq!(seed.as_slice(), &okm);
        assert_eq!(hmac.len(), 32);
    }

    #[test]
    fn master_key_parses_base64_variants() {
        let standard = STANDARD.encode([0xfb; 32]);
        let url = URL_SAFE.encode([0xfb; 32]);
        let unpadded = standard.trim_end_matches('=').to_string();
        for encoded in [standard, url, unpadded] {
            let key = MasterKey::from_base64(&encoded).expect("valid key");
            assert_eq!(key.0.as_slice(), &[0xfb; 32]);
        }
    }

    #[test]
    fn master_key_rejects_bad_input() {
        assert_eq!(
            MasterKey::from_base64("not base64 !!").unwrap_err(),
            KeyError::InvalidEncoding
        );
        assert_eq!(
            MasterKey::from_base64(&STANDARD.encode([1u8; 16])).unwrap_err(),
            KeyError::InvalidLength
        );
        assert_eq!(
            MasterKey::from_base64("").unwrap_err(),
            KeyError::InvalidLength
        );
    }

    #[test]
    fn generated_master_keys_are_random_and_valid() {
        let a = generate_master_key_base64().unwrap();
        let b = generate_master_key_base64().unwrap();
        assert_ne!(a, b);
        MasterKey::from_base64(&a).expect("generated key parses");
    }

    #[test]
    fn api_key_hmac_uses_derived_key_not_master() {
        let master = MasterKey::from_bytes(MASTER_A);
        let keyring = Keyring::new(&master);
        let hash = keyring.api_keys.hmac.hash("some-api-key");
        assert_eq!(hash.len(), 64);
        assert_ne!(hash, hmac_sha256_hex(&MASTER_A, "some-api-key"));
    }

    #[test]
    fn debug_output_never_contains_key_material() {
        let keyring = Keyring::new(&MasterKey::from_bytes(MASTER_A));
        let rendered = format!("{keyring:?} {:?}", MasterKey::from_bytes(MASTER_A));
        assert!(rendered.contains("REDACTED"));
        assert!(!rendered.contains(&hex::encode(MASTER_A)));
        assert!(!rendered.contains("17, 17"), "no raw byte arrays");
    }
}
