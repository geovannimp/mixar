//! Drop the pre-NI-Stem `track_stem` cache table.
//!
//! Those builds stored four path columns (`vocals_path`, …) instead of the
//! current `path`. SQLite cannot ADD a `NOT NULL path` to a populated table, so
//! the incompatible table is dropped and the baseline's frozen table batch
//! recreates it empty; the stem files themselves stay on disk until the next
//! ensure or clear.
//!
//! One-shot by design: only a database written before this migration can carry
//! the old layout, and a future incompatible shape gets its own migration.

use sea_orm::DatabaseConnection;

use super::{execute, table_columns, table_exists, MigrationError};

pub(crate) const VERSION: &str = super::LEGACY_TRACK_STEM_VERSION;
const TRACK_STEM: &str = "track_stem";
pub(super) const COLUMN_PATH: &str = "path";
pub(super) const COLUMN_VOCALS_PATH: &str = "vocals_path";
pub(super) const COLUMN_DRUMS_PATH: &str = "drums_path";

pub(crate) struct LegacyTrackStemCache;

migration!(LegacyTrackStemCache, VERSION);

fn up_impl(conn: &DatabaseConnection) -> Result<(), MigrationError> {
    if !table_exists(conn, VERSION, TRACK_STEM)? {
        return Ok(());
    }
    let columns = table_columns(conn, VERSION, TRACK_STEM)?;
    let has_path = columns.iter().any(|(name, _)| name == COLUMN_PATH);
    let has_legacy = columns
        .iter()
        .any(|(name, _)| name == COLUMN_VOCALS_PATH || name == COLUMN_DRUMS_PATH);
    if has_legacy || !has_path {
        execute(conn, VERSION, format!("DROP TABLE {TRACK_STEM}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::ConnectionTrait;

    fn open() -> crate::db::Db {
        crate::db::open_in_memory_unmigrated().unwrap()
    }

    fn run(conn: &DatabaseConnection, sql: &str) {
        conn.execute_unprepared(sql).unwrap();
    }

    #[test]
    fn legacy_four_path_table_is_dropped() {
        let db = open();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        run(conn, &format!("DROP TABLE {TRACK_STEM}"));
        run(
            conn,
            &format!(
                "CREATE TABLE {TRACK_STEM} (track_id TEXT NOT NULL PRIMARY KEY, vocals_path TEXT)"
            ),
        );

        up_impl(conn).unwrap();

        assert!(!super::table_exists(conn, VERSION, TRACK_STEM).unwrap());
    }

    #[test]
    fn current_shape_survives() {
        let db = open();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        run(conn, &format!("DROP TABLE {TRACK_STEM}"));
        run(
            conn,
            &format!(
                "CREATE TABLE {TRACK_STEM} (track_id TEXT NOT NULL PRIMARY KEY, path TEXT NOT NULL)"
            ),
        );

        up_impl(conn).unwrap();

        assert!(super::table_exists(conn, VERSION, TRACK_STEM).unwrap());
    }

    #[test]
    fn missing_table_is_left_to_the_baseline() {
        let db = open();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        run(conn, &format!("DROP TABLE {TRACK_STEM}"));

        up_impl(conn).unwrap();

        assert!(!super::table_exists(conn, VERSION, TRACK_STEM).unwrap());
    }
}
