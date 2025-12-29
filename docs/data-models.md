# Data Models: Arktos Wallet

This document describes the data models used within the Arktos Wallet application, primarily focusing on how wallets and their associated accounts are structured and stored.

## Storage Mechanism

The Arktos Wallet server uses **SQLite** as its primary data storage. To ensure the security of sensitive information, such as private keys and passphrases, the SQLite database is encrypted using **SQLCipher**. This is facilitated by the `rusqlite` crate with the `bundled-sqlcipher-vendored-openssl` feature enabled.

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

Represents a single blockchain account within a wallet. Each account is associated with a specific private key.

| Field Name     | Data Type          | Description                                        |
| :------------- | :----------------- | :------------------------------------------------- |
| `ID`           | Incremented Integer| A unique, auto-incrementing identifier for the account within the context of its parent wallet. |
| `Private Key`  | Text (Encrypted)   | The private key for this blockchain account. This is derived from the wallet's passphrase and is stored encrypted. |

## Relationships

*   A `Wallet` can contain one or more `Account`s.
*   Each `Account` is uniquely linked to a single `Wallet`.

## Encryption

All sensitive data, specifically the `Passphrase` within the `Wallet` model and the `Private Key` within the `Account` model, are encrypted at rest using SQLCipher. This ensures that even if the database file is accessed directly, the contents remain protected. The server handles the encryption and decryption processes transparently to the API consumer.
