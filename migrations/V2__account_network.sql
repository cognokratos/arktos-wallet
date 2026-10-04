-- V2: make the address network part of an account's identity.
--
-- Bitcoin test networks share coin type 1' but encode addresses differently
-- (tb1… vs bcrt1…), and BITCOIN_NETWORK can change between restarts, so the
-- network must be stored and included in the uniqueness key. Ethereum
-- addresses are identical on every EVM chain ID, so Ethereum accounts use the
-- single network value 'evm'; the configured chain ID is reported in API
-- responses, not stored.
--
-- SQLite cannot alter UNIQUE/CHECK constraints in place, so the table is
-- rebuilt. Existing rows were derived on Bitcoin mainnet (the only network
-- before V2). Ethereum addresses stay stored in canonical lowercase hex;
-- EIP-55 checksums are applied when they are returned.

CREATE TABLE accounts_v2 (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id        INTEGER NOT NULL REFERENCES wallets (id) ON DELETE RESTRICT,
    chain_type       TEXT    NOT NULL CHECK (chain_type IN ('Bitcoin', 'Ethereum')),
    network          TEXT    NOT NULL,
    -- Non-hardened BIP32 address index (last path component).
    account_index    INTEGER NOT NULL CHECK (account_index BETWEEN 0 AND 2147483647),
    derivation_path  TEXT    NOT NULL,
    public_key       TEXT    NOT NULL,
    address          TEXT    NOT NULL,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (wallet_id, chain_type, network, account_index),
    CHECK (
        (chain_type = 'Bitcoin' AND network = 'mainnet'
            AND derivation_path = 'm/86''/0''/0''/0/' || account_index) OR
        (chain_type = 'Bitcoin' AND network IN ('testnet', 'signet', 'regtest')
            AND derivation_path = 'm/86''/1''/0''/0/' || account_index) OR
        (chain_type = 'Ethereum' AND network = 'evm'
            AND derivation_path = 'm/44''/60''/0''/0/' || account_index
            AND address = lower(address))
    )
) STRICT;

INSERT INTO accounts_v2
    (id, wallet_id, chain_type, network, account_index, derivation_path, public_key, address, created_at)
SELECT id, wallet_id, chain_type,
       CASE chain_type WHEN 'Bitcoin' THEN 'mainnet' ELSE 'evm' END,
       account_index, derivation_path, public_key,
       CASE chain_type WHEN 'Ethereum' THEN lower(address) ELSE address END, created_at
FROM accounts;

DROP TABLE accounts;
ALTER TABLE accounts_v2 RENAME TO accounts;
