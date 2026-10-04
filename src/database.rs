//! SQLCipher database: connection ownership, configuration, migrations and
//! the blocking boundary.
//!
//! ```text
//! async code ──► Database::read / Database::write
//!                   │  tokio::task::spawn_blocking
//!                   ▼
//!               one rusqlite::Connection (SQLCipher) behind a Mutex
//! ```
//!
//! Arktos uses exactly one connection. All database work runs on Tokio's
//! blocking thread pool, never on async worker threads, and the mutex is only
//! held inside those blocking closures (never across an `.await`). Writes run
//! in `BEGIN IMMEDIATE` transactions. One connection serializes all access,
//! which is predictable and more than fast enough for a wallet service.

use rusqlite::{Connection, ErrorCode, OpenFlags, TransactionBehavior, ffi};
use rusqlite_migration::{M, Migrations};
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// Versioned schema migrations, applied in order at startup.
const MIGRATIONS: &[M<'static>] = &[
    M::up(include_str!("../migrations/V1__initial_schema.sql")),
    M::up(include_str!("../migrations/V2__account_network.sql")).foreign_key_check(),
];

/// How long a statement waits for a lock held by another connection (e.g. an
/// operator's `sqlcipher` shell or a backup) before failing with "busy".
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

const IN_MEMORY: &str = ":memory:";

/// Controlled persistence errors. Messages never contain keys or row data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// A UNIQUE / PRIMARY KEY constraint was violated.
    AlreadyExists,
    /// A FOREIGN KEY constraint was violated.
    ForeignKeyViolation,
    /// Another constraint (CHECK, NOT NULL) was violated.
    ConstraintViolation(String),
    /// The database is busy, locked, read-only or otherwise inaccessible.
    Unavailable(String),
    /// The file is not a database, or `DATABASE_KEY` is wrong.
    WrongKeyOrNotADatabase,
    /// The linked SQLite library has no SQLCipher support.
    NotSqlCipher,
    /// Tables exist but were not created by the migration system.
    UnmanagedSchema,
    Migration(String),
    /// Stored data cannot be interpreted (e.g. unknown enum value).
    CorruptData(String),
    Internal(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::AlreadyExists => write!(f, "record already exists"),
            StoreError::ForeignKeyViolation => write!(f, "referenced record does not exist"),
            StoreError::ConstraintViolation(m) => write!(f, "constraint violation: {m}"),
            StoreError::Unavailable(m) => write!(f, "database unavailable: {m}"),
            StoreError::WrongKeyOrNotADatabase => write!(
                f,
                "cannot read database: DATABASE_KEY is wrong or the file is not an Arktos database"
            ),
            StoreError::NotSqlCipher => write!(f, "SQLite library was built without SQLCipher"),
            StoreError::UnmanagedSchema => write!(
                f,
                "database was created by a pre-migration version of Arktos; \
                 start from a new database file"
            ),
            StoreError::Migration(m) => write!(f, "database migration failed: {m}"),
            StoreError::CorruptData(m) => write!(f, "corrupt data in database: {m}"),
            StoreError::Internal(m) => write!(f, "database error: {m}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(err: rusqlite::Error) -> Self {
        match &err {
            rusqlite::Error::SqliteFailure(e, msg) => match e.code {
                ErrorCode::ConstraintViolation => match e.extended_code {
                    ffi::SQLITE_CONSTRAINT_UNIQUE | ffi::SQLITE_CONSTRAINT_PRIMARYKEY => {
                        StoreError::AlreadyExists
                    }
                    ffi::SQLITE_CONSTRAINT_FOREIGNKEY => StoreError::ForeignKeyViolation,
                    _ => StoreError::ConstraintViolation(
                        msg.clone().unwrap_or_else(|| e.to_string()),
                    ),
                },
                ErrorCode::NotADatabase => StoreError::WrongKeyOrNotADatabase,
                ErrorCode::DatabaseCorrupt => StoreError::CorruptData(err.to_string()),
                ErrorCode::DatabaseBusy
                | ErrorCode::DatabaseLocked
                | ErrorCode::CannotOpen
                | ErrorCode::ReadOnly
                | ErrorCode::DiskFull
                | ErrorCode::SystemIoFailure
                | ErrorCode::PermissionDenied => StoreError::Unavailable(err.to_string()),
                _ => StoreError::Internal(err.to_string()),
            },
            rusqlite::Error::FromSqlConversionFailure(..)
            | rusqlite::Error::InvalidColumnType(..)
            | rusqlite::Error::IntegralValueOutOfRange(..) => {
                StoreError::CorruptData(err.to_string())
            }
            _ => StoreError::Internal(err.to_string()),
        }
    }
}

impl From<rusqlite_migration::Error> for StoreError {
    fn from(err: rusqlite_migration::Error) -> Self {
        match err {
            rusqlite_migration::Error::RusqliteError { err, .. } => StoreError::from(err),
            other => StoreError::Migration(other.to_string()),
        }
    }
}

/// Read-only diagnostics about the open database (no secrets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseInfo {
    pub sqlite_version: String,
    pub cipher_version: String,
    pub journal_mode: String,
    pub synchronous: i64,
    pub foreign_keys: bool,
    pub busy_timeout_ms: i64,
    pub schema_version: i64,
    pub latest_schema_version: usize,
}

/// The application's single SQLCipher connection.
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open (creating if needed) the encrypted database at `path`, configure
    /// it and apply pending migrations. Blocking: call once at startup (or
    /// inside `spawn_blocking`).
    pub fn new(path: &str, key: &str) -> Result<Self, StoreError> {
        let mut conn = open_connection(path, key)?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Run read-only work on the blocking pool.
    pub async fn read<T, F>(&self, f: F) -> Result<T, StoreError>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> Result<T, StoreError> + Send + 'static,
    {
        let conn = self.conn.clone();
        run_blocking(move || f(&lock(&conn))).await
    }

    /// Run work in a `BEGIN IMMEDIATE` transaction on the blocking pool.
    /// Committed if `f` returns `Ok`, rolled back otherwise.
    pub async fn write<T, F>(&self, f: F) -> Result<T, StoreError>
    where
        T: Send + 'static,
        F: FnOnce(&rusqlite::Transaction<'_>) -> Result<T, StoreError> + Send + 'static,
    {
        let conn = self.conn.clone();
        run_blocking(move || {
            let mut conn = lock(&conn);
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let value = f(&tx)?;
            tx.commit()?;
            Ok(value)
        })
        .await
    }

    /// Cheap readiness probe: the connection can read the schema.
    pub async fn ping(&self) -> Result<(), StoreError> {
        self.read(|conn| {
            conn.query_row("SELECT count(*) FROM sqlite_schema", [], |_| Ok(()))?;
            Ok(())
        })
        .await
    }

    pub async fn info(&self) -> Result<DatabaseInfo, StoreError> {
        self.read(|conn| {
            let pragma_text = |name: &str| -> Result<String, StoreError> {
                Ok(conn.pragma_query_value(None, name, |row| row.get(0))?)
            };
            let pragma_int = |name: &str| -> Result<i64, StoreError> {
                Ok(conn.pragma_query_value(None, name, |row| row.get(0))?)
            };
            Ok(DatabaseInfo {
                sqlite_version: conn.query_row("SELECT sqlite_version()", [], |r| r.get(0))?,
                cipher_version: cipher_version(conn)?.unwrap_or_default(),
                journal_mode: pragma_text("journal_mode")?,
                synchronous: pragma_int("synchronous")?,
                foreign_keys: pragma_int("foreign_keys")? == 1,
                busy_timeout_ms: pragma_int("busy_timeout")?,
                schema_version: pragma_int("user_version")?,
                latest_schema_version: MIGRATIONS.len(),
            })
        })
        .await
    }
}

/// Lock the connection. A poisoned mutex only means an earlier closure
/// panicked; any open transaction was rolled back when it unwound, so the
/// connection is still consistent and safe to reuse.
fn lock(conn: &Mutex<Connection>) -> MutexGuard<'_, Connection> {
    conn.lock().unwrap_or_else(PoisonError::into_inner)
}

async fn run_blocking<T, F>(f: F) -> Result<T, StoreError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, StoreError> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| StoreError::Internal(format!("database task failed: {e}")))?
}

/// Open and configure one SQLCipher connection.
fn open_connection(path: &str, key: &str) -> Result<Connection, StoreError> {
    let in_memory = path == IN_MEMORY;
    if !in_memory {
        prepare_parent_directory(Path::new(path))?;
    }

    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(path, flags)
        .map_err(|e| StoreError::Unavailable(format!("cannot open database at {path}: {e}")))?;
    if !in_memory {
        restrict_file_permissions(Path::new(path))?;
    }

    // SQLCipher: `key` must be the first statement. `pragma_update` quotes the
    // value as a string literal, so no SQL is assembled from the secret.
    conn.pragma_update(None, "key", key)?;
    // Plain SQLite ignores `PRAGMA key`; refuse to run unencrypted.
    if cipher_version(&conn)?.is_none() {
        return Err(StoreError::NotSqlCipher);
    }
    // First real read: fails with SQLITE_NOTADB if the key is wrong.
    conn.query_row("SELECT count(*) FROM sqlite_schema", [], |_| Ok(()))?;

    // Enforce declared foreign keys (off by default in SQLite, per connection).
    conn.pragma_update(None, "foreign_keys", true)?;
    // WAL: commits append to the (SQLCipher-encrypted) -wal file, and readers
    // such as an operator shell or backup do not block the server's writes.
    // In-memory databases keep their "memory" journal.
    let journal_mode: String =
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))?;
    // FULL: every commit is fsynced. A wallet whose address was handed out
    // must survive a power loss, so durability beats the small write cost.
    conn.pragma_update(None, "synchronous", "FULL")?;
    conn.busy_timeout(BUSY_TIMEOUT)?;

    let foreign_keys: i64 = conn.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
    if foreign_keys != 1 {
        return Err(StoreError::Internal(
            "foreign keys could not be enabled".into(),
        ));
    }
    if !in_memory && !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(StoreError::Internal(format!(
            "WAL journal mode could not be enabled (got {journal_mode})"
        )));
    }
    Ok(conn)
}

/// `Some(version)` when the linked library is SQLCipher.
fn cipher_version(conn: &Connection) -> Result<Option<String>, StoreError> {
    let mut stmt = conn.prepare("PRAGMA cipher_version")?;
    let mut rows = stmt.query([])?;
    Ok(match rows.next()? {
        Some(row) => Some(row.get::<_, String>(0)?).filter(|v| !v.is_empty()),
        None => None,
    })
}

fn migrate(conn: &mut Connection) -> Result<(), StoreError> {
    let user_version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let has_tables: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%')",
        [],
        |r| r.get(0),
    )?;
    if user_version == 0 && has_tables {
        return Err(StoreError::UnmanagedSchema);
    }
    Migrations::from_slice(MIGRATIONS).to_latest(conn)?;
    Ok(())
}

/// Create the database directory (owner-only on Unix) if it is missing.
fn prepare_parent_directory(path: &Path) -> Result<(), StoreError> {
    let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return Ok(());
    };
    if parent.is_dir() {
        return Ok(());
    }
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(parent).map_err(|e| {
        StoreError::Unavailable(format!(
            "cannot create database directory {}: {e}",
            parent.display()
        ))
    })
}

/// Make the database file owner-read/write only. SQLite creates the -wal and
/// -shm files with the same permissions as the database file.
fn restrict_file_permissions(path: &Path) -> Result<(), StoreError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| {
            StoreError::Unavailable(format!("cannot set permissions on {}: {e}", path.display()))
        })?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        Migrations::from_slice(MIGRATIONS)
            .validate()
            .expect("migrations must apply cleanly to an empty database");
    }

    #[test]
    fn constraint_errors_are_classified() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE p (id INTEGER PRIMARY KEY, v TEXT UNIQUE CHECK (v <> 'bad'));
             CREATE TABLE c (p_id INTEGER NOT NULL REFERENCES p (id));
             INSERT INTO p (id, v) VALUES (1, 'a');",
        )
        .unwrap();
        let err = |sql: &str| StoreError::from(conn.execute(sql, []).unwrap_err());
        assert_eq!(
            err("INSERT INTO p (v) VALUES ('a')"),
            StoreError::AlreadyExists
        );
        assert_eq!(
            err("INSERT INTO c (p_id) VALUES (99)"),
            StoreError::ForeignKeyViolation
        );
        assert!(matches!(
            err("INSERT INTO p (v) VALUES ('bad')"),
            StoreError::ConstraintViolation(_)
        ));
    }
}
