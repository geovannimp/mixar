//! SQLCipher access to a Rekordbox `master.db`.
//!
//! Rekordbox 6/7 encrypt the desktop library with SQLCipher, keyed by a
//! passphrase that is the same on every install (no user secret, no key file).
//! pyrekordbox recovers it from the application binary: base85-decode an
//! obfuscated blob, XOR with `657f48f84c437cc1`, zlib-inflate. [`DATABASE_KEY`]
//! is that derivation's output, as published by the onelibrary
//! reverse-engineering notes and verified there against a live `master.db`;
//! it is a constant because the blob only ships inside AlphaTheta's binary.
//!
//! A rotated passphrase surfaces as [`LibraryError::Backend`] with an
//! explanation, and [`super::RekordboxLibrary::open_keyed`] (or the
//! [`KEY_ENV`] variable) takes a replacement without a rebuild.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use library_core::{path_label, LibraryError, Result};
use rusqlite::{Connection, ErrorCode, OpenFlags};

use super::schema::backend;

/// Passphrase SQLCipher derives the `master.db` key from.
pub const DATABASE_KEY: &str = "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";

/// Environment variable overriding [`DATABASE_KEY`] for a rotated passphrase.
pub const KEY_ENV: &str = "REKORDBOX_DATABASE_KEY";

/// A private copy of a live library, removed when dropped.
///
/// Rekordbox runs while Mixar imports, and a WAL-mode database with a hot
/// `-wal` is not reliably readable in place. The copy lives in a fresh temp
/// directory so nothing else can read it, and the original is only ever read.
pub(crate) struct DatabaseCopy {
    dir: PathBuf,
    path: PathBuf,
}

impl DatabaseCopy {
    /// Copy `source` and its `-wal` into a private directory.
    ///
    /// The guard is built once the private directory exists, so a failure while
    /// copying still drops it and removes what landed there, while a failing
    /// *creation* never deletes a directory this call did not make.
    pub(crate) fn create(source: &Path) -> Result<Self> {
        let dir = temp_dir()?;
        // Creation is exclusive, so on failure there is nothing to clean up —
        // and taking the guard only afterwards keeps `Drop` from deleting a
        // directory this call did not create.
        create_private_dir(&dir)?;
        let copy = Self {
            path: dir.join("master.db"),
            dir,
        };

        fs::copy(source, &copy.path)
            .map_err(|e| backend(format!("copy {}: {e}", path_label(source))))?;

        // The write-ahead log holds pages the main file has not absorbed yet;
        // copying the database without it silently loses recent edits. The
        // `-shm` is scratch state shared with the running process, so it is
        // never copied: SQLite rebuilds it from the copied `-wal`.
        let name = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "master.db".to_string());
        let wal = source.with_file_name(format!("{name}-wal"));
        if wal.is_file() {
            fs::copy(&wal, copy.dir.join("master.db-wal"))
                .map_err(|e| backend(format!("copy {}: {e}", path_label(&wal))))?;
        }

        Ok(copy)
    }

    /// Path of the copy's main database file.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for DatabaseCopy {
    fn drop(&mut self) {
        // Best effort: a leftover temp directory is not worth failing a
        // finished import over. A crash or SIGKILL leaves it behind; it is
        // owner-only, and the next import creates a fresh directory rather than
        // reusing it.
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Open one database file read-only, unlocking it with `passphrase`.
///
/// `label` names the file in errors: the caller passes the *source* path, so a
/// message never points at this adapter's temp copy.
pub(crate) fn open_library(copy: &Path, label: &str, passphrase: &str) -> Result<Connection> {
    let conn = open_read_only(copy)?;
    // An already-decrypted `master.db` is a plain SQLite database: it needs no
    // key, and applying one would make SQLite reject the plaintext header.
    if !is_plaintext(copy) {
        apply_key(&conn, passphrase, label)?;
    }
    conn.execute_batch("PRAGMA query_only = 1;")
        .map_err(|e| backend(format!("set query_only: {e}")))?;
    Ok(conn)
}

/// Open a database with the SQLCipher key applied unconditionally.
///
/// Test hook: production opens the temp copy through [`open_library`].
#[cfg(test)]
pub(crate) fn open_encrypted(path: &Path, passphrase: &str) -> Result<Connection> {
    let conn = open_read_only(path)?;
    apply_key(&conn, passphrase, &path_label(path))?;
    Ok(conn)
}

fn open_read_only(path: &Path) -> Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| backend(format!("open {}: {e}", path_label(path))))
}

/// `PRAGMA key` is lazy: the key is only exercised by the first read, so this
/// runs a cheap query and maps a decryption failure to a typed error.
///
/// The passphrase goes in as a SQL string literal (`PRAGMA key = '…'`), never
/// as `x'…'`: the hex form skips the KDF and SQLite then reports the file as
/// not a database (issue #555).
fn apply_key(conn: &Connection, passphrase: &str, label: &str) -> Result<()> {
    check_passphrase(passphrase)?;
    conn.execute_batch(&format!("PRAGMA key = '{}';", escape(passphrase)))
        .map_err(|e| backend(format!("apply SQLCipher key: {e}")))?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|e| decrypt_error(label, e))?;
    Ok(())
}

/// True when the file carries SQLite's plaintext header.
fn is_plaintext(path: &Path) -> bool {
    use std::io::Read;

    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut header = [0u8; SQLITE_HEADER.len()];
    file.read_exact(&mut header).is_ok() && header == *SQLITE_HEADER
}

/// First 16 bytes of every unencrypted SQLite database.
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

/// Passphrase for a regular import: the [`KEY_ENV`] override, else the
/// community-verified constant.
pub(crate) fn default_passphrase() -> String {
    nonempty_env(KEY_ENV).unwrap_or_else(|| DATABASE_KEY.to_string())
}

fn decrypt_error(label: &str, error: rusqlite::Error) -> LibraryError {
    let wrong_key = match &error {
        rusqlite::Error::SqliteFailure(failure, _) => failure.code == ErrorCode::NotADatabase,
        _ => false,
    };
    if wrong_key {
        return backend(format!(
            "{label}: the database could not be decrypted. Rekordbox's SQLCipher \
             passphrase differs from the known constant; set {KEY_ENV} to the \
             current one (`RekordboxLibrary::open_keyed` also accepts it)."
        ));
    }
    backend(format!("read {label}: {error}"))
}

/// SQL string literal escaping: a doubled single quote.
fn escape(passphrase: &str) -> String {
    passphrase.replace('\'', "''")
}

/// Reject a passphrase that cannot be carried in a SQL string literal.
///
/// `PRAGMA key` takes no bound parameters, so the value goes in as text.
/// Doubling quotes covers the delimiter, and any other character (newlines
/// included) is legal inside a literal; a NUL is not, so it is refused rather
/// than truncated silently.
fn check_passphrase(passphrase: &str) -> Result<()> {
    if passphrase.contains('\0') {
        return Err(backend(
            "the SQLCipher passphrase contains a NUL byte; check REKORDBOX_DATABASE_KEY",
        ));
    }
    Ok(())
}

/// Fresh, unique directory under the system temp directory.
fn temp_dir() -> Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    Ok(std::env::temp_dir().join(format!("mixar-rekordbox-{}-{nanos}", std::process::id())))
}

/// Create the copy directory already owner-only on Unix.
///
/// `create_dir_all` followed by a chmod leaves a window where the directory
/// carries the umask's mode, so the mode is applied at creation instead. On
/// Windows the temp directory's inherited ACL is per-user already, and file
/// modes do not exist.
fn create_private_dir(dir: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        builder.mode(0o700);
    }
    // Exclusive: an existing path is an error rather than reuse, so a stale
    // directory is never handed out.
    builder.create(dir)?;
    // The umask can strip owner bits from the requested mode, so set it again.
    if let Err(err) = restrict_to_owner(dir) {
        let _ = fs::remove_dir_all(dir);
        return Err(err);
    }
    Ok(())
}

/// Owner-only access, so the decrypted pages are not world-readable.
#[cfg(unix)]
fn restrict_to_owner(dir: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_to_owner(_dir: &Path) -> Result<()> {
    Ok(())
}

/// Environment override, kept verbatim: a passphrase may legitimately contain
/// leading or trailing whitespace, and `open_keyed` does not trim either.
fn nonempty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new encrypted database at `path`, keyed with `passphrase`.
    fn encrypted_fixture(path: &Path, passphrase: &str) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(&format!("PRAGMA key = '{}';", escape(passphrase)))
            .unwrap();
        conn.execute_batch("CREATE TABLE probe (id INTEGER PRIMARY KEY, name TEXT);")
            .unwrap();
        conn.execute("INSERT INTO probe (name) VALUES ('hello')", [])
            .unwrap();
    }

    #[test]
    fn reads_with_the_right_passphrase_and_rejects_a_wrong_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("master.db");
        encrypted_fixture(&path, "correct-horse");

        let conn = open_encrypted(&path, "correct-horse").unwrap();
        let name: String = conn
            .query_row("SELECT name FROM probe", [], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "hello");

        let error = open_encrypted(&path, "wrong-horse").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("could not be decrypted"), "{message}");
        assert!(message.contains(KEY_ENV), "{message}");
    }

    #[test]
    fn the_hex_pragma_form_does_not_unlock_a_string_keyed_database() {
        // Documents the trap the issue records: `x'…'` bypasses the KDF (here
        // SQLite rejects the literal outright), so it never opens a database
        // keyed with the string form.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("master.db");
        encrypted_fixture(&path, "abc123");

        let conn = Connection::open(&path).unwrap();
        let hex = format!("x'{}'", escape("abc123"));
        let opened = conn
            .execute_batch(&format!("PRAGMA key = {hex};"))
            .and_then(|()| {
                conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map(|_| ())
            });
        assert!(opened.is_err(), "x'…' must not unlock a KDF-derived key");
    }

    #[cfg(unix)]
    #[test]
    fn the_private_copy_directory_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("master.db");
        encrypted_fixture(&path, "fixture");

        let copy = DatabaseCopy::create(&path).unwrap();
        let mode = fs::metadata(&copy.dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "mode {mode:o}");
    }

    #[test]
    fn the_copy_keeps_uncommitted_wal_pages_and_leaves_the_original_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("master.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA key = 'fixture'; PRAGMA journal_mode = WAL;")
            .unwrap();
        conn.execute_batch("CREATE TABLE probe (id INTEGER PRIMARY KEY);")
            .unwrap();

        let original = fs::read(&path).unwrap();
        let copy = DatabaseCopy::create(&path).unwrap();
        assert!(copy.dir.join("master.db-wal").is_file(), "wal copied");
        // The shared-memory file is scratch: the copy must work without it.
        assert!(!copy.dir.join("master.db-shm").exists(), "shm not copied");
        let copied = open_encrypted(copy.path(), "fixture").unwrap();
        // WAL rows only exist in the copied `-wal`, and SQLite replays them.
        let tables: i64 = copied
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'probe'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 1);
        drop(copied);

        // The source is untouched for as long as the import runs; closing the
        // writer afterwards legitimately checkpoints its WAL into it.
        assert_eq!(fs::read(&path).unwrap(), original, "source left alone");
        drop(conn);
    }
}
