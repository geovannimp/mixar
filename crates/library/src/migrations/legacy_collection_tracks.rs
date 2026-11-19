//! Rename the pre-`collection_entries` table.
//!
//! Older databases called the membership table `collection_tracks`. Entity sync
//! only creates missing tables, so without the rename it would build an empty
//! `collection_entries` beside the populated old one and every collection would
//! appear empty. Runs before sync; a fresh database has no old table.

use sea_orm::DatabaseConnection;

use super::{execute, table_exists, Migration, MigrationError};

pub(crate) const VERSION: &str = "2026111001_legacy_collection_tracks";

pub(crate) const MIGRATION: Migration = Migration {
    version: VERSION,
    up,
};

fn up(conn: &DatabaseConnection) -> Result<(), MigrationError> {
    if !table_exists(conn, VERSION, "collection_tracks")? {
        return Ok(());
    }
    execute(
        conn,
        VERSION,
        "ALTER TABLE collection_tracks RENAME TO collection_entries",
    )
}
