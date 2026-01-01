use arktos_wallet::{
    db::Database,
    services::{CreateWalletRequest, Services},
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
