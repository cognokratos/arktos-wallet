//! Database-level tests: migrations, SQLCipher, configuration, constraints,
//! transactions, ownership isolation and concurrency.

use arktos_wallet::api_key::ApiKey;
use arktos_wallet::database::{Database, StoreError};
use arktos_wallet::domain::{BitcoinNetwork, DerivationIndex, Network};
use arktos_wallet::error::AppError;
use arktos_wallet::key_services::KeyServices;
use arktos_wallet::key_store::KeyStore;
use arktos_wallet::keys::{Keyring, MasterKey};
use arktos_wallet::wallet_services::{
    CreateWalletRequest, GetBitcoinAddressRequest, WalletServices,
};
use arktos_wallet::wallet_store::{NewAccount, WalletStore};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

const DB_KEY: &str = "persistence-test-key";

fn keyring() -> Keyring {
    Keyring::new(&MasterKey::from_bytes([0x42; 32]))
}

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
    }

    fn path(&self) -> PathBuf {
        self.dir.path().join("arktos.db")
    }

    fn open(&self) -> Arc<Database> {
        Arc::new(Database::new(self.path().to_str().unwrap(), DB_KEY).expect("open database"))
    }

    /// A second, raw connection to the same file (as an operator shell would).
    fn raw(&self) -> Connection {
        raw_connection(&self.path(), DB_KEY)
    }
}

fn raw_connection(path: &Path, key: &str) -> Connection {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "key", key).unwrap();
    conn
}

async fn owner(db: &Arc<Database>, name: &str) -> i64 {
    KeyStore::new(db.clone())
        .create_api_key(name, &format!("{:0>64}", hex::encode(name)))
        .await
        .unwrap()
}

const BTC: Network = Network::Bitcoin(BitcoinNetwork::Mainnet);

fn idx(i: u32) -> DerivationIndex {
    DerivationIndex::new(i).unwrap()
}

fn account(wallet_id: i64, network: Network, index: u32) -> NewAccount {
    NewAccount {
        wallet_id,
        derivation_path: network.derivation_path(idx(index)),
        network,
        account_index: idx(index),
        public_key: format!("0xpub{index}"),
        address: format!("addr{index}"),
    }
}

fn columns(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

// ---------------------------------------------------------------------------
// Migrations and configuration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn empty_database_is_migrated_and_configured() {
    let fx = Fixture::new();
    let db = fx.open();

    let info = db.info().await.unwrap();
    assert_eq!(info.schema_version, 2);
    assert_eq!(info.latest_schema_version, 2);
    assert!(info.foreign_keys, "foreign keys must be enforced");
    assert_eq!(info.journal_mode, "wal");
    assert_eq!(info.synchronous, 2, "synchronous = FULL");
    assert_eq!(info.busy_timeout_ms, 5000);
    assert!(!info.cipher_version.is_empty(), "SQLCipher must be linked");

    let conn = fx.raw();
    assert_eq!(
        columns(&conn, "accounts"),
        [
            "id",
            "wallet_id",
            "chain_type",
            "network",
            "account_index",
            "derivation_path",
            "public_key",
            "address",
            "created_at"
        ]
    );
    assert!(columns(&conn, "wallets").contains(&"encrypted_passphrase".to_string()));
    assert!(columns(&conn, "api_keys").contains(&"key_hash".to_string()));
}

#[tokio::test]
async fn reopening_is_idempotent_and_keeps_data() {
    let fx = Fixture::new();
    let db = fx.open();
    let key_id = owner(&db, "a").await;
    WalletStore::new(db.clone())
        .create_wallet(key_id, "main", "{\"v\":1}")
        .await
        .unwrap();
    drop(db);

    for _ in 0..2 {
        let db = fx.open();
        assert_eq!(db.info().await.unwrap().schema_version, 2);
        assert!(
            WalletStore::new(db)
                .get_wallet(key_id, "main")
                .await
                .unwrap()
                .is_some()
        );
    }
}

#[test]
fn pre_migration_schema_is_rejected() {
    let fx = Fixture::new();
    // Tables created without the migration system (user_version = 0).
    fx.raw()
        .execute_batch("CREATE TABLE wallets (id INTEGER PRIMARY KEY);")
        .unwrap();
    let err = Database::new(fx.path().to_str().unwrap(), DB_KEY)
        .err()
        .unwrap();
    assert_eq!(err, StoreError::UnmanagedSchema);
}

#[test]
fn newer_schema_version_is_rejected() {
    let fx = Fixture::new();
    drop(fx.open());
    fx.raw().pragma_update(None, "user_version", 99).unwrap();
    let err = Database::new(fx.path().to_str().unwrap(), DB_KEY)
        .err()
        .unwrap();
    assert!(matches!(err, StoreError::Migration(_)), "{err:?}");
}

#[test]
fn missing_parent_directories_are_created() {
    let fx = Fixture::new();
    let nested = fx.dir.path().join("a/b/arktos.db");
    Database::new(nested.to_str().unwrap(), DB_KEY).expect("open nested path");
    assert!(nested.exists());
}

#[cfg(unix)]
#[test]
fn database_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let fx = Fixture::new();
    drop(fx.open());
    let mode = std::fs::metadata(fx.path()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn unwritable_location_gives_a_clear_error() {
    let fx = Fixture::new();
    let blocker = fx.dir.path().join("file");
    std::fs::write(&blocker, b"not a directory").unwrap();
    let err = Database::new(blocker.join("arktos.db").to_str().unwrap(), DB_KEY)
        .err()
        .unwrap();
    assert!(matches!(err, StoreError::Unavailable(_)), "{err:?}");
    assert!(err.to_string().contains("file"), "path is named: {err}");
}

// ---------------------------------------------------------------------------
// SQLCipher
// ---------------------------------------------------------------------------

#[test]
fn database_file_is_encrypted() {
    let fx = Fixture::new();
    drop(fx.open());
    let header = std::fs::read(fx.path()).unwrap();
    assert!(
        !header.starts_with(b"SQLite format 3\0"),
        "file must not be plaintext SQLite"
    );
}

#[test]
fn wrong_key_is_detected() {
    let fx = Fixture::new();
    drop(fx.open());
    let err = Database::new(fx.path().to_str().unwrap(), "another-key")
        .err()
        .unwrap();
    assert_eq!(err, StoreError::WrongKeyOrNotADatabase);
    assert!(!err.to_string().contains(DB_KEY));
}

#[test]
fn keys_with_sql_metacharacters_are_handled_safely() {
    let fx = Fixture::new();
    let key = "it's'; DROP TABLE wallets; --\"";
    drop(Database::new(fx.path().to_str().unwrap(), key).expect("quoted key works"));
    Database::new(fx.path().to_str().unwrap(), key).expect("same key reopens");
    assert_eq!(
        Database::new(fx.path().to_str().unwrap(), "it's").err(),
        Some(StoreError::WrongKeyOrNotADatabase),
        "the key is not truncated at the quote"
    );
}

// ---------------------------------------------------------------------------
// Constraints
// ---------------------------------------------------------------------------

#[tokio::test]
async fn foreign_keys_are_enforced() {
    let fx = Fixture::new();
    let db = fx.open();
    let wallets = WalletStore::new(db.clone());

    assert_eq!(
        wallets.create_wallet(999, "orphan", "x").await.unwrap_err(),
        StoreError::ForeignKeyViolation
    );
    assert_eq!(
        wallets
            .insert_account(account(999, BTC, 0))
            .await
            .unwrap_err(),
        StoreError::ForeignKeyViolation
    );
}

#[tokio::test]
async fn wallet_names_are_unique_per_owner() {
    let fx = Fixture::new();
    let db = fx.open();
    let (a, b) = (owner(&db, "a").await, owner(&db, "b").await);
    let wallets = WalletStore::new(db);

    wallets.create_wallet(a, "main", "x").await.unwrap();
    assert_eq!(
        wallets.create_wallet(a, "main", "y").await.unwrap_err(),
        StoreError::AlreadyExists
    );
    wallets
        .create_wallet(b, "main", "z")
        .await
        .expect("another owner may reuse the name");
}

#[tokio::test]
async fn api_key_hashes_are_unique() {
    let fx = Fixture::new();
    let keys = KeyStore::new(fx.open());
    let hash = "a".repeat(64);
    keys.create_api_key("one", &hash).await.unwrap();
    assert_eq!(
        keys.create_api_key("two", &hash).await.unwrap_err(),
        StoreError::AlreadyExists
    );
}

#[tokio::test]
async fn accounts_are_unique_and_insert_is_idempotent() {
    let fx = Fixture::new();
    let db = fx.open();
    let key_id = owner(&db, "a").await;
    let wallets = WalletStore::new(db);
    let wallet = wallets.create_wallet(key_id, "w", "x").await.unwrap();

    let first = wallets
        .insert_account(account(wallet.id, BTC, 0))
        .await
        .unwrap();
    let again = wallets
        .insert_account(account(wallet.id, BTC, 0))
        .await
        .unwrap();
    assert_eq!(first.id, again.id, "same row is returned");
    assert_eq!(first.derivation_path, "m/86'/0'/0'/0/0");

    // Same wallet + chain + index with different public data is never stored.
    let mut conflicting = account(wallet.id, BTC, 0);
    conflicting.address = "other".into();
    assert!(matches!(
        wallets.insert_account(conflicting).await.unwrap_err(),
        StoreError::CorruptData(_)
    ));

    // Other chain / index are separate accounts.
    let eth = wallets
        .insert_account(account(wallet.id, Network::Ethereum, 0))
        .await
        .unwrap();
    assert_ne!(eth.id, first.id);
    assert_eq!(eth.derivation_path, "m/44'/60'/0'/0/0");
}

#[tokio::test]
async fn check_constraints_reject_inconsistent_rows() {
    let fx = Fixture::new();
    let db = fx.open();
    let key_id = owner(&db, "a").await;
    let wallets = WalletStore::new(db.clone());
    let wallet = wallets.create_wallet(key_id, "w", "x").await.unwrap();

    let mut wrong_path = account(wallet.id, BTC, 1);
    wrong_path.derivation_path = Network::Ethereum.derivation_path(idx(1));
    assert!(matches!(
        wallets.insert_account(wrong_path).await.unwrap_err(),
        StoreError::ConstraintViolation(_)
    ));

    // Testnet accounts must use coin type 1'.
    let mut wrong_coin = account(wallet.id, Network::Bitcoin(BitcoinNetwork::Testnet), 1);
    wrong_coin.derivation_path = BTC.derivation_path(idx(1));
    assert!(matches!(
        wallets.insert_account(wrong_coin).await.unwrap_err(),
        StoreError::ConstraintViolation(_)
    ));

    // Ethereum addresses are stored in canonical lowercase.
    let mut mixed_case = account(wallet.id, Network::Ethereum, 2);
    mixed_case.address = "0xAbC".into();
    assert!(matches!(
        wallets.insert_account(mixed_case).await.unwrap_err(),
        StoreError::ConstraintViolation(_)
    ));

    // Hardened indices cannot be built as a DerivationIndex; the schema
    // rejects them too.
    let wallet_id = wallet.id;
    let hardened = db
        .write(move |tx| {
            tx.execute(
                "INSERT INTO accounts (wallet_id, chain_type, network, account_index, derivation_path, public_key, address)
                 VALUES (?1, 'Bitcoin', 'mainnet', 2147483648, 'm/86''/0''/0''/0/2147483648', 'pk', 'addr')",
                [wallet_id],
            )?;
            Ok(())
        })
        .await;
    assert!(matches!(hardened, Err(StoreError::ConstraintViolation(_))));

    assert!(matches!(
        wallets.create_wallet(key_id, "", "x").await.unwrap_err(),
        StoreError::ConstraintViolation(_)
    ));
}

#[tokio::test]
async fn accounts_are_isolated_by_network() {
    let fx = Fixture::new();
    let db = fx.open();
    let key_id = owner(&db, "a").await;
    let wallets = WalletStore::new(db);
    let wallet = wallets.create_wallet(key_id, "w", "x").await.unwrap();

    let testnet = Network::Bitcoin(BitcoinNetwork::Testnet);
    let regtest = Network::Bitcoin(BitcoinNetwork::Regtest);
    let main_row = wallets
        .insert_account(account(wallet.id, BTC, 0))
        .await
        .unwrap();
    assert_eq!(
        wallets
            .get_account(wallet.id, testnet, idx(0))
            .await
            .unwrap(),
        None,
        "a mainnet account is never returned for testnet"
    );
    let test_row = wallets
        .insert_account(account(wallet.id, testnet, 0))
        .await
        .unwrap();
    let reg_row = wallets
        .insert_account(account(wallet.id, regtest, 0))
        .await
        .unwrap();
    assert_ne!(main_row.id, test_row.id);
    assert_ne!(test_row.id, reg_row.id);
    assert_eq!(test_row.network, testnet);
    assert_eq!(test_row.derivation_path, "m/86'/1'/0'/0/0");
    assert_eq!(
        wallets.get_account(wallet.id, BTC, idx(0)).await.unwrap(),
        Some(main_row)
    );
}

#[tokio::test]
async fn v1_database_is_upgraded_to_v2() {
    let fx = Fixture::new();
    // Build a schema-version-1 database exactly as the previous release did.
    let raw = fx.raw();
    raw.execute_batch(include_str!("../migrations/V1__initial_schema.sql"))
        .unwrap();
    raw.pragma_update(None, "user_version", 1).unwrap();
    raw.execute_batch(&format!(
        "INSERT INTO api_keys (id, key_hash, key_name) VALUES (1, '{}', 'old');
         INSERT INTO wallets (id, key_id, name, encrypted_passphrase) VALUES (1, 1, 'w', 'x');
         INSERT INTO accounts (id, wallet_id, chain_type, account_index, derivation_path, public_key, address)
           VALUES (10, 1, 'Bitcoin', 0, 'm/86''/0''/0''/0/0', '0xpk', 'bc1pold'),
                  (11, 1, 'Ethereum', 0, 'm/44''/60''/0''/0/0', '0xpk', '0xabcdef');",
        "d".repeat(64)
    ))
    .unwrap();
    drop(raw);

    let db = fx.open();
    assert_eq!(db.info().await.unwrap().schema_version, 2);
    let wallets = WalletStore::new(db);
    let btc = wallets.get_account(1, BTC, idx(0)).await.unwrap().unwrap();
    assert_eq!((btc.id, btc.address.as_str()), (10, "bc1pold"));
    let eth = wallets
        .get_account(1, Network::Ethereum, idx(0))
        .await
        .unwrap()
        .unwrap();
    assert_eq!((eth.id, eth.address.as_str()), (11, "0xabcdef"));
    assert!(
        wallets
            .get_account(1, Network::Bitcoin(BitcoinNetwork::Signet), idx(0))
            .await
            .unwrap()
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// Transactions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn failed_write_transaction_leaves_no_partial_state() {
    let fx = Fixture::new();
    let db = fx.open();

    let result = db
        .write(|tx| {
            tx.execute(
                "INSERT INTO api_keys (key_hash, key_name) VALUES (?1, 'partial')",
                [&"b".repeat(64)],
            )?;
            // Second step fails: wallet references a missing API key.
            tx.execute(
                "INSERT INTO wallets (key_id, name, encrypted_passphrase) VALUES (12345, 'w', 'x')",
                [],
            )?;
            Ok(())
        })
        .await;
    assert_eq!(result, Err(StoreError::ForeignKeyViolation));

    let keys = KeyStore::new(db).list_api_keys().await.unwrap();
    assert!(
        keys.is_empty(),
        "first insert must be rolled back: {keys:?}"
    );
}

// ---------------------------------------------------------------------------
// Ownership isolation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wallets_are_isolated_by_owner() {
    let fx = Fixture::new();
    let db = fx.open();
    let (a, b) = (owner(&db, "a").await, owner(&db, "b").await);
    let wallets = WalletStore::new(db);
    let wa = wallets.create_wallet(a, "main", "secret-a").await.unwrap();
    let wb = wallets.create_wallet(b, "main", "secret-b").await.unwrap();

    let got = wallets.get_wallet(a, "main").await.unwrap().unwrap();
    assert_eq!(
        (got.id, got.encrypted_passphrase.as_str()),
        (wa.id, "secret-a")
    );
    assert_eq!(wallets.get_wallet_by_id(a, wb.id).await.unwrap(), None);
    assert_eq!(wallets.get_wallet_by_id(b, wa.id).await.unwrap(), None);
    let listed: Vec<i64> = wallets
        .list_wallets(a)
        .await
        .unwrap()
        .iter()
        .map(|w| w.id)
        .collect();
    assert_eq!(listed, [wa.id]);
}

// ---------------------------------------------------------------------------
// Timestamps
// ---------------------------------------------------------------------------

#[tokio::test]
async fn timestamps_come_from_the_database_in_utc() {
    let fx = Fixture::new();
    let db = fx.open();
    let key_id = owner(&db, "a").await;
    let wallets = WalletStore::new(db);
    let created = wallets.create_wallet(key_id, "w", "x").await.unwrap();
    let read = wallets.get_wallet(key_id, "w").await.unwrap().unwrap();

    assert_eq!(
        created.created_at, read.created_at,
        "returned value is authoritative"
    );
    let ts = created.created_at.as_bytes();
    assert_eq!(ts.len(), 24, "{}", created.created_at);
    assert_eq!(
        (ts[10], ts[19], ts[23]),
        (b'T', b'.', b'Z'),
        "{}",
        created.created_at
    );
}

// ---------------------------------------------------------------------------
// Concurrency
// ---------------------------------------------------------------------------

fn services(db: &Arc<Database>) -> (Arc<WalletServices>, Arc<KeyServices>) {
    (
        Arc::new(WalletServices::new(
            db.clone(),
            keyring().wallet,
            Default::default(),
        )),
        Arc::new(KeyServices::new(db.clone(), keyring().api_keys)),
    )
}

async fn api_key(keys: &KeyServices, name: &str) -> ApiKey {
    let raw = keys.create(name).await.unwrap();
    keys.validate(&raw).await.unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_requests_respect_constraints_without_deadlock() {
    let fx = Fixture::new();
    let db = fx.open();
    let (wallets, keys) = services(&db);
    let owner = api_key(&keys, "owner").await;

    let run = async {
        // 16 racing creations of the same wallet: exactly one wins.
        let creations: Vec<_> = (0..16)
            .map(|_| {
                let (wallets, owner) = (wallets.clone(), owner.clone());
                tokio::spawn(async move {
                    wallets
                        .create_wallet(
                            &owner,
                            CreateWalletRequest {
                                wallet_name: "race".into(),
                            },
                        )
                        .await
                })
            })
            .collect();
        let mut created = 0;
        for task in creations {
            match task.await.unwrap() {
                Ok(_) => created += 1,
                Err(AppError::WalletAlreadyExists { .. }) => {}
                Err(other) => panic!("unexpected error: {other:?}"),
            }
        }
        assert_eq!(created, 1);

        // 32 racing first derivations of the same account, mixed with reads
        // and writes on other wallets.
        let mut tasks = Vec::new();
        for i in 0..32 {
            let (wallets, keys, owner) = (wallets.clone(), keys.clone(), owner.clone());
            tasks.push(tokio::spawn(async move {
                if i % 4 == 0 {
                    let other = api_key(&keys, &format!("k{i}")).await;
                    wallets
                        .create_wallet(
                            &other,
                            CreateWalletRequest {
                                wallet_name: "race".into(),
                            },
                        )
                        .await
                        .unwrap();
                }
                wallets
                    .get_bitcoin_address(
                        &owner,
                        GetBitcoinAddressRequest {
                            wallet_name: "race".into(),
                            account_index: Some(0),
                        },
                    )
                    .await
                    .unwrap()
                    .address
            }));
        }
        let mut addresses = Vec::new();
        for task in tasks {
            addresses.push(task.await.unwrap());
        }
        addresses.dedup();
        assert_eq!(addresses.len(), 1, "all requests see the same account");
    };
    tokio::time::timeout(Duration::from_secs(60), run)
        .await
        .expect("no deadlock");

    let rows: i64 = db
        .read(|conn| Ok(conn.query_row("SELECT count(*) FROM accounts", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(rows, 1, "exactly one account row");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writes_wait_for_external_locks_within_busy_timeout() {
    let fx = Fixture::new();
    let db = fx.open();

    // An external connection (e.g. an operator shell) holds the write lock.
    let path = fx.path();
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let conn = raw_connection(&path, DB_KEY);
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        locked_tx.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        conn.execute_batch("COMMIT").unwrap();
    });
    locked_rx.recv().unwrap();

    // Reads are not blocked in WAL mode...
    db.ping()
        .await
        .expect("readers proceed during a write lock");
    // ...and the write waits (busy_timeout) instead of failing immediately.
    let started = std::time::Instant::now();
    KeyStore::new(db)
        .create_api_key("after-lock", &"c".repeat(64))
        .await
        .expect("write succeeds once the lock is released");
    assert!(started.elapsed() >= Duration::from_millis(300));
    holder.join().unwrap();
}
