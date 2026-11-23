use std::sync::atomic::{AtomicUsize, Ordering};

use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

use super::*;

fn connection() -> DatabaseConnection {
    Database::connect("sqlite::memory:").expect("in-memory database")
}

fn table_exists_now(conn: &DatabaseConnection, table: &str) -> bool {
    conn.query_one_raw(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "SELECT COUNT(*) AS count FROM sqlite_schema WHERE type = 'table' AND name = ?",
        [table.into()],
    ))
    .expect("schema query")
    .and_then(|row| row.try_get::<i64>("", "count").ok())
    .unwrap_or(0)
        > 0
}

fn ledger_rows(conn: &DatabaseConnection) -> Vec<(String, String)> {
    conn.query_all_raw(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!(
            "SELECT {VERSION_COLUMN}, {APPLIED_AT_COLUMN} FROM {LEDGER_TABLE} ORDER BY {VERSION_COLUMN}"
        ),
    ))
    .expect("ledger query")
    .iter()
    .map(|row| {
        let applied_at = row
            .try_get::<String>("", APPLIED_AT_COLUMN)
            .or_else(|_| {
                row.try_get::<i64>("", APPLIED_AT_COLUMN)
                    .map(|value| value.to_string())
            })
            .expect("applied_at");
        (
            row.try_get::<String>("", VERSION_COLUMN)
                .expect("version"),
            applied_at,
        )
    })
    .collect()
}

/// A ledger shaped by SeaORM's migrator: `applied_at` as an integer epoch.
fn insert_integer_ledger(conn: &DatabaseConnection, version: &str) {
    conn.execute_unprepared(&format!(
        "CREATE TABLE {LEDGER_TABLE} (
             {VERSION_COLUMN} varchar NOT NULL PRIMARY KEY,
             {APPLIED_AT_COLUMN} integer NOT NULL
         );"
    ))
    .expect("integer ledger table");
    conn.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        format!("INSERT INTO {LEDGER_TABLE} ({VERSION_COLUMN}, {APPLIED_AT_COLUMN}) VALUES (?, ?)"),
        [version.into(), 1_700_000_000i64.into()],
    ))
    .expect("integer ledger row");
}

fn versions(conn: &DatabaseConnection) -> Vec<String> {
    ledger_rows(conn)
        .into_iter()
        .map(|(version, _)| version)
        .collect()
}

struct CreateFirst;

impl Migration for CreateFirst {
    const VERSION: &'static str = "2026010100_first";

    fn up(&self, conn: &DatabaseConnection) -> Result<(), Error> {
        conn.execute_unprepared("CREATE TABLE first (id integer)")?;
        Ok(())
    }
}

struct CreateSecond;

impl Migration for CreateSecond {
    const VERSION: &'static str = "2026010101_second";

    fn up(&self, conn: &DatabaseConnection) -> Result<(), Error> {
        conn.execute_unprepared("CREATE TABLE second (id integer)")?;
        Ok(())
    }
}

/// Same version as [`CreateFirst`], to prove duplicates are rejected.
struct SameVersionAsFirst;

impl Migration for SameVersionAsFirst {
    const VERSION: &'static str = "2026010100_first";

    fn up(&self, conn: &DatabaseConnection) -> Result<(), Error> {
        conn.execute_unprepared("CREATE TABLE duplicate (id integer)")?;
        Ok(())
    }
}

static FAILED_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

struct Fails;

impl Migration for Fails {
    const VERSION: &'static str = "2026010102_fails";

    fn up(&self, conn: &DatabaseConnection) -> Result<(), Error> {
        FAILED_ATTEMPTS.fetch_add(1, Ordering::SeqCst);
        conn.execute_unprepared("CREATE TABLE rolled_back (id integer)")?;
        Err(Error::message("boom"))
    }
}

#[test]
fn applies_each_step_once_and_records_it() {
    let conn = connection();
    // Registered out of order on purpose: the chain sorts by version.
    let migrator = Migrator::new().push(CreateSecond).push(CreateFirst);

    let report = migrator.up(&conn).expect("first run");

    assert_eq!(report.applied, ["2026010100_first", "2026010101_second"]);
    assert!(table_exists_now(&conn, "first"));
    assert!(table_exists_now(&conn, "second"));
    let rows = ledger_rows(&conn);
    assert_eq!(
        rows.iter()
            .map(|(version, _)| version.as_str())
            .collect::<Vec<_>>(),
        ["2026010100_first", "2026010101_second"]
    );
    assert!(
        rows.iter()
            .all(|(_, applied_at)| applied_at.contains('T') && applied_at.contains('+')),
        "the ledger keeps RFC3339 text: {rows:?}"
    );

    let again = migrator.up(&conn).expect("second run");
    assert!(again.is_empty());
    assert_eq!(ledger_rows(&conn), rows, "a second run records nothing");
}

#[test]
fn status_converts_an_integer_applied_at_to_rfc3339() {
    let conn = connection();
    insert_integer_ledger(&conn, <CreateFirst as Migration>::VERSION);

    let status = Migrator::new().push(CreateFirst).status(&conn).unwrap();

    assert_eq!(
        status[0].applied_at.as_deref(),
        Some("2023-11-14T22:13:20+00:00"),
        "status reports RFC 3339 for an epoch ledger"
    );
}

#[test]
fn report_lists_only_versions_this_call_applied() {
    let conn = connection();
    let migrator = Migrator::new().push(CreateFirst);

    let first = migrator.up(&conn).unwrap();
    assert_eq!(
        first.applied,
        vec![<CreateFirst as Migration>::VERSION],
        "the first call ran the step"
    );

    let second = migrator.up(&conn).unwrap();
    assert!(
        second.applied.is_empty(),
        "an applied step is not reported again: {:?}",
        second.applied
    );
}

#[test]
fn rolls_back_a_failed_step_and_leaves_it_pending() {
    let conn = connection();
    let migrator = Migrator::new().push(Fails);

    let err = migrator.up(&conn).expect_err("the step fails");

    assert_eq!(err.version(), Some("2026010102_fails"));
    assert!(err.to_string().contains("boom"), "{err}");
    assert!(
        !table_exists_now(&conn, "rolled_back"),
        "a failed step writes nothing"
    );
    assert!(
        ledger_rows(&conn).is_empty(),
        "a failed step is not recorded"
    );

    migrator.up(&conn).expect_err("retrying fails the same way");
    assert_eq!(FAILED_ATTEMPTS.load(Ordering::SeqCst), 2);
}

#[test]
fn a_failing_step_keeps_the_steps_before_it() {
    let conn = connection();

    Migrator::new()
        .push(CreateFirst)
        .push(Fails)
        .up(&conn)
        .expect_err("the second step fails");

    assert!(table_exists_now(&conn, "first"));
    assert_eq!(versions(&conn), ["2026010100_first"]);
    assert!(!table_exists_now(&conn, "rolled_back"));
}

#[test]
fn preserves_rows_recorded_by_another_build() {
    let conn = connection();
    Migrator::new()
        .up(&conn)
        .expect("empty chain creates the ledger");
    conn.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        format!("INSERT INTO {LEDGER_TABLE} ({VERSION_COLUMN}, {APPLIED_AT_COLUMN}) VALUES (?, ?)"),
        [
            "2099010100_from_newer_build".into(),
            "2030-01-01T00:00:00+00:00".into(),
        ],
    ))
    .expect("foreign row");

    let report = Migrator::new()
        .push(CreateFirst)
        .up(&conn)
        .expect("apply the missing step");

    assert_eq!(report.applied, ["2026010100_first"]);
    let rows = ledger_rows(&conn);
    assert!(
        rows.iter().any(
            |(version, applied_at)| version == "2099010100_from_newer_build"
                && applied_at == "2030-01-01T00:00:00+00:00"
        ),
        "unknown rows are carried over untouched: {rows:?}"
    );
}

#[test]
fn rejects_duplicate_versions_before_touching_the_database() {
    let conn = connection();

    let err = Migrator::new()
        .push(CreateFirst)
        .push(SameVersionAsFirst)
        .up(&conn)
        .expect_err("duplicate versions");

    assert!(
        err.to_string()
            .contains("duplicate migration version 2026010100_first"),
        "{err}"
    );
    assert!(!table_exists_now(&conn, LEDGER_TABLE), "no SQL runs");
    assert!(!table_exists_now(&conn, "first"));
}

#[test]
fn status_reports_pending_and_applied_without_creating_the_ledger() {
    let conn = connection();
    let migrator = Migrator::new().push(CreateFirst);

    assert_eq!(
        migrator.status(&conn).expect("pending status"),
        [Status {
            version: "2026010100_first",
            applied_at: None,
        }]
    );
    assert!(
        !table_exists_now(&conn, LEDGER_TABLE),
        "status is read-only"
    );

    migrator.up(&conn).expect("apply");
    let applied = migrator.status(&conn).expect("applied status");
    assert_eq!(applied[0].version, "2026010100_first");
    assert!(applied[0]
        .applied_at
        .as_deref()
        .is_some_and(|applied_at| applied_at.contains('T')));
}

#[test]
fn a_custom_ledger_table_is_used_instead_of_the_default_one() {
    let conn = connection();

    Migrator::new()
        .with_ledger_table("custom_ledger")
        .push(CreateFirst)
        .up(&conn)
        .expect("apply into the custom ledger");

    assert!(table_exists_now(&conn, "custom_ledger"));
    assert!(!table_exists_now(&conn, LEDGER_TABLE));
}

#[test]
fn a_malformed_ledger_name_is_rejected() {
    let conn = connection();

    let err = Migrator::new()
        .with_ledger_table("ledger\"; DROP TABLE tracks; --")
        .up(&conn)
        .expect_err("quoted identifiers are not accepted");

    assert!(
        err.to_string().contains("invalid ledger table name"),
        "{err}"
    );
}

#[test]
fn reads_an_integer_ledger_written_by_seaorms_migrator() {
    let conn = connection();
    insert_integer_ledger(&conn, "2099010100_from_newer_build");

    let status = Migrator::new()
        .push(CreateFirst)
        .status(&conn)
        .expect("status reads both ledger shapes");
    assert_eq!(
        status,
        [Status {
            version: "2026010100_first",
            applied_at: None,
        }]
    );

    Migrator::new()
        .push(CreateFirst)
        .up(&conn)
        .expect("apply next to the integer ledger");

    let rows = ledger_rows(&conn);
    assert!(
        rows.iter().any(
            |(version, applied_at)| version == "2099010100_from_newer_build"
                && applied_at == "1700000000"
        ),
        "the integer row is preserved: {rows:?}"
    );
    assert!(rows
        .iter()
        .any(|(version, _)| version == "2026010100_first"));
}
