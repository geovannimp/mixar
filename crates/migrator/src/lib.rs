//! Versioned, single-direction migrations for a synchronous SeaORM connection.
//!
//! A migration is a plain type: a version and a function that runs on the
//! connection the application keeps using afterwards. This crate owns the
//! ledger, the ordering and the per-step transaction; it has no schema differ,
//! no `down`, and no async runtime.
//!
//! ```ignore
//! struct Baseline;
//!
//! impl migrator::Migration for Baseline {
//!     const VERSION: &'static str = "2026111200_library_schema_baseline";
//!
//!     fn up(&self, conn: &sea_orm::DatabaseConnection) -> Result<(), migrator::Error> {
//!         conn.execute_unprepared(include_str!("baseline.sql"))?;
//!         Ok(())
//!     }
//! }
//!
//! let report = migrator::Migrator::new().push(Baseline).up(&conn)?;
//! ```
//!
//! Steps are matched by exact version, so a ledger row written by another build
//! (a newer one, a side-by-side one) is never re-applied and never deleted.
//! Re-running is therefore safe: a step that is already recorded does nothing.

use std::fmt;

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

/// Table recording applied steps unless a [`Migrator`] names another one.
pub const LEDGER_TABLE: &str = "seaql_migrations";
const DEFAULT_LEDGER: &str = LEDGER_TABLE;
/// Ledger column holding a step version.
pub const VERSION_COLUMN: &str = "version";
/// Ledger column holding when a step was applied.
pub const APPLIED_AT_COLUMN: &str = "applied_at";

/// A SQLite statement with its bound values.
fn sqlite_statement(sql: String, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Sqlite, sql, values)
}

/// Rows for a ledger or pragma query.
fn query_all(
    conn: &DatabaseConnection,
    sql: String,
    values: Vec<sea_orm::Value>,
) -> Result<Vec<sea_orm::QueryResult>, Error> {
    conn.query_all_raw(sqlite_statement(sql, values))
        .map_err(Error::from)
}

/// At most one row for a ledger or schema query.
fn query_one(
    conn: &DatabaseConnection,
    sql: String,
    values: Vec<sea_orm::Value>,
) -> Result<Option<sea_orm::QueryResult>, Error> {
    conn.query_one_raw(sqlite_statement(sql, values))
        .map_err(Error::from)
}

/// One migration step.
pub trait Migration {
    /// Recorded in the ledger. Unique and sorted lexicographically, so a
    /// timestamp prefix keeps the chain in release order.
    const VERSION: &'static str;

    /// Applies the step. It runs inside the migrator's `BEGIN IMMEDIATE`
    /// transaction: returning `Err` rolls back everything it wrote, including
    /// the version row.
    ///
    /// `?` works on [`sea_orm::DbErr`]; use [`Error::message`] for a failure
    /// that needs its own wording.
    fn up(&self, conn: &DatabaseConnection) -> Result<(), Error>;
}

/// A chain of steps, applied oldest version first.
pub struct Migrator {
    ledger: String,
    steps: Vec<Step>,
}

impl Migrator {
    /// An empty chain recording into `seaql_migrations`.
    pub fn new() -> Self {
        Self {
            ledger: DEFAULT_LEDGER.to_owned(),
            steps: Vec::new(),
        }
    }

    /// Records versions in `table` instead of [`DEFAULT_LEDGER`].
    ///
    /// The name is used as a quoted identifier, so it must be plain ASCII
    /// letters, digits and underscores.
    pub fn with_ledger_table(mut self, table: impl Into<String>) -> Self {
        self.ledger = table.into();
        self
    }

    /// Appends a step. Registration order does not matter; [`Self::up`] sorts
    /// by version before running anything.
    #[must_use]
    pub fn push<M: Migration + 'static>(mut self, migration: M) -> Self {
        self.steps.push(Step {
            version: M::VERSION,
            up: Box::new(move |conn| migration.up(conn)),
        });
        self
    }

    /// Applies every step that is not recorded yet.
    ///
    /// Each step commits its schema changes and its version row together; a
    /// failure leaves the database exactly as it was before that step.
    pub fn up(&self, conn: &DatabaseConnection) -> Result<Report, Error> {
        let ordered = self.ordered()?;
        validate_ledger(&self.ledger)?;
        self.ensure_ledger(conn)?;

        let recorded: std::collections::HashSet<String> = self
            .applied(conn)?
            .into_iter()
            .map(|row| row.version)
            .collect();
        let mut applied = Vec::new();
        for step in ordered {
            if recorded.contains(step.version) {
                continue;
            }
            self.begin(conn, step.version)?;
            match self.apply(conn, step) {
                Ok(ran) => {
                    if let Err(err) = self.commit(conn, step.version) {
                        self.rollback(conn);
                        return Err(err);
                    }
                    if ran {
                        applied.push(step.version);
                        tracing::info!(version = step.version, "migration applied");
                    }
                }
                Err(err) => {
                    self.rollback(conn);
                    return Err(err);
                }
            }
        }
        Ok(Report { applied })
    }

    /// The chain with each step's applied timestamp, oldest version first.
    ///
    /// Does not create the ledger: a database that never ran the chain reports
    /// every step as pending.
    pub fn status(&self, conn: &DatabaseConnection) -> Result<Vec<Status>, Error> {
        let ordered = self.ordered()?;
        validate_ledger(&self.ledger)?;
        let recorded = if table_exists(conn, &self.ledger)? {
            self.applied(conn)?
        } else {
            Vec::new()
        };
        Ok(ordered
            .into_iter()
            .map(|step| Status {
                version: step.version,
                applied_at: recorded
                    .iter()
                    .find(|row| row.version == step.version)
                    .map(|row| row.applied_at.clone()),
            })
            .collect())
    }

    fn ordered(&self) -> Result<Vec<&Step>, Error> {
        let mut ordered: Vec<&Step> = self.steps.iter().collect();
        ordered.sort_by_key(|step| step.version);
        if let Some(duplicate) = ordered
            .windows(2)
            .find(|pair| pair[0].version == pair[1].version)
        {
            return Err(Error::message(format!(
                "duplicate migration version {}",
                duplicate[0].version
            )));
        }
        Ok(ordered)
    }

    /// Run the step unless a competing opener recorded it first.
    ///
    /// `Ok(false)` means the version was already in the ledger inside this
    /// transaction, so this call did not apply it.
    fn apply(&self, conn: &DatabaseConnection, step: &Step) -> Result<bool, Error> {
        // A competing opener may have applied this step between the fast path
        // and the write lock.
        if self.is_applied(conn, step.version)? {
            return Ok(false);
        }
        (step.up)(conn).map_err(|err| err.with_version(step.version))?;
        self.record(conn, step.version)?;
        Ok(true)
    }

    fn ensure_ledger(&self, conn: &DatabaseConnection) -> Result<(), Error> {
        conn.execute_unprepared(&format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" \
                 ( \"{VERSION_COLUMN}\" varchar NOT NULL PRIMARY KEY, \
                   \"{APPLIED_AT_COLUMN}\" varchar NOT NULL )",
            self.ledger
        ))
        .map(|_| ())
        .map_err(Error::from)
    }

    fn applied(&self, conn: &DatabaseConnection) -> Result<Vec<LedgerRow>, Error> {
        let rows = query_all(
            conn,
            format!(
                "SELECT {VERSION_COLUMN}, {APPLIED_AT_COLUMN} FROM \"{}\" \
                 ORDER BY {VERSION_COLUMN}",
                self.ledger
            ),
            Vec::new(),
        )?;
        rows.iter()
            .map(|row| {
                Ok(LedgerRow {
                    version: row
                        .try_get::<String>("", VERSION_COLUMN)
                        .map_err(Error::from)?,
                    applied_at: applied_at_text(row).map_err(Error::from)?,
                })
            })
            .collect()
    }

    fn is_applied(&self, conn: &DatabaseConnection, version: &str) -> Result<bool, Error> {
        let row = query_one(
            conn,
            format!(
                "SELECT 1 AS found FROM \"{}\" WHERE {VERSION_COLUMN} = ? LIMIT 1",
                self.ledger
            ),
            vec![version.into()],
        )?;
        Ok(row.is_some())
    }

    fn record(&self, conn: &DatabaseConnection, version: &str) -> Result<(), Error> {
        let applied_at = self.applied_at_value(conn)?;
        conn.execute_raw(sqlite_statement(
            format!(
                "INSERT INTO \"{}\" ({VERSION_COLUMN}, {APPLIED_AT_COLUMN}) VALUES (?, ?)",
                self.ledger
            ),
            vec![version.into(), applied_at],
        ))
        .map(|_| ())
        .map_err(Error::from)
    }

    /// A timestamp in the shape the ledger's `applied_at` column already uses.
    ///
    /// A ledger created by another build declares it `integer`; writing text
    /// there would leave two shapes in one column.
    fn applied_at_value(&self, conn: &DatabaseConnection) -> Result<sea_orm::Value, Error> {
        if self.ledger_applied_at_is_integer(conn)? {
            Ok(jiff::Timestamp::now().as_second().into())
        } else {
            Ok(utc_rfc3339(jiff::Timestamp::now()).into())
        }
    }

    fn ledger_applied_at_is_integer(&self, conn: &DatabaseConnection) -> Result<bool, Error> {
        let rows = query_all(
            conn,
            format!("PRAGMA table_info(\"{}\")", self.ledger),
            Vec::new(),
        )?;
        for row in rows {
            let name: String = row.try_get("", "name").map_err(Error::from)?;
            if name == APPLIED_AT_COLUMN {
                let kind: String = row.try_get("", "type").map_err(Error::from)?;
                return Ok(kind.to_ascii_lowercase().contains("int"));
            }
        }
        Ok(false)
    }

    fn begin(&self, conn: &DatabaseConnection, version: &'static str) -> Result<(), Error> {
        conn.execute_unprepared("BEGIN IMMEDIATE")
            .map(|_| ())
            .map_err(|err| Error::message(err.to_string()).with_version(version))
    }

    fn commit(&self, conn: &DatabaseConnection, version: &'static str) -> Result<(), Error> {
        conn.execute_unprepared("COMMIT")
            .map(|_| ())
            .map_err(|err| Error::message(err.to_string()).with_version(version))
    }

    fn rollback(&self, conn: &DatabaseConnection) {
        let _ = conn.execute_unprepared("ROLLBACK");
    }
}

impl Default for Migrator {
    fn default() -> Self {
        Self::new()
    }
}

/// One step, ready to run: its ledger version and the function body.
struct Step {
    version: &'static str,
    up: StepFn,
}

type StepFn = Box<dyn Fn(&DatabaseConnection) -> Result<(), Error>>;

/// What one [`Migrator::up`] call applied, oldest first.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub applied: Vec<&'static str>,
}

impl Report {
    /// True when the database was already up to date.
    pub fn is_empty(&self) -> bool {
        self.applied.is_empty()
    }
}

/// One step as the ledger sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub version: &'static str,
    /// RFC 3339 text, or `None` while the step is pending.
    pub applied_at: Option<String>,
}

/// A failure from a step or from the ledger.
#[derive(Debug)]
pub struct Error {
    version: Option<&'static str>,
    message: String,
}

impl Error {
    /// A failure that needs its own wording; the migrator adds the version
    /// when the error comes out of a step.
    pub fn message(message: impl Into<String>) -> Self {
        Self {
            version: None,
            message: message.into(),
        }
    }

    /// The version of the step that failed, if the failure belongs to one.
    pub fn version(&self) -> Option<&'static str> {
        self.version
    }

    fn with_version(mut self, version: &'static str) -> Self {
        self.version.get_or_insert(version);
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.version {
            Some(version) => write!(f, "migration {version}: {}", self.message),
            None => write!(f, "migration: {}", self.message),
        }
    }
}

impl std::error::Error for Error {}

impl From<sea_orm::DbErr> for Error {
    fn from(err: sea_orm::DbErr) -> Self {
        Self::message(err.to_string())
    }
}

struct LedgerRow {
    version: String,
    applied_at: String,
}

/// RFC 3339 in UTC with chrono's `AutoSi` precision: 0, 3, 6 or 9 fractional
/// digits, chosen from the instant's nanosecond value.
fn utc_rfc3339(ts: jiff::Timestamp) -> String {
    let precision = match ts.subsec_nanosecond() {
        0 => 0,
        ns if ns % 1_000_000 == 0 => 3,
        ns if ns % 1_000 == 0 => 6,
        _ => 9,
    };
    let ts = ts.display_with_offset(jiff::tz::Offset::UTC);
    match precision {
        0 => format!("{ts:.0}"),
        3 => format!("{ts:.3}"),
        6 => format!("{ts:.6}"),
        _ => format!("{ts:.9}"),
    }
}

/// Unix seconds as RFC 3339 in UTC, using chrono's proleptic Gregorian range
/// and signed/expanded year formatting.
fn epoch_rfc3339(secs: i64) -> Option<String> {
    let (year, month, day, hour, minute, second) = civil_from_unix(secs);
    (-262_143..=262_142).contains(&year).then(|| {
        format!(
            "{}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}+00:00",
            chrono_year(year)
        )
    })
}

fn chrono_year(year: i64) -> String {
    if year < 0 {
        format!("-{:04}", -year)
    } else if year > 9_999 {
        format!("+{year}")
    } else {
        format!("{year:04}")
    }
}

/// Howard Hinnant's `civil_from_days`, extended with the time of day.
fn civil_from_unix(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (
        year,
        month as u32,
        day as u32,
        (sod / 3_600) as u32,
        ((sod % 3_600) / 60) as u32,
        (sod % 60) as u32,
    )
}

/// `applied_at` is text in ledgers this crate writes, but a ledger created by
/// SeaORM's migrator holds an integer epoch; both decode, and status reports
/// whichever is stored.
fn applied_at_text(row: &sea_orm::QueryResult) -> Result<String, sea_orm::DbErr> {
    if let Ok(text) = row.try_get::<String>("", APPLIED_AT_COLUMN) {
        return Ok(text);
    }
    let epoch = row.try_get::<i64>("", APPLIED_AT_COLUMN)?;
    epoch_rfc3339(epoch)
        .ok_or_else(|| sea_orm::DbErr::Custom(format!("applied_at epoch {epoch} is out of range")))
}

fn validate_ledger(table: &str) -> Result<(), Error> {
    let valid = !table.is_empty()
        && table
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if valid {
        Ok(())
    } else {
        Err(Error::message(format!(
            "invalid ledger table name {table:?}"
        )))
    }
}

fn table_exists(conn: &DatabaseConnection, table: &str) -> Result<bool, Error> {
    let row = query_one(
        conn,
        "SELECT COUNT(*) AS count FROM sqlite_schema WHERE type = 'table' AND name = ?".to_owned(),
        vec![table.into()],
    )?;
    Ok(row
        .and_then(|result| result.try_get::<i64>("", "count").ok())
        .unwrap_or(0)
        > 0)
}

#[cfg(test)]
mod tests;
