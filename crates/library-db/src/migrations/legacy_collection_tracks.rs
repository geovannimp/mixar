//! Rename the pre-`collection_entries` table.
//!
//! Older databases called the membership table `collection_tracks`. Without the
//! rename the baseline would create an empty `collection_entries` beside the
//! populated old one and every collection would appear empty. A fresh database
//! has no old table, so this step is a no-op there.

use sea_orm::DatabaseConnection;

use super::{execute, table_exists, MigrationError};

pub(crate) const VERSION: &str = super::LEGACY_COLLECTION_TRACKS_VERSION;
const LEGACY_TABLE: &str = "collection_tracks";
const CURRENT_TABLE: &str = "collection_entries";

pub(crate) struct LegacyCollectionTracks;

migration!(LegacyCollectionTracks, VERSION);

fn up_impl(conn: &DatabaseConnection) -> Result<(), MigrationError> {
    if !table_exists(conn, VERSION, LEGACY_TABLE)? {
        return Ok(());
    }
    // Both names means the database already has a membership table under the
    // current name; picking either one would silently lose the other's rows.
    if table_exists(conn, VERSION, CURRENT_TABLE)? {
        return Err(MigrationError::new(
            VERSION,
            format!("both {LEGACY_TABLE} and {CURRENT_TABLE} exist"),
        ));
    }
    execute(
        conn,
        VERSION,
        format!("ALTER TABLE {LEGACY_TABLE} RENAME TO {CURRENT_TABLE}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    fn open() -> crate::db::Db {
        crate::db::open_in_memory_unmigrated().unwrap()
    }

    fn column_name(column: impl sea_orm::ColumnTrait) -> String {
        column.as_column_ref().1.to_string()
    }

    fn with_conn<T>(db: &crate::db::Db, f: impl FnOnce(&DatabaseConnection) -> T) -> T {
        let handle = db.conn().unwrap();
        f(handle.as_connection())
    }

    fn run(conn: &DatabaseConnection, sql: &str) {
        conn.execute_unprepared(sql).unwrap();
    }

    fn exists(conn: &DatabaseConnection, table: &str) -> bool {
        super::table_exists(conn, VERSION, table).unwrap()
    }

    #[test]
    fn legacy_table_is_renamed_with_its_rows() {
        let db = open();
        with_conn(&db, |conn| {
            let id = column_name(crate::entity::collection_entries::Column::Id);
            let position = column_name(crate::entity::collection_entries::Column::Position);
            run(conn, &format!("DROP TABLE {CURRENT_TABLE}"));
            run(
                conn,
                &format!(
                    "CREATE TABLE {LEGACY_TABLE} ({id} TEXT NOT NULL PRIMARY KEY, {position} INTEGER)"
                ),
            );
            run(
                conn,
                &format!("INSERT INTO {LEGACY_TABLE} VALUES ('e1', 3)"),
            );

            up_impl(conn).unwrap();

            assert!(!exists(conn, LEGACY_TABLE));
            assert!(exists(conn, CURRENT_TABLE));
            let row = conn
                .query_one_raw(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    format!("SELECT {id}, {position} FROM {CURRENT_TABLE}"),
                ))
                .unwrap()
                .unwrap();
            assert_eq!(row.try_get::<String>("", id.as_str()).unwrap(), "e1");
            assert_eq!(row.try_get::<i64>("", position.as_str()).unwrap(), 3);
        });
    }

    #[test]
    fn both_tables_are_rejected_and_left_alone() {
        let db = open();
        with_conn(&db, |conn| {
            let id = column_name(crate::entity::collection_entries::Column::Id);
            run(
                conn,
                &format!("CREATE TABLE {LEGACY_TABLE} ({id} TEXT NOT NULL PRIMARY KEY)"),
            );

            let error = up_impl(conn).unwrap_err();

            assert!(error
                .to_string()
                .contains(&format!("both {LEGACY_TABLE} and {CURRENT_TABLE} exist")));
            assert!(exists(conn, LEGACY_TABLE));
            assert!(exists(conn, CURRENT_TABLE));
        });
    }
}
