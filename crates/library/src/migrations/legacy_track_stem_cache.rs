//! Drop the pre-NI-Stem `track_stem` cache table.
//!
//! Those builds stored four path columns (`vocals_path`, …) instead of the
//! current `path`. Entity sync cannot ADD a `NOT NULL path` to a populated
//! table, so the incompatible table is dropped and sync recreates it empty; the
//! stem files themselves stay on disk until the next ensure or clear.
//!
//! One-shot by design: only a database written before this migration can carry
//! the old layout, and a future incompatible shape gets its own migration.

use sea_orm::DatabaseConnection;

use super::{execute, table_columns, table_exists, Migration, MigrationError};

pub(crate) const VERSION: &str = "2026111002_legacy_track_stem_cache";

pub(crate) const MIGRATION: Migration = Migration {
    version: VERSION,
    up,
};

fn up(conn: &DatabaseConnection) -> Result<(), MigrationError> {
    if !table_exists(conn, VERSION, "track_stem")? {
        return Ok(());
    }
    let columns = table_columns(conn, VERSION, "track_stem")?;
    let has_path = columns.iter().any(|(name, _)| name == "path");
    let has_legacy = columns
        .iter()
        .any(|(name, _)| name == "vocals_path" || name == "drums_path");
    if has_legacy || !has_path {
        execute(conn, VERSION, "DROP TABLE track_stem")?;
    }
    Ok(())
}
