use arktos_wallet::key_services::KeyServices;
use arktos_wallet::wallet::ChainType::Bitcoin;
use arktos_wallet::wallet_store::WalletStore;
use arktos_wallet::{
    database::Database,
    wallet_services::{CreateWalletRequest, GetBitcoinAddressRequest, WalletServices},
};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_create_wallet_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir
        .path()
        .join("test.db")
        .to_str()
        .unwrap()
        .to_string();

    let db =
        Arc::new(Database::new(&db_path, "test_cipher_key").expect("Failed to create database"));
    let key_services = KeyServices::new(db.clone(), "secret".to_string());
    let services = WalletServices::new(db, "test_cipher_key".to_string());

    let api_key = key_services.create("IntegrationTestAPIKey").await.unwrap();
    let api_key = key_services.validate(&api_key).await.unwrap();
    let req = CreateWalletRequest {
        wallet_name: "IntegrationTestWallet".to_string(),
    };

    let response = services.create_wallet(&api_key, req).await;
    assert!(response.is_ok(), "Wallet creation should succeed");

    let resp = response.unwrap();
    assert_eq!(resp.wallet_name, "IntegrationTestWallet");
}

#[tokio::test]
async fn test_wallet_persistence_after_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir
        .path()
        .join("test.db")
        .to_str()
        .unwrap()
        .to_string();

    let db =
        Arc::new(Database::new(&db_path, "test_cipher_key").expect("Failed to create database"));
    let key_services = KeyServices::new(db, "secret".to_string());
    let api_key = key_services.create("IntegrationTestAPIKey").await.unwrap();
    let api_key = key_services.validate(&api_key).await.unwrap();

    {
        let db = Arc::new(
            Database::new(&db_path, "test_cipher_key").expect("Failed to create database"),
        );
        let services = WalletServices::new(db, "test_cipher_key".to_string());

        let req = CreateWalletRequest {
            wallet_name: "PersistenceTest".to_string(),
        };

        let response = services.create_wallet(&api_key, req).await;
        assert!(response.is_ok());
    }

    // Recreate database and verify wallet persists
    {
        let db = Arc::new(
            Database::new(&db_path, "test_cipher_key").expect("Failed to create database"),
        );

        let wallet_store = WalletStore::new(db.clone());

        let wallet = wallet_store
            .get_wallet(api_key.id, "PersistenceTest")
            .expect("Should retrieve wallet")
            .expect("Wallet should exist");

        assert_eq!(wallet.name, "PersistenceTest");
    }
}

#[tokio::test]
async fn test_wallet_and_account_creation_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir
        .path()
        .join("test.db")
        .to_str()
        .unwrap()
        .to_string();

    let db =
        Arc::new(Database::new(&db_path, "test_cipher_key").expect("Failed to create database"));
    let key_services = KeyServices::new(db.clone(), "secret".to_string());
    let services = WalletServices::new(db.clone(), "test_cipher_key".to_string());

    let api_key = key_services.create("IntegrationTestAPIKey").await.unwrap();
    let api_key = key_services.validate(&api_key).await.unwrap();

    // Step 1: Create wallet
    let wallet_req = CreateWalletRequest {
        wallet_name: "MultiAccountWallet".to_string(),
    };
    let wallet_resp = services
        .create_wallet(&api_key, wallet_req)
        .await
        .expect("Failed to create wallet");
    let wallet_id = wallet_resp.wallet_id;
    let wallet_name = wallet_resp.wallet_name;

    // Step 2: Create Bitcoin account (should derive keys and store in DB)
    let btc_account_req = GetBitcoinAddressRequest {
        wallet_name: wallet_name.clone(),
        account_index: Some(0),
    };
    let btc_account = services
        .get_bitcoin_address(&api_key, btc_account_req)
        .await
        .expect("Failed to create Bitcoin account");

    assert_eq!(btc_account.wallet_name, wallet_name);
    assert_eq!(btc_account.account_index, 0);
    assert!(!btc_account.bitcoin_address.is_empty());

    // Step 3: Create Ethereum account
    // TODO: Implement Ethereum account creation in WalletServices

    // Step 4: Verify Bitcoin and Ethereum accounts have different public keys
    // TODO: Uncomment when Ethereum account creation is implemented
    // assert_ne!(
    //     btc_account.public_key, eth_account.public_key,
    //     "Bitcoin and Ethereum accounts should have different keys"
    // );

    // Step 5: Create another Bitcoin account with different index
    let btc_account_idx1_req = GetBitcoinAddressRequest {
        wallet_name: wallet_name.clone(),
        account_index: Some(1),
    };
    let btc_account_idx1 = services
        .get_bitcoin_address(&api_key, btc_account_idx1_req)
        .await
        .expect("Failed to create second Bitcoin account");

    // Step 6: Verify different indices produce different keys
    assert_ne!(
        btc_account.bitcoin_address, btc_account_idx1.bitcoin_address,
        "Different account indices should have different keys"
    );

    let wallet_store = WalletStore::new(db.clone());

    // Step 7: Verify accounts are persisted in database
    let stored_account = wallet_store
        .get_account(wallet_id, 0, &Bitcoin)
        .expect("Failed to query account")
        .expect("Account should exist in database");

    assert_eq!(stored_account.wallet_id, wallet_id);
    assert_eq!(stored_account.account_index, 0);
    assert_eq!(stored_account.chain_type, Bitcoin);
    assert_eq!(stored_account.address, btc_account.bitcoin_address);
}

#[tokio::test]
async fn test_get_bitcoin_address_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir
        .path()
        .join("test.db")
        .to_str()
        .unwrap()
        .to_string();

    let db =
        Arc::new(Database::new(&db_path, "test_cipher_key").expect("Failed to create database"));
    let key_services = KeyServices::new(db.clone(), "secret".to_string());
    let services = WalletServices::new(db, "test_cipher_key".to_string());

    let api_key = key_services.create("IntegrationTestAPIKey").await.unwrap();
    let api_key = key_services.validate(&api_key).await.unwrap();

    // Create wallet
    let wallet_req = CreateWalletRequest {
        wallet_name: "BitcoinAddressTestWallet".to_string(),
    };
    let wallet_resp = services
        .create_wallet(&api_key, wallet_req)
        .await
        .expect("Failed to create wallet");

    // Get Bitcoin address
    let addr_req = GetBitcoinAddressRequest {
        wallet_name: wallet_resp.wallet_name.clone(),
        account_index: Some(0),
    };
    let addr_resp = services
        .get_bitcoin_address(&api_key, addr_req)
        .await
        .expect("Failed to get Bitcoin address");

    assert_eq!(addr_resp.wallet_name, wallet_resp.wallet_name);
    assert_eq!(addr_resp.account_index, 0);
    assert!(
        addr_resp.bitcoin_address.starts_with("bc1"),
        "Should be valid Bitcoin address (Taproot)"
    );
    assert!(!addr_resp.bitcoin_address.is_empty());
}

#[tokio::test]
async fn test_get_bitcoin_address_different_indices_produce_different_addresses() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir
        .path()
        .join("test.db")
        .to_str()
        .unwrap()
        .to_string();

    let db =
        Arc::new(Database::new(&db_path, "test_cipher_key").expect("Failed to create database"));
    let key_services = KeyServices::new(db.clone(), "secret".to_string());
    let services = WalletServices::new(db, "test_cipher_key".to_string());

    let api_key = key_services.create("IntegrationTestAPIKey").await.unwrap();
    let api_key = key_services.validate(&api_key).await.unwrap();

    // Create wallet
    let wallet_req = CreateWalletRequest {
        wallet_name: "MultiAddressWallet".to_string(),
    };
    let wallet_resp = services
        .create_wallet(&api_key, wallet_req)
        .await
        .expect("Failed to create wallet");

    // Get Bitcoin address for account 0
    let addr_req_0 = GetBitcoinAddressRequest {
        wallet_name: wallet_resp.wallet_name.clone(),
        account_index: Some(0),
    };
    let addr_0 = services
        .get_bitcoin_address(&api_key, addr_req_0)
        .await
        .expect("Failed to get Bitcoin address for account 0");

    // Get Bitcoin address for account 1
    let addr_req_1 = GetBitcoinAddressRequest {
        wallet_name: wallet_resp.wallet_name.clone(),
        account_index: Some(1),
    };
    let addr_1 = services
        .get_bitcoin_address(&api_key, addr_req_1)
        .await
        .expect("Failed to get Bitcoin address for account 1");

    // Verify addresses are different
    assert_ne!(
        addr_0.bitcoin_address, addr_1.bitcoin_address,
        "Different account indices should produce different Bitcoin addresses"
    );
    assert!(addr_0.bitcoin_address.starts_with("bc1"));
    assert!(addr_1.bitcoin_address.starts_with("bc1"));
}

#[tokio::test]
async fn test_get_bitcoin_address_consistency() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir
        .path()
        .join("test.db")
        .to_str()
        .unwrap()
        .to_string();

    let db =
        Arc::new(Database::new(&db_path, "test_cipher_key").expect("Failed to create database"));
    let key_services = KeyServices::new(db.clone(), "secret".to_string());
    let services = WalletServices::new(db, "test_cipher_key".to_string());

    let api_key = key_services.create("IntegrationTestAPIKey").await.unwrap();
    let api_key = key_services.validate(&api_key).await.unwrap();

    // Create wallet
    let wallet_req = CreateWalletRequest {
        wallet_name: "ConsistencyTestWallet".to_string(),
    };
    let wallet_resp = services
        .create_wallet(&api_key, wallet_req)
        .await
        .expect("Failed to create wallet");

    // Get Bitcoin address first time
    let addr_req_1 = GetBitcoinAddressRequest {
        wallet_name: wallet_resp.wallet_name.clone(),
        account_index: Some(0),
    };
    let addr_1 = services
        .get_bitcoin_address(&api_key, addr_req_1)
        .await
        .expect("Failed to get Bitcoin address first time");

    // Get Bitcoin address second time
    let addr_req_2 = GetBitcoinAddressRequest {
        wallet_name: wallet_resp.wallet_name.clone(),
        account_index: Some(0),
    };
    let addr_2 = services
        .get_bitcoin_address(&api_key, addr_req_2)
        .await
        .expect("Failed to get Bitcoin address second time");

    // Verify same address is returned
    assert_eq!(
        addr_1.bitcoin_address, addr_2.bitcoin_address,
        "Same wallet and account index should produce the same Bitcoin address"
    );
}

#[tokio::test]
async fn test_get_bitcoin_address_with_default_account_index_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir
        .path()
        .join("test.db")
        .to_str()
        .unwrap()
        .to_string();

    let db =
        Arc::new(Database::new(&db_path, "test_cipher_key").expect("Failed to create database"));
    let key_services = KeyServices::new(db.clone(), "secret".to_string());
    let services = WalletServices::new(db, "test_cipher_key".to_string());

    let api_key = key_services.create("IntegrationTestAPIKey").await.unwrap();
    let api_key = key_services.validate(&api_key).await.unwrap();

    // Create wallet
    let wallet_req = CreateWalletRequest {
        wallet_name: "DefaultAccountWallet".to_string(),
    };
    let wallet_resp = services
        .create_wallet(&api_key, wallet_req)
        .await
        .expect("Failed to create wallet");

    // Get Bitcoin address without specifying account index
    let addr_req = GetBitcoinAddressRequest {
        wallet_name: wallet_resp.wallet_name.clone(),
        account_index: None,
    };
    let addr = services
        .get_bitcoin_address(&api_key, addr_req)
        .await
        .expect("Failed to get Bitcoin address");

    // Verify it defaults to account index 0
    assert_eq!(addr.account_index, 0);
    assert!(addr.bitcoin_address.starts_with("bc1"));
}

#[tokio::test]
async fn test_get_bitcoin_address_invalid_wallet_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir
        .path()
        .join("test.db")
        .to_str()
        .unwrap()
        .to_string();

    let db =
        Arc::new(Database::new(&db_path, "test_cipher_key").expect("Failed to create database"));
    let key_services = KeyServices::new(db.clone(), "secret".to_string());
    let services = WalletServices::new(db, "test_cipher_key".to_string());

    let api_key = key_services.create("IntegrationTestAPIKey").await.unwrap();
    let api_key = key_services.validate(&api_key).await.unwrap();

    // Try to get Bitcoin address for non-existent wallet
    let addr_req = GetBitcoinAddressRequest {
        wallet_name: "NonExistentWallet".to_string(),
        account_index: Some(0),
    };

    let result = services.get_bitcoin_address(&api_key, addr_req).await;
    assert!(
        result.is_err(),
        "Should return error for non-existent wallet"
    );
}

#[tokio::test]
async fn test_get_bitcoin_address_performance_requirement() {
    // NFR2: get_bitcoin_address shall respond within 100 milliseconds 95% of the time
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir
        .path()
        .join("test.db")
        .to_str()
        .unwrap()
        .to_string();

    let db =
        Arc::new(Database::new(&db_path, "test_cipher_key").expect("Failed to create database"));
    let key_services = KeyServices::new(db.clone(), "secret".to_string());
    let services = WalletServices::new(db, "test_cipher_key".to_string());

    let api_key = key_services.create("PerformanceTestAPIKey").await.unwrap();
    let api_key = key_services.validate(&api_key).await.unwrap();

    // Create wallet
    let wallet_req = CreateWalletRequest {
        wallet_name: "PerformanceTestWallet".to_string(),
    };
    let wallet_resp = services
        .create_wallet(&api_key, wallet_req)
        .await
        .expect("Failed to create wallet");

    // Pre-create the first address to test retrieval performance
    let addr_req = GetBitcoinAddressRequest {
        wallet_name: wallet_resp.wallet_name.clone(),
        account_index: Some(0),
    };
    services
        .get_bitcoin_address(&api_key, addr_req.clone())
        .await
        .expect("Failed to get Bitcoin address");

    // Run multiple iterations to measure performance
    let num_iterations = 20;
    let mut response_times = Vec::new();

    for i in 1..num_iterations {
        let addr_req = GetBitcoinAddressRequest {
            wallet_name: wallet_resp.wallet_name.clone(),
            account_index: Some(i as u32),
        };

        let start = std::time::Instant::now();
        services
            .get_bitcoin_address(&api_key, addr_req)
            .await
            .expect("Failed to get Bitcoin address");
        let elapsed = start.elapsed();

        response_times.push(elapsed.as_millis());
    }

    // Calculate statistics
    response_times.sort();
    let p95_index = ((response_times.len() as f32) * 0.95) as usize;
    let p95_time = response_times[p95_index];

    println!(
        "Performance: {} iterations, p95={} ms, max={} ms",
        response_times.len(),
        p95_time,
        response_times.iter().max().unwrap()
    );

    // NFR2 requirement: 95th percentile should be under 100ms
    assert!(
        p95_time < 100,
        "95th percentile response time should be under 100ms (was {}ms)",
        p95_time
    );
}
