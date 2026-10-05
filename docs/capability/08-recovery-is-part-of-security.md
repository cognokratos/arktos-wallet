# C8 — Recovery is part of security

> **Security includes the ability of the legitimate owner to recover.**

**Question:** the server's disk dies. What exactly do you need to bring Arktos back on a clean machine with every wallet intact? And what happens when one of those things is missing?

**Prerequisite (optional):** durable state, crash and restart semantics in general are covered in the [sophos-agent runtime path](https://github.com/cognokratos/sophos-agent/blob/main/docs/RUNTIME-LEARNING-PATH.md). This lesson is about the cryptographic side: with encryption, a missing key is not a degraded mode. Whatever that key protected is unrecoverable from the encrypted data.

## Mental model

Every encryption layer adds something you must keep in order to read your own data. Arktos has two layers, so a restore depends on **three** things:

```text
database files          arktos.db (+ arktos.db-wal, arktos.db-shm while running)
+ DATABASE_KEY          opens the SQLCipher file
+ MASTER_KEY            decrypts recovery phrases, and verifies client API keys
```

There are also two non-secret requirements:

- **A compatible binary.** Migrations only move forward and run at startup. A binary *older* than the database's schema refuses to open it (`newer_schema_version_is_rejected` in [`tests/persistence_tests.rs`](../../tests/persistence_tests.rs)).
- **The same chain configuration** (`BITCOIN_NETWORK`). Nothing is lost if it differs: accounts are stored per network. But the server will serve and derive a different set of addresses.

`ADMIN_API_KEY` is **not** a recovery dependency. It is compared against the environment and is not bound to any stored data, so a new value works immediately.

## What each loss means

Every consequence below describes what can be recovered **from Arktos's own data**. "Lost from Arktos storage" is not the same as "provably lost everywhere": an operator may hold an independent backup of a recovery phrase, for example one exported deliberately with the operator tooling (`make decrypt`). The lesson is that Arktos's data alone no longer suffices.

| Lost | Consequence |
|---|---|
| Database files | Everything Arktos stored. Recovery phrases are random, not derived from any key, so no key can regenerate them |
| `DATABASE_KEY` | The file cannot be opened: `cannot read database: DATABASE_KEY is wrong or the file is not an Arktos database`. Everything inside is unreadable, including the ciphertexts that `MASTER_KEY` could have opened |
| `MASTER_KEY` | The database opens, but **every recovery phrase is undecryptable** and **every client API key stops verifying**, because the HMAC key is derived from `MASTER_KEY`. Arktos can no longer recover the private-key hierarchy from its stored data. If no independent backup of a recovery phrase exists, that wallet is not recoverable through Arktos, and any funds controlled solely by that phrase are effectively lost |

### The dangerous partial failure

The `MASTER_KEY` row has a trap in it. Suppose an operator "recovers" by starting the server with a **new** `MASTER_KEY` and the old database:

1. The server starts normally. `MASTER_KEY` is valid, and `DATABASE_KEY` opens the file.
2. Every agent gets `401`, because the old API-key hashes no longer match.
3. The admin re-issues keys with `POST /admin/api-keys/{id}/rotate`. Ownership is by key id, so the owners get their wallets back.
4. `get_*_address` for an **already-derived** index **succeeds**: it is served from the public `accounts` row without any decryption.
5. `get_*_address` for a **new** index fails with `internal error`.

Step 4 is the trap. The service keeps handing out deposit addresses for wallets whose private keys it can no longer recover. An agent will pass those addresses to payers. Unless an independent backup of the phrase exists somewhere, payments to them are effectively lost. (Arktos implements no signing today, so even a healthy instance cannot spend; the point is that a correct restore would let the owner recover the phrase and spend elsewhere, and this one cannot.) This exact sequence is pinned in a test:

```bash
cargo test --test secret_storage_tests losing_the_master_key_leaves_only_already_public_data_usable
```

What should an operator, or a future version of Arktos, do to make this failure loud? Arktos has no "key-check value" that would detect a wrong `MASTER_KEY` at startup. Consider what adding one would cost and what it would leak.

## Recovery combinations

Fill in this table *before* reading the answer below it.

| You hold | Open the database? | Decrypt phrases? | Verify existing client API keys? | Recoverable |
|---|---|---|---|---|
| Database only | | | | |
| `DATABASE_KEY` only | | | | |
| `MASTER_KEY` only | | | | |
| DB + `DATABASE_KEY` | | | | |
| DB + `MASTER_KEY` | | | | |
| DB + both keys | | | | |

<details>
<summary>Answer</summary>

| You hold | Open the database? | Decrypt phrases? | Verify existing client API keys? | Recoverable |
|---|---|---|---|---|
| Database only | no | no | no | nothing |
| `DATABASE_KEY` only | n/a | n/a | n/a | nothing: there is no data |
| `MASTER_KEY` only | n/a | n/a | n/a | nothing: phrases are random, not derived from `MASTER_KEY` |
| DB + `DATABASE_KEY` | yes | **no** | **no** | wallet names, owners, public accounts and addresses, i.e. metadata. **No recovery phrases**, so no access to funds through Arktos. Re-issued keys give access to [the trap above](#the-dangerous-partial-failure) |
| DB + `MASTER_KEY` | **no** | no, because the envelopes are inside the unreadable file | no | nothing |
| DB + both keys | yes | yes | yes | everything (with a compatible binary) |

The "DB + `MASTER_KEY`" row surprises people. Field encryption sits *inside* SQLCipher, so without `DATABASE_KEY` you cannot even reach the ciphertext that `MASTER_KEY` would open. The layers are independent for **confidentiality** and **jointly required** for **availability**.

Compare a different design in which recovery phrases are derived deterministically from `MASTER_KEY` and the wallet id. Then "`MASTER_KEY` only" would recover every wallet, and `MASTER_KEY` would also become a single secret whose theft compromises every wallet ever created, including wallets in backups you have deleted. Arktos chose random phrases. Each choice moves risk; neither removes it.

</details>

## Backups with SQLCipher and WAL

Arktos runs SQLite in WAL mode ([`src/database.rs`](../../src/database.rs)). While it runs, committed transactions can live in `arktos.db-wal` and not yet be in `arktos.db`, so **copying only `arktos.db` from a running server can lose committed wallets.** The supported approaches, from [Architecture — Files, Permissions and Backups](../architecture.md#files-permissions-and-backups):

| Method | When | Result |
|---|---|---|
| `VACUUM INTO '<path>'` from a keyed `sqlcipher` shell (`make sql`) | Online | One self-contained file, encrypted with the same `DATABASE_KEY`, **including the schema version** |
| Stop Arktos, then copy `arktos.db` together with any `-wal`/`-shm` files | Offline | Consistent file set |
| The shell's `.backup` | Not supported | Does not work for encrypted databases |
| `sqlcipher_export()` | Not supported | Loses `PRAGMA user_version`, so Arktos would reject the copy as unmanaged or outdated |

A backup is **still encrypted** with `DATABASE_KEY`, and the phrases inside it still need `MASTER_KEY`. A backup without its keys is not a backup.

## Lab

With the [lab server](../CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md#lab-setup) and a wallet with at least one derived address:

### Observe: an online backup

```bash
labsql "VACUUM INTO '$LAB/backup.db';"
ls -l "$LAB"
```

Note the backup's permissions. `VACUUM INTO` creates the file with your umask, often `0644`, until Arktos opens it and tightens it to `0600`. The contents are ciphertext, but treat the file as sensitive from the moment it exists.

### Restore on a "clean machine"

```bash
DATABASE_PATH="$LAB/backup.db" cargo run --quiet --bin arktos-wallet -- db-info
```

`db-info` prints diagnostics only: the SQLCipher version, the schema version and the pragmas. It prints no data and no secrets. Then simulate each loss:

```bash
DATABASE_PATH="$LAB/backup.db" DATABASE_KEY=wrong cargo run --quiet --bin arktos-wallet -- db-info
```

To observe the `MASTER_KEY` trap end to end, restart the lab server with `MASTER_KEY=$(make secret)` against the same database. Watch `mcp "$A" ping '{}'` fail with `401`, re-issue A's key through `/admin/api-keys/{id}/rotate`, and then request an index you derived earlier and an index you did not.

### Exercise: the recovery inventory

Write the document an on-call engineer would need at 3 a.m. to answer: **"What must I have to restore Arktos on a clean machine?"** For each item, record:

- what it is, and where the authoritative copy lives;
- who can access it, and who can access it *alone*;
- how you would detect that it is wrong *before* serving traffic (compare the trap);
- how old it may be. The database backup has a recovery point objective (RPO). The keys must never be "old", because there is exactly one valid value of each.

### Explain: should the keys live with the database backup?

| Option | Availability | Security |
|---|---|---|
| DB backup + `DATABASE_KEY` + `MASTER_KEY` in one bundle | Excellent: one restore artifact | **The bundle is full custody.** Both encryption layers add nothing against anyone who obtains it |
| DB backups in storage A; both keys together in a secret manager B | Good: two systems to restore from | Storage A alone yields nothing. Secret manager B alone yields nothing (no data) |
| DB in A; `DATABASE_KEY` in B; `MASTER_KEY` in C, held by different people | Weaker: three things to keep alive and in sync | Strongest: the people who run backups never hold `MASTER_KEY`, which is the [separate compromise domains](02-design-key-hierarchies.md#why-database_key-is-not-derived-from-master_key) of lesson 02 applied to operations |

Every extra separation reduces the number of people who can steal everything, and it increases the number of ways everything can be lost. Pick deliberately, write the choice down, and **rehearse the restore**.

## Rotation and migration today

| Secret | Rotation in Arktos | Notes |
|---|---|---|
| Client API key | `POST /admin/api-keys/{id}/rotate` | Cheap: API keys are verify-only, so a new random key just replaces the stored HMAC |
| `ADMIN_API_KEY` | Change the environment variable and restart | Not bound to stored data |
| `DATABASE_KEY` | Not provided by Arktos | SQLCipher has its own rekey mechanism. Validate any procedure against a `VACUUM INTO` copy first |
| `MASTER_KEY` | **Not supported** | Rotation would mean re-encrypting every envelope *and* re-issuing every client API key. See [Challenge 3](CHALLENGES.md#challenge-3--rotate-wallet-encryption-keys) |
| Schema | Automatic forward migrations at startup ([`src/database.rs`](../../src/database.rs)) | Take a backup before upgrading. An older binary will refuse the migrated file |

## Failure mode

- Strong encryption with keys stored "somewhere safe" that nobody has ever restored from.
- Backing up `arktos.db` from a running server without its WAL.
- One bundle containing the database and both keys, which turns two layers into zero.
- "Recovering" with a fresh key and a server that looks healthy.

## Takeaway

> Encryption without a recovery plan can turn a security control into permanent data loss.

## Related reference

- [Architecture — Files, Permissions and Backups](../architecture.md#files-permissions-and-backups)
- [Deployment Guide — Generating Secrets](../deployment-guide.md#generating-secrets), [Persistence](../deployment-guide.md#persistence)
- [Development Guide — Database Operations](../development-guide.md#database-operations)
- [Challenge 4 — HSM/KMS-backed keys](CHALLENGES.md#challenge-4--hsmkms-backed-keys)

Previous: [C7](07-design-least-capability-tools.md) · Next: [Secret lifecycle walkthrough](SECRET-LIFECYCLE-WALKTHROUGH.md)
