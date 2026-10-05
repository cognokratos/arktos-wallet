//! End-to-end checks of how wallet secrets are stored and read back:
//! versioned envelopes, purpose-separated keys, derivation regression vectors
//! and the absence of secrets in service responses.

use arktos_wallet::api_key::ApiKey;
use arktos_wallet::crypto;
use arktos_wallet::database::Database;
use arktos_wallet::domain::{DerivationIndex, Network};
use arktos_wallet::key_services::KeyServices;
use arktos_wallet::keys::{Keyring, MasterKey};
use arktos_wallet::wallet_services::{
    CreateWalletRequest, GetBitcoinAddressRequest, GetEthereumAddressRequest, WalletServices,
};
use arktos_wallet::wallet_store::WalletStore;
use serde_json::Value;
use std::sync::Arc;
use tempfile::TempDir;

const MASTER: [u8; 32] = [0x42; 32];
/// Public BIP39 test mnemonic.
const TEST_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

struct Fixture {
    db: Arc<Database>,
    path: std::path::PathBuf,
    _dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("secrets.db");
        let db = Arc::new(Database::new(path.to_str().unwrap(), "db-key").unwrap());
        Self {
            db,
            path,
            _dir: dir,
        }
    }

    fn keyring(&self, master: [u8; 32]) -> Keyring {
        Keyring::new(&MasterKey::from_bytes(master))
    }

    fn wallet_services(&self, master: [u8; 32]) -> WalletServices {
        WalletServices::new(
            self.db.clone(),
            self.keyring(master).wallet,
            Default::default(),
        )
    }

    async fn api_key(&self) -> ApiKey {
        let keys = KeyServices::new(self.db.clone(), self.keyring(MASTER).api_keys);
        let raw = keys.create("owner").await.unwrap();
        keys.validate(&raw).await.unwrap()
    }

    fn store(&self) -> WalletStore {
        WalletStore::new(self.db.clone())
    }
}

fn btc(wallet: &str) -> GetBitcoinAddressRequest {
    GetBitcoinAddressRequest {
        wallet_name: wallet.into(),
        account_index: None,
    }
}

fn eth(wallet: &str) -> GetEthereumAddressRequest {
    GetEthereumAddressRequest {
        wallet_name: wallet.into(),
        account_index: None,
    }
}

/// Private key of m/44'/60'/0'/0/0 for TEST_MNEMONIC (publicly known).
const TEST_ETH_PRIVATE_KEY: &str =
    "1ab42cc412b618bdea3a599e3c9bae199ebf030895b039e9db1e30dafb12b727";

impl Fixture {
    /// Create a wallet holding TEST_MNEMONIC, sealed exactly as create_wallet does.
    async fn known_wallet(&self, owner: &ApiKey, name: &str) {
        let keys = self.keyring(MASTER).wallet;
        let sealed = crypto::seal(keys.seed.aead(), TEST_MNEMONIC.as_bytes()).unwrap();
        self.store()
            .create_wallet(owner.id, name, &sealed)
            .await
            .unwrap();
    }

    /// Account column names and every cell of every table, read straight
    /// from the SQLCipher file.
    fn dump_database(&self) -> (Vec<String>, String) {
        let conn = rusqlite::Connection::open(&self.path).unwrap();
        conn.pragma_update(None, "key", "db-key").unwrap();
        let columns: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('accounts')")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let mut cells = String::new();
        for table in ["api_keys", "wallets", "accounts"] {
            let mut stmt = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
            let width = stmt.column_count();
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                for i in 0..width {
                    let value: rusqlite::types::Value = row.get(i).unwrap();
                    cells.push_str(&format!("{value:?}\n"));
                }
            }
        }
        (columns, cells)
    }
}

#[tokio::test]
async fn new_wallets_store_an_encrypted_phrase_in_a_v1_envelope() {
    let fx = Fixture::new();
    let owner = fx.api_key().await;
    fx.wallet_services(MASTER)
        .create_wallet(
            &owner,
            CreateWalletRequest {
                wallet_name: "w".into(),
            },
        )
        .await
        .unwrap();

    let wallet = fx.store().get_wallet(owner.id, "w").await.unwrap().unwrap();
    let envelope: Value =
        serde_json::from_str(&wallet.encrypted_passphrase).expect("v1 JSON envelope");
    assert_eq!(envelope["v"], 1);
    assert_eq!(envelope["alg"], "A256GCM");

    let keys = fx.keyring(MASTER).wallet;
    let phrase = crypto::open(keys.seed.aead(), &wallet.encrypted_passphrase).unwrap();
    assert_eq!(std::str::from_utf8(&phrase).unwrap().split(' ').count(), 12);
}

#[tokio::test]
async fn accounts_persist_only_public_data() {
    let fx = Fixture::new();
    let owner = fx.api_key().await;
    fx.known_wallet(&owner, "known").await;
    let services = fx.wallet_services(MASTER);
    services
        .get_bitcoin_address(&owner, btc("known"))
        .await
        .unwrap();
    let eth_resp = services
        .get_ethereum_address(&owner, eth("known"))
        .await
        .unwrap();

    let wallet_id = fx
        .store()
        .get_wallet(owner.id, "known")
        .await
        .unwrap()
        .unwrap()
        .id;
    let account = fx
        .store()
        .get_account(
            wallet_id,
            Network::Ethereum,
            DerivationIndex::new(0).unwrap(),
        )
        .await
        .unwrap()
        .unwrap();
    // Stored canonically in lowercase; returned with the EIP-55 checksum.
    assert_eq!(account.address, eth_resp.address.to_lowercase());

    let (columns, cells) = fx.dump_database();
    assert!(
        columns.iter().all(|c| !c.contains("private")),
        "accounts columns: {columns:?}"
    );
    assert!(!cells.contains(TEST_ETH_PRIVATE_KEY), "private key stored");
    assert!(!cells.contains("abandon"), "plaintext mnemonic stored");
}

#[tokio::test]
async fn responses_never_contain_seed_or_private_key() {
    let fx = Fixture::new();
    let owner = fx.api_key().await;
    fx.known_wallet(&owner, "known").await;
    let services = fx.wallet_services(MASTER);
    let btc_resp = services
        .get_bitcoin_address(&owner, btc("known"))
        .await
        .unwrap();
    let eth_resp = services
        .get_ethereum_address(&owner, eth("known"))
        .await
        .unwrap();
    let wallet = fx
        .store()
        .get_wallet(owner.id, "known")
        .await
        .unwrap()
        .unwrap();

    let rendered = format!(
        "{} {} {btc_resp:?} {eth_resp:?} {wallet:?}",
        serde_json::to_string(&btc_resp).unwrap(),
        serde_json::to_string(&eth_resp).unwrap()
    );
    assert!(!rendered.contains(TEST_ETH_PRIVATE_KEY), "{rendered}");
    assert!(!rendered.contains("abandon"), "{rendered}");
    assert!(
        !rendered.contains(&wallet.encrypted_passphrase),
        "{rendered}"
    );
}

#[tokio::test]
async fn wallet_is_unreadable_with_a_different_master_key() {
    let fx = Fixture::new();
    let owner = fx.api_key().await;
    fx.wallet_services(MASTER)
        .create_wallet(
            &owner,
            CreateWalletRequest {
                wallet_name: "w".into(),
            },
        )
        .await
        .unwrap();

    let err = fx
        .wallet_services([0x99; 32])
        .get_bitcoin_address(&owner, btc("w"))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("failed to decrypt secret"), "{err}");
}

#[tokio::test]
async fn api_key_hashes_depend_on_the_master_key() {
    let fx = Fixture::new();
    let keys = KeyServices::new(fx.db.clone(), fx.keyring(MASTER).api_keys);
    let raw = keys.create("client").await.unwrap();

    assert!(keys.validate(&raw).await.is_ok());
    // The stored hash depends on the derived HMAC key, i.e. on the master key.
    let other = KeyServices::new(fx.db.clone(), fx.keyring([0x07; 32]).api_keys);
    assert!(other.validate(&raw).await.is_err());
}

#[tokio::test]
async fn known_mnemonic_derives_published_addresses_end_to_end() {
    let fx = Fixture::new();
    let owner = fx.api_key().await;
    fx.known_wallet(&owner, "known").await;

    let services = fx.wallet_services(MASTER);
    let btc_resp = services
        .get_bitcoin_address(&owner, btc("known"))
        .await
        .unwrap();
    let eth_resp = services
        .get_ethereum_address(&owner, eth("known"))
        .await
        .unwrap();

    // BIP86 m/86'/0'/0'/0/0 and BIP44 m/44'/60'/0'/0/0 reference addresses.
    assert_eq!(
        btc_resp.address,
        "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
    );
    assert_eq!(
        eth_resp.address,
        "0x9858EfFD232B4033E47d90003D41EC34EcaEda94"
    );
}

#[tokio::test]
async fn non_envelope_values_are_rejected() {
    let fx = Fixture::new();
    let owner = fx.api_key().await;
    // e.g. bare base64(nonce‖ciphertext) without the versioned envelope
    fx.store()
        .create_wallet(owner.id, "raw", "AAECAwQFBgcICQoLhHQcaKnSqbzcg0Vdr_0fa7Dx")
        .await
        .unwrap();

    let err = fx
        .wallet_services(MASTER)
        .get_bitcoin_address(&owner, btc("raw"))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("invalid encrypted-secret envelope"), "{err}");
}

#[tokio::test]
async fn losing_the_master_key_leaves_only_already_public_data_usable() {
    let fx = Fixture::new();
    let keys = KeyServices::new(fx.db.clone(), fx.keyring(MASTER).api_keys);
    let raw = keys.create("owner").await.unwrap();
    let owner = keys.validate(&raw).await.unwrap();
    let before = fx.wallet_services(MASTER);
    before
        .create_wallet(
            &owner,
            CreateWalletRequest {
                wallet_name: "w".into(),
            },
        )
        .await
        .unwrap();
    let derived = before.get_bitcoin_address(&owner, btc("w")).await.unwrap();

    // Restart with the same database but a replacement MASTER_KEY.
    const REPLACEMENT: [u8; 32] = [0x99; 32];
    let keys = KeyServices::new(fx.db.clone(), fx.keyring(REPLACEMENT).api_keys);
    assert!(
        keys.validate(&raw).await.is_err(),
        "old API keys stop verifying"
    );
    // An admin can re-issue the owner's key; ownership is by key id.
    let reissued = keys.rotate(owner.id).await.unwrap().unwrap();
    let owner = keys.validate(&reissued).await.unwrap();

    let after = fx.wallet_services(REPLACEMENT);
    // Stored accounts are public rows: they are served without decryption...
    assert_eq!(
        after.get_bitcoin_address(&owner, btc("w")).await.unwrap(),
        derived
    );
    // ...but nothing new can be derived: the recovery phrase is gone.
    let err = after
        .get_bitcoin_address(
            &owner,
            GetBitcoinAddressRequest {
                wallet_name: "w".into(),
                account_index: Some(1),
            },
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("failed to decrypt secret"), "{err}");
}
