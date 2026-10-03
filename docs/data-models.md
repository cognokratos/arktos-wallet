# Data Models: Arktos Wallet

This document describes the data models used within the Arktos Wallet application, primarily focusing on how wallets and their associated accounts are structured and stored.

## Storage Mechanism

The Arktos Wallet server uses **SQLite** as its primary data storage. To ensure the security of sensitive information, such as recovery passphrases, the SQLite database is encrypted using **SQLCipher**. This is facilitated by the `rusqlite` crate with the `bundled-sqlcipher-vendored-openssl` feature enabled.

## Data Schemas

The core entities managed by the application are `Wallet` and `Account`.

### 1. Wallet

Represents a user's wallet, which can hold multiple blockchain accounts.

| Field Name     | Data Type          | Description                                        |
| :------------- | :----------------- | :------------------------------------------------- |
| `Name`         | String             | A user-defined name for the wallet.                |
| `Passphrase`   | Text (Encrypted)   | The recovery passphrase for the wallet. This is used internally to derive private keys and is stored encrypted. |
| `Accounts`     | List of `Account`  | A collection of blockchain accounts associated with this wallet. |

### 2. Account

Represents a single blockchain account within a wallet. Only public data is stored; the account's private key is re-derived from the wallet's passphrase when needed and never persisted.

| Field Name     | Data Type          | Description                                        |
| :------------- | :----------------- | :------------------------------------------------- |
| `ID`           | Incremented Integer| A unique, auto-incrementing identifier for the account within the context of its parent wallet. |
| `Account Index`| Integer            | BIP32 index used for derivation. |
| `Chain`        | Text               | `Bitcoin` or `Ethereum`. |
| `Public Key`   | Text               | Compressed public key (`0x`-hex). |
| `Address`      | Text               | Derived public address. |

## Relationships

*   A `Wallet` can contain one or more `Account`s.
*   Each `Account` is uniquely linked to a single `Wallet`.

## Encryption

Two independent layers protect sensitive data:

* **SQLCipher** (`DATABASE_KEY`) encrypts the entire database file.
* **Field encryption** (AES-256-GCM) additionally encrypts the `Passphrase` (`wallets.encrypted_passphrase`) with the wallet-seed key derived from `MASTER_KEY` via HKDF-SHA256, so someone who can read the opened database still sees only ciphertext. No private keys are stored.

Values are stored as a versioned envelope `{"v":1,"alg":"A256GCM","nonce":…,"ct":…}`. API keys are stored only as HMAC-SHA256 hashes (`api_keys.key_hash`). See [Architecture — Key Hierarchy & Secret Storage](./architecture.md#key-hierarchy--secret-storage).
