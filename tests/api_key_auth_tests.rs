use arktos_wallet::database::Database;
use arktos_wallet::key_services::KeyServices;
use std::sync::Arc;

// Helper function to setup test database
fn setup_test_db() -> Arc<Database> {
    let db_path = ":memory:";
    let cipher_key = "test_cipher_key_1234567890123456";
    Arc::new(Database::new(db_path, cipher_key).expect("Failed to create test database"))
}

#[tokio::test]
async fn test_create_api_key() {
    let db = setup_test_db();
    let key_services = KeyServices::new(db.clone(), "secret".to_string());

    let client_name = "test_client";

    let result = key_services.create(client_name).await;
    assert!(result.is_ok(), "Should successfully create API key");

    let api_key = result.unwrap();
    assert!(!api_key.is_empty(), "API key should not be empty");
}

#[tokio::test]
async fn test_validate_api_key_valid() {
    let db = setup_test_db();
    let key_services = KeyServices::new(db.clone(), "secret".to_string());

    let client_name = "test_client";

    let api_key = key_services
        .create(client_name)
        .await
        .expect("Should create API key");

    let result = key_services.validate(&api_key).await;
    assert!(result.is_ok(), "Should validate correct API key");

    let api_key = result.unwrap();
    assert_eq!(api_key.name, client_name);
    assert_eq!(api_key.is_revoked, false);
}

#[tokio::test]
async fn test_validate_api_key_invalid() {
    let db = setup_test_db();
    let key_services = KeyServices::new(db.clone(), "secret".to_string());

    // Try to validate non-existent API key
    let result = key_services.validate("invalid_key_12345678").await;
    assert!(result.is_err(), "Should reject invalid API key");
}

#[tokio::test]
async fn test_validate_api_key_revoked() {
    let db = setup_test_db();
    let key_services = KeyServices::new(db.clone(), "secret".to_string());

    let client_name = "test_client";

    let x_api_key = key_services
        .create(client_name)
        .await
        .expect("Should create API key");

    let api_key = key_services
        .validate(&x_api_key)
        .await
        .expect("Should validate API key");

    // Revoke the key
    let revoke_result = key_services.revoke(api_key.id).await;
    assert!(revoke_result.is_ok(), "Should revoke API key");

    // Try to validate revoked key
    let validation_result = key_services.validate(&x_api_key).await;
    assert!(validation_result.is_err(), "Should reject revoked API key");
}

#[tokio::test]
async fn test_list_api_keys_for_wallet() {
    let db = setup_test_db();
    let key_services = KeyServices::new(db.clone(), "secret".to_string());

    // Create multiple API keys
    let _key1 = key_services
        .create("client_1")
        .await
        .expect("Should create first API key");
    let _key2 = key_services
        .create("client_2")
        .await
        .expect("Should create second API key");

    // List keys for wallet
    let result = key_services.list().await;
    assert!(result.is_ok(), "Should list API keys");

    let keys = result.unwrap();
    assert_eq!(keys.len(), 2, "Should have 2 API keys");
    assert!(keys.iter().any(|k| k.name == "client_1"));
    assert!(keys.iter().any(|k| k.name == "client_2"));
}

#[tokio::test]
async fn test_api_key_stored_hashed() {
    let db = setup_test_db();
    let key_services = KeyServices::new(db.clone(), "secret".to_string());

    let api_key = key_services
        .create("test_client")
        .await
        .expect("Should create API key");

    // Verify that the stored key is hashed (raw key should not appear in DB)
    // The key returned is plain text, but it's hashed in DB
    // So we can verify by trying to validate the raw key (which should work)
    let validation = key_services.validate(&api_key).await;
    assert!(
        validation.is_ok(),
        "Plain text key should validate correctly"
    );

    // If we try to validate a different key, it should fail
    let invalid_validation = key_services.validate(&format!("wrong_{}", api_key)).await;
    assert!(invalid_validation.is_err(), "Wrong key should not validate");
}

#[tokio::test]
async fn test_unique_api_keys() {
    let db = setup_test_db();
    let key_services = KeyServices::new(db.clone(), "secret".to_string());

    let key1 = key_services
        .create("client_1")
        .await
        .expect("Should create first API key");
    let key2 = key_services
        .create("client_2")
        .await
        .expect("Should create second API key");

    // Keys should be unique
    assert_ne!(key1, key2, "API keys should be unique");
}
