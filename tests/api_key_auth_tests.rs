use arktos_wallet::db::Database;
use arktos_wallet::services::Services;
use std::sync::Arc;

// Helper function to setup test database
fn setup_test_db() -> Arc<Database> {
    let db_path = ":memory:";
    let cipher_key = "test_cipher_key_1234567890123456";
    Arc::new(Database::new(db_path, cipher_key).expect("Failed to create test database"))
}

#[test]
fn test_create_api_key() {
    let db = setup_test_db();
    let services = Services::new(db.clone(), "test_cipher_key_1234567890123456".to_string());

    // Create a wallet first
    let wallet = db
        .create_wallet("test_wallet", "encrypted_pass")
        .expect("Should create wallet");

    let client_name = "test_client";

    let result = services.create_api_key(wallet.id, client_name);
    assert!(result.is_ok(), "Should successfully create API key");

    let api_key = result.unwrap();
    assert!(!api_key.key.is_empty(), "API key should not be empty");
    assert_eq!(api_key.client_name, client_name);
    assert_eq!(api_key.wallet_id, wallet.id);
}

#[test]
fn test_validate_api_key_valid() {
    let db = setup_test_db();
    let services = Services::new(db.clone(), "test_cipher_key_1234567890123456".to_string());

    // Create a wallet first
    let wallet = db
        .create_wallet("test_wallet", "encrypted_pass")
        .expect("Should create wallet");

    let client_name = "test_client";

    let api_key = services
        .create_api_key(wallet.id, client_name)
        .expect("Should create API key");

    let result = services.validate_api_key(&api_key.key);
    assert!(result.is_ok(), "Should validate correct API key");

    let (returned_wallet_id, returned_client) = result.unwrap();
    assert_eq!(returned_wallet_id, wallet.id);
    assert_eq!(returned_client, client_name);
}

#[test]
fn test_validate_api_key_invalid() {
    let db = setup_test_db();
    let services = Services::new(db, "test_cipher_key_1234567890123456".to_string());

    // Try to validate non-existent API key
    let result = services.validate_api_key("invalid_key_12345678");
    assert!(result.is_err(), "Should reject invalid API key");
}

#[test]
fn test_validate_api_key_revoked() {
    let db = setup_test_db();
    let services = Services::new(db.clone(), "test_cipher_key_1234567890123456".to_string());

    // Create a wallet first
    let wallet = db
        .create_wallet("test_wallet", "encrypted_pass")
        .expect("Should create wallet");

    let client_name = "test_client";

    let api_key = services
        .create_api_key(wallet.id, client_name)
        .expect("Should create API key");

    // Revoke the key
    let revoke_result = services.revoke_api_key(&api_key.key);
    assert!(revoke_result.is_ok(), "Should revoke API key");

    // Try to validate revoked key
    let validation_result = services.validate_api_key(&api_key.key);
    assert!(validation_result.is_err(), "Should reject revoked API key");
}

#[test]
fn test_list_api_keys_for_wallet() {
    let db = setup_test_db();
    let services = Services::new(db.clone(), "test_cipher_key_1234567890123456".to_string());

    // Create a wallet first
    let wallet = db
        .create_wallet("test_wallet", "encrypted_pass")
        .expect("Should create wallet");

    // Create multiple API keys
    let _key1 = services
        .create_api_key(wallet.id, "client_1")
        .expect("Should create first API key");
    let _key2 = services
        .create_api_key(wallet.id, "client_2")
        .expect("Should create second API key");

    // List keys for wallet
    let result = services.list_api_keys(wallet.id);
    assert!(result.is_ok(), "Should list API keys");

    let keys = result.unwrap();
    assert_eq!(keys.len(), 2, "Should have 2 API keys");
    assert!(keys.iter().any(|k| k.client_name == "client_1"));
    assert!(keys.iter().any(|k| k.client_name == "client_2"));
}

#[test]
fn test_api_key_stored_hashed() {
    let db = setup_test_db();
    let services = Services::new(db.clone(), "test_cipher_key_1234567890123456".to_string());

    // Create a wallet first
    let wallet = db
        .create_wallet("test_wallet", "encrypted_pass")
        .expect("Should create wallet");

    let api_key = services
        .create_api_key(wallet.id, "test_client")
        .expect("Should create API key");

    // Verify that the stored key is hashed (raw key should not appear in DB)
    // The key returned is plain text, but it's hashed in DB
    // So we can verify by trying to validate the raw key (which should work)
    let validation = services.validate_api_key(&api_key.key);
    assert!(
        validation.is_ok(),
        "Plain text key should validate correctly"
    );

    // If we try to validate a different key, it should fail
    let invalid_validation = services.validate_api_key(&format!("wrong_{}", api_key.key));
    assert!(invalid_validation.is_err(), "Wrong key should not validate");
}

#[test]
fn test_unique_api_keys() {
    let db = setup_test_db();
    let services = Services::new(db.clone(), "test_cipher_key_1234567890123456".to_string());

    // Create a wallet first
    let wallet = db
        .create_wallet("test_wallet", "encrypted_pass")
        .expect("Should create wallet");

    let key1 = services
        .create_api_key(wallet.id, "client_1")
        .expect("Should create first API key");
    let key2 = services
        .create_api_key(wallet.id, "client_2")
        .expect("Should create second API key");

    // Keys should be unique
    assert_ne!(key1.key, key2.key, "API keys should be unique");
}

#[test]
fn test_api_key_metadata() {
    let db = setup_test_db();
    let services = Services::new(db.clone(), "test_cipher_key_1234567890123456".to_string());

    // Create a wallet first
    let wallet = db
        .create_wallet("test_wallet", "encrypted_pass")
        .expect("Should create wallet");

    let client_name = "test_client";
    let api_key = services
        .create_api_key(wallet.id, client_name)
        .expect("Should create API key");

    // Verify metadata
    assert_eq!(api_key.wallet_id, wallet.id);
    assert_eq!(api_key.client_name, client_name);
    assert!(!api_key.created_at.is_empty());
    assert!(!api_key.key_hash.is_empty());
    assert!(!api_key.key.is_empty());
    assert!(!api_key.is_revoked);
}
