use arktos_wallet::models::ChainType::{Bitcoin, Ethereum};
use arktos_wallet::{
    db::Database,
    services::{CreateWalletRequest, GetAccountRequest, Services},
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

    let handler = Services::new(db, "test_cipher_key".to_string());

    let req = CreateWalletRequest {
        wallet_name: "IntegrationTestWallet".to_string(),
    };

    let response = handler.create_wallet(req).await;
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

    {
        let db = Arc::new(
            Database::new(&db_path, "test_cipher_key").expect("Failed to create database"),
        );

        let handler = Services::new(db, "test_cipher_key".to_string());

        let req = CreateWalletRequest {
            wallet_name: "PersistenceTest".to_string(),
        };

        let response = handler.create_wallet(req).await;
        assert!(response.is_ok());
    }

    // Recreate database and verify wallet persists
    {
        let db = Arc::new(
            Database::new(&db_path, "test_cipher_key").expect("Failed to create database"),
        );

        let wallet = db
            .get_wallet("PersistenceTest")
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

    let handler = Services::new(db.clone(), "test_cipher_key".to_string());

    // Step 1: Create wallet
    let wallet_req = CreateWalletRequest {
        wallet_name: "MultiAccountWallet".to_string(),
    };
    let wallet_resp = handler
        .create_wallet(wallet_req)
        .await
        .expect("Failed to create wallet");
    let wallet_id = wallet_resp.wallet_id;

    // Step 2: Create Bitcoin account (should derive keys and store in DB)
    let btc_account_req = GetAccountRequest {
        wallet_id,
        account_index: 0,
        chain_type: Bitcoin,
    };
    let btc_account = handler
        .create_or_get_account(btc_account_req)
        .await
        .expect("Failed to create Bitcoin account");

    assert_eq!(btc_account.wallet_id, wallet_id);
    assert_eq!(btc_account.account_index, 0);
    assert_eq!(btc_account.chain_type, Bitcoin);
    assert!(!btc_account.public_key.is_empty());

    // Step 3: Create Ethereum account
    let eth_account_req = GetAccountRequest {
        wallet_id,
        account_index: 0,
        chain_type: Ethereum,
    };
    let eth_account = handler
        .create_or_get_account(eth_account_req)
        .await
        .expect("Failed to create Ethereum account");

    assert_eq!(eth_account.wallet_id, wallet_id);
    assert_eq!(eth_account.account_index, 0);
    assert_eq!(eth_account.chain_type, Ethereum);
    assert!(!eth_account.public_key.is_empty());

    // Step 4: Verify Bitcoin and Ethereum accounts have different public keys
    assert_ne!(
        btc_account.public_key, eth_account.public_key,
        "Bitcoin and Ethereum accounts should have different keys"
    );

    // Step 5: Create another Bitcoin account with different index
    let btc_account_idx1_req = GetAccountRequest {
        wallet_id,
        account_index: 1,
        chain_type: Bitcoin,
    };
    let btc_account_idx1 = handler
        .create_or_get_account(btc_account_idx1_req)
        .await
        .expect("Failed to create second Bitcoin account");

    // Step 6: Verify different indices produce different keys
    assert_ne!(
        btc_account.public_key, btc_account_idx1.public_key,
        "Different account indices should have different keys"
    );

    // Step 7: Verify accounts are persisted in database
    let stored_account = db
        .get_account(wallet_id, 0, &Bitcoin)
        .expect("Failed to query account")
        .expect("Account should exist in database");

    assert_eq!(stored_account.wallet_id, wallet_id);
    assert_eq!(stored_account.account_index, 0);
    assert_eq!(stored_account.chain_type, Bitcoin);
    assert_eq!(stored_account.public_key, btc_account.public_key);
}
