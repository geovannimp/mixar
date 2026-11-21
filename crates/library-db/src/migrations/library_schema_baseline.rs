//! The library schema frozen at the first versioned migration.
//!
//! Fresh databases are created here and databases written by older builds are
//! upgraded in place, both through the same statements. The table batch is a
//! byte-for-byte capture of the schema the entity-first sync produced before
//! migrations existed, so do not regenerate it from the entities: a later schema
//! change appends a migration of its own and leaves this baseline untouched.
//!
//! The parts the capture cannot express as plain `CREATE TABLE` live here: the
//! two compatibility operations that used to run before schema sync (renaming
//! `collection_tracks` and dropping an unusable stem cache table), the additive
//! column upgrades for pre-existing tables, the checks that reject legacy shapes
//! the current app cannot read, and the indexes.

use std::collections::BTreeMap;

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};

use crate::db::{TRACKS_TABLE, TRACK_BEAT_SEGMENT_TABLE};

pub struct Baseline;

migration!(Baseline, super::BASELINE_VERSION, db_error);

fn up_impl(conn: &DatabaseConnection) -> Result<(), DbErr> {
    rename_legacy_collection_tracks(conn)?;
    drop_legacy_track_stem(conn)?;

    execute_sql(conn, SCHEMA)?;

    // One reading of each table's column set, refreshed as columns are added, so
    // the upgrade and the validation below agree on what the table now has.
    let mut columns = BTreeMap::new();
    add_missing_columns(conn, &mut columns)?;
    validate_frozen_columns(conn, &mut columns)?;
    create_indexes(conn)?;
    ensure_unique_indexes(conn)?;
    Ok(())
}

/// The frozen table batch: one `CREATE TABLE IF NOT EXISTS` per application
/// table, ordered so the tables a foreign key points at come first.
///
/// Copied byte-for-byte from the schema the entity-first build produced. The
/// catalog stores this text, so reflowing a line is a schema change: append a
/// migration instead of editing it.
const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS "sampler_bank" ( "id" varchar NOT NULL PRIMARY KEY, "name" varchar NOT NULL, "play_mode" varchar, "sort_index" integer NOT NULL, "updated_at" varchar NOT NULL );
CREATE TABLE IF NOT EXISTS "tracks" ( "id" varchar NOT NULL PRIMARY KEY, "source_type" varchar NOT NULL, "source_ref" varchar NOT NULL, "provider" varchar, "title" varchar, "artist" varchar, "album" varchar, "genre" varchar, "bpm" double, "bpm_source" varchar, "key" varchar, "key_source" varchar, "loudness_lufs" double, "analysis_completed_at" varchar, "duration_ms" integer, "sample_rate" integer, "channels" integer, "bitrate_kbps" integer, "replaygain_track_gain_db" double, "isrc" varchar, "rating" integer, "comment" varchar, "year" integer, "track_number" integer, "disc_number" integer, "color_label" varchar, "color_rgb" integer, "metadata_edited" boolean, "serato_imported_at" varchar, "serato_adopted_kinds" integer, "last_sampler_bank_id" varchar, "added_at" varchar NOT NULL, "updated_at" varchar NOT NULL, FOREIGN KEY ("last_sampler_bank_id") REFERENCES "sampler_bank" ("id") ON DELETE SET NULL ON UPDATE CASCADE );
CREATE TABLE IF NOT EXISTS "collections" ( "id" varchar NOT NULL PRIMARY KEY, "name" varchar NOT NULL, "collection_type" varchar NOT NULL, "sortable" integer NOT NULL DEFAULT 1, "scan_folder_tree" integer NOT NULL DEFAULT 1, "fs_path" varchar UNIQUE );
CREATE TABLE IF NOT EXISTS "collection_entries" ( "id" varchar NOT NULL PRIMARY KEY, "collection_id" varchar NOT NULL, "track_id" varchar NOT NULL, "position" integer, FOREIGN KEY ("collection_id") REFERENCES "collections" ("id") ON DELETE CASCADE ON UPDATE CASCADE, FOREIGN KEY ("track_id") REFERENCES "tracks" ("id") ON DELETE CASCADE ON UPDATE CASCADE );
CREATE TABLE IF NOT EXISTS "history_sessions" ( "id" varchar NOT NULL PRIMARY KEY, "xspf_path" varchar NOT NULL UNIQUE, "title" varchar NOT NULL, "started_at" varchar NOT NULL, "last_activity_at" varchar NOT NULL, "closed" integer NOT NULL, "entry_count" integer NOT NULL );
CREATE TABLE IF NOT EXISTS "sampler_slot" ( "bank_id" varchar NOT NULL, "slot_index" integer NOT NULL, "track_id" varchar, "path" varchar, "label" varchar, "updated_at" varchar NOT NULL, CONSTRAINT "pk-sampler_slot" PRIMARY KEY ("bank_id", "slot_index"), FOREIGN KEY ("bank_id") REFERENCES "sampler_bank" ("id") ON DELETE CASCADE ON UPDATE CASCADE, FOREIGN KEY ("track_id") REFERENCES "tracks" ("id") ON DELETE SET NULL ON UPDATE CASCADE );
CREATE TABLE IF NOT EXISTS "track_analysis" ( "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT, "track_id" varchar NOT NULL, "backend" varchar NOT NULL, "backend_version" varchar NOT NULL, "analyzed_at" varchar NOT NULL, "bpm" double, "bpm_confidence" double, "key" varchar, "key_confidence" double, "key_clarity" double, "grid_stability" double, "sample_rate" integer NOT NULL, "duration_analyzed_ms" integer NOT NULL, "loudness_lufs" double, "beat_grid_json" varchar, FOREIGN KEY ("track_id") REFERENCES "tracks" ("id") ON DELETE CASCADE ON UPDATE CASCADE );
CREATE TABLE IF NOT EXISTS "track_beat_grid" ( "track_id" varchar NOT NULL PRIMARY KEY, "source" varchar NOT NULL, "covered_to_ms" integer, "created_at" varchar NOT NULL, "updated_at" varchar NOT NULL, FOREIGN KEY ("track_id") REFERENCES "tracks" ("id") ON DELETE CASCADE ON UPDATE CASCADE );
CREATE TABLE IF NOT EXISTS "track_beat_segment" ( "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT, "track_id" varchar NOT NULL, "segment_index" integer NOT NULL, "start_time_ms" integer NOT NULL, "bpm" double NOT NULL, "beats_per_bar" integer NOT NULL, "beat_unit" integer NOT NULL, "start_beat_index" integer NOT NULL, "beats_to_next" integer, "downbeat_offset_beats" integer NOT NULL, FOREIGN KEY ("track_id") REFERENCES "track_beat_grid" ("track_id") ON DELETE CASCADE ON UPDATE CASCADE );
CREATE TABLE IF NOT EXISTS "track_cue" ( "track_id" varchar NOT NULL PRIMARY KEY, "cue_ms" integer NOT NULL, "source" varchar NOT NULL, "updated_at" varchar NOT NULL, FOREIGN KEY ("track_id") REFERENCES "tracks" ("id") ON DELETE CASCADE ON UPDATE CASCADE );
CREATE TABLE IF NOT EXISTS "track_hot_cue" ( "track_id" varchar NOT NULL, "slot_index" integer NOT NULL, "position_ms" integer NOT NULL, "loop_length_beats" integer, "color" varchar, "label" varchar, "updated_at" varchar NOT NULL, CONSTRAINT "pk-track_hot_cue" PRIMARY KEY ("track_id", "slot_index"), FOREIGN KEY ("track_id") REFERENCES "tracks" ("id") ON DELETE CASCADE ON UPDATE CASCADE );
CREATE TABLE IF NOT EXISTS "track_loop" ( "track_id" varchar NOT NULL, "slot_index" integer NOT NULL, "in_ms" integer NOT NULL, "out_ms" integer NOT NULL, "label" varchar, "color" varchar, "auto_saved" boolean NOT NULL DEFAULT FALSE, "updated_at" varchar NOT NULL, CONSTRAINT "pk-track_loop" PRIMARY KEY ("track_id", "slot_index"), FOREIGN KEY ("track_id") REFERENCES "tracks" ("id") ON DELETE CASCADE ON UPDATE CASCADE );
CREATE TABLE IF NOT EXISTS "track_stem" ( "track_id" varchar NOT NULL PRIMARY KEY, "backend" varchar NOT NULL, "source_fingerprint" varchar NOT NULL DEFAULT '', "format" varchar NOT NULL DEFAULT 'opus', "sample_rate" integer NOT NULL, "path" varchar NOT NULL, "generated_at" varchar NOT NULL, FOREIGN KEY ("track_id") REFERENCES "tracks" ("id") ON DELETE CASCADE ON UPDATE CASCADE );
CREATE TABLE IF NOT EXISTS "track_waveform" ( "track_id" varchar NOT NULL PRIMARY KEY, "version" integer NOT NULL, "amplitude_mode" varchar NOT NULL, "channel_mode" varchar NOT NULL, "overview_count" integer NOT NULL, "overview_bytes" varbinary_blob NOT NULL, "generated_at" varchar NOT NULL, FOREIGN KEY ("track_id") REFERENCES "tracks" ("id") ON DELETE CASCADE ON UPDATE CASCADE );
"#;

/// Columns later schema revisions added to tables that already existed.
///
/// `(table, column, definition)`. The definition copies the type, nullability
/// and default from the capture, and a statement only runs when the column is
/// absent, so values and any extra legacy columns survive untouched.
const ADDED_COLUMNS: &[(&str, &str, &str)] = &[
    (TRACKS_TABLE, "provider", "varchar"),
    (TRACKS_TABLE, "title", "varchar"),
    (TRACKS_TABLE, "artist", "varchar"),
    (TRACKS_TABLE, "album", "varchar"),
    (TRACKS_TABLE, "genre", "varchar"),
    (TRACKS_TABLE, "bpm", "double"),
    (TRACKS_TABLE, "key", "varchar"),
    (TRACKS_TABLE, "duration_ms", "integer"),
    (TRACKS_TABLE, "sample_rate", "integer"),
    (TRACKS_TABLE, "channels", "integer"),
    (TRACKS_TABLE, "bitrate_kbps", "integer"),
    (TRACKS_TABLE, "replaygain_track_gain_db", "double"),
    (TRACKS_TABLE, "isrc", "varchar"),
    (TRACKS_TABLE, "rating", "integer"),
    (TRACKS_TABLE, "comment", "varchar"),
    (TRACKS_TABLE, "year", "integer"),
    (TRACKS_TABLE, "track_number", "integer"),
    (TRACKS_TABLE, "disc_number", "integer"),
    (TRACKS_TABLE, "color_label", "varchar"),
    (TRACKS_TABLE, "color_rgb", "integer"),
    (TRACKS_TABLE, "metadata_edited", "boolean"),
    (TRACKS_TABLE, "serato_imported_at", "varchar"),
    (TRACKS_TABLE, "serato_adopted_kinds", "integer"),
    (TRACKS_TABLE, "bpm_source", "varchar"),
    (TRACKS_TABLE, "key_source", "varchar"),
    (TRACKS_TABLE, "loudness_lufs", "double"),
    (TRACKS_TABLE, "analysis_completed_at", "varchar"),
    (TRACKS_TABLE, "last_sampler_bank_id", "varchar"),
    (
        COLLECTIONS,
        "scan_folder_tree",
        "integer NOT NULL DEFAULT 1",
    ),
    (COLLECTIONS, "sortable", "integer NOT NULL DEFAULT 1"),
    (TRACK_LOOP, "auto_saved", "boolean NOT NULL DEFAULT 0"),
    (TRACK_ANALYSIS, "loudness_lufs", "double"),
    (TRACK_ANALYSIS, "beat_grid_json", "varchar"),
    (
        TRACK_STEM,
        "source_fingerprint",
        "varchar NOT NULL DEFAULT ''",
    ),
    ("track_stem", "format", "varchar NOT NULL DEFAULT 'opus'"),
];

/// Every column the current app reads, per table.
///
/// Presence is all that is checked: types, defaults and foreign keys belong to
/// the table that already owns them and are never rewritten.
const REQUIRED_COLUMNS: &[(&str, &[&str])] = &[
    (
        "sampler_bank",
        &["id", "name", "play_mode", "sort_index", "updated_at"],
    ),
    (
        "tracks",
        &[
            "id",
            "source_type",
            "source_ref",
            "provider",
            "title",
            "artist",
            "album",
            "genre",
            "bpm",
            "key",
            "duration_ms",
            "sample_rate",
            "channels",
            "bitrate_kbps",
            "replaygain_track_gain_db",
            "isrc",
            "rating",
            "comment",
            "year",
            "track_number",
            "disc_number",
            "color_label",
            "color_rgb",
            "metadata_edited",
            "serato_imported_at",
            "serato_adopted_kinds",
            "last_sampler_bank_id",
            "added_at",
            "updated_at",
        ],
    ),
    (
        COLLECTIONS,
        &[
            "id",
            "name",
            "collection_type",
            "sortable",
            "scan_folder_tree",
            "fs_path",
        ],
    ),
    (
        COLLECTION_ENTRIES,
        &["id", "collection_id", "track_id", "position"],
    ),
    (
        "history_sessions",
        &[
            "id",
            "xspf_path",
            "title",
            "started_at",
            "last_activity_at",
            "closed",
            "entry_count",
        ],
    ),
    (
        "sampler_slot",
        &[
            "bank_id",
            "slot_index",
            "track_id",
            "path",
            "label",
            "updated_at",
        ],
    ),
    (
        TRACK_ANALYSIS,
        &[
            "track_id",
            "backend",
            "backend_version",
            "analyzed_at",
            "bpm",
            "bpm_confidence",
            "key",
            "key_confidence",
            "key_clarity",
            "grid_stability",
            "sample_rate",
            "duration_analyzed_ms",
            "loudness_lufs",
            "beat_grid_json",
        ],
    ),
    (
        "track_beat_grid",
        &[
            "track_id",
            "source",
            "covered_to_ms",
            "created_at",
            "updated_at",
        ],
    ),
    (
        "track_beat_segment",
        &[
            "id",
            "track_id",
            "segment_index",
            "start_time_ms",
            "bpm",
            "beats_per_bar",
            "beat_unit",
            "start_beat_index",
            "beats_to_next",
            "downbeat_offset_beats",
        ],
    ),
    ("track_cue", &["track_id", "cue_ms", "source", "updated_at"]),
    (
        "track_hot_cue",
        &[
            "track_id",
            "slot_index",
            "position_ms",
            "loop_length_beats",
            "color",
            "label",
            "updated_at",
        ],
    ),
    (
        "track_loop",
        &[
            "track_id",
            "slot_index",
            "in_ms",
            "out_ms",
            "label",
            "color",
            "auto_saved",
            "updated_at",
        ],
    ),
    (
        TRACK_STEM,
        &[
            "track_id",
            "backend",
            "source_fingerprint",
            "format",
            "sample_rate",
            "path",
            "generated_at",
        ],
    ),
    (
        "track_waveform",
        &[
            "track_id",
            "version",
            "amplitude_mode",
            "channel_mode",
            "overview_count",
            "overview_bytes",
            "generated_at",
        ],
    ),
];

/// Indexes the entity-first sync created with the tables.
///
/// Guarded so a legacy database that already carries one keeps it as it is.
pub(super) const TRACK_ANALYSIS: &str = "track_analysis";
pub(super) const TRACK_ID_COLUMN: &str = "track_id";
const TRACK_STEM: &str = "track_stem";
const COLLECTION_ENTRIES: &str = "collection_entries";
const COLLECTIONS: &str = "collections";
const TRACK_LOOP: &str = "track_loop";
const HISTORY_SESSIONS: &str = "history_sessions";
const COLUMN_ARTIST: &str = "artist";
const COLUMN_SOURCE_REF: &str = "source_ref";
const COLUMN_TITLE: &str = "title";
const COLUMN_FS_PATH: &str = "fs_path";
const COLUMN_XSPF_PATH: &str = "xspf_path";
const LEGACY_COLLECTION_TRACKS: &str = "collection_tracks";
pub(super) const INDEX_TRACK_ANALYSIS_TRACK_ID: &str = "idx-track_analysis-track_id";
const INDEX_COLLECTIONS_FS_PATH: &str = "idx-collections-fs_path";
const INDEX_HISTORY_SESSIONS_XSPF_PATH: &str = "idx-history_sessions-xspf_path";
const INDEX_TRACK_BEAT_SEGMENT_TRACK_ID: &str = "idx-track_beat_segment-track_id";
const INDEX_TRACKS_ARTIST: &str = "idx-tracks-artist";
const INDEX_TRACKS_SOURCE_REF: &str = "idx-tracks-source_ref";
const INDEX_TRACKS_TITLE: &str = "idx-tracks-title";

const INDEXES: &[(&str, &str, &str)] = &[
    (
        TRACK_ANALYSIS,
        TRACK_ID_COLUMN,
        INDEX_TRACK_ANALYSIS_TRACK_ID,
    ),
    (
        TRACK_BEAT_SEGMENT_TABLE,
        TRACK_ID_COLUMN,
        INDEX_TRACK_BEAT_SEGMENT_TRACK_ID,
    ),
    (TRACKS_TABLE, COLUMN_ARTIST, INDEX_TRACKS_ARTIST),
    (TRACKS_TABLE, COLUMN_SOURCE_REF, INDEX_TRACKS_SOURCE_REF),
    (TRACKS_TABLE, COLUMN_TITLE, INDEX_TRACKS_TITLE),
];

/// `(table, column, index)` for the uniqueness a legacy table may spell without
/// a constraint, because adding a constraint to a populated table would mean
/// rebuilding it.
const UNIQUE_INDEXES: &[(&str, &str, &str)] = &[
    (COLLECTIONS, COLUMN_FS_PATH, INDEX_COLLECTIONS_FS_PATH),
    (
        HISTORY_SESSIONS,
        COLUMN_XSPF_PATH,
        INDEX_HISTORY_SESSIONS_XSPF_PATH,
    ),
];

type ColumnCache = BTreeMap<&'static str, Vec<String>>;

/// Rename the playlist entry table the app has always read.
///
/// The rows, their ids and their positions move with the table. A database that
/// carries both names is ambiguous: no rename can be right, so refuse before
/// altering either.
fn rename_legacy_collection_tracks(conn: &DatabaseConnection) -> Result<(), DbErr> {
    let has_legacy = table_exists(conn, LEGACY_COLLECTION_TRACKS)?;
    let has_current = table_exists(conn, COLLECTION_ENTRIES)?;
    if has_legacy && has_current {
        return Err(err("both collection_tracks and collection_entries exist"));
    }
    if has_legacy {
        execute_sql(
            conn,
            &format!(
                "ALTER TABLE \"{LEGACY_COLLECTION_TRACKS}\" RENAME TO \"{COLLECTION_ENTRIES}\""
            ),
        )?;
    }
    Ok(())
}

/// Pre-NI-Stem databases store four path columns (`vocals_path`, …) and the
/// current app cannot add its `NOT NULL path` to that populated table, so drop
/// the cache table and let the batch recreate it. Stem files stay on disk and
/// are regenerated on the next ensure; current-format rows are kept.
fn drop_legacy_track_stem(conn: &DatabaseConnection) -> Result<(), DbErr> {
    if !table_exists(conn, TRACK_STEM)? {
        return Ok(());
    }
    let columns = read_columns(conn, TRACK_STEM)?;
    let has_path = columns
        .iter()
        .any(|name| name == super::legacy_track_stem_cache::COLUMN_PATH);
    let has_legacy = columns.iter().any(|name| {
        name == super::legacy_track_stem_cache::COLUMN_VOCALS_PATH
            || name == super::legacy_track_stem_cache::COLUMN_DRUMS_PATH
    });
    if has_legacy || !has_path {
        execute_sql(conn, &format!("DROP TABLE \"{TRACK_STEM}\""))?;
    }
    Ok(())
}

fn add_missing_columns(conn: &DatabaseConnection, cache: &mut ColumnCache) -> Result<(), DbErr> {
    for (table, column, definition) in ADDED_COLUMNS {
        let existing = columns_of(conn, cache, table)?;
        if existing.iter().any(|name| name == column) {
            continue;
        }
        let sql = format!("ALTER TABLE \"{table}\" ADD COLUMN \"{column}\" {definition}");
        execute_sql(conn, &sql)?;
        existing.push((*column).to_string());
    }
    Ok(())
}

/// Reject legacy shapes the current app cannot read instead of inventing data.
///
/// Playlist entries need their own entry id (no inferred ids for
/// `(collection_id, track_id)` tables), and every table needs the columns the
/// stores query. A rejection rolls the whole baseline back, rename and cache
/// drop included.
fn validate_frozen_columns(
    conn: &DatabaseConnection,
    cache: &mut ColumnCache,
) -> Result<(), DbErr> {
    if !table_exists(conn, COLLECTION_ENTRIES)? {
        return Err(err(format!(
            "unsupported legacy schema: {COLLECTION_ENTRIES} is missing"
        )));
    }
    let primary_key = primary_key(conn, COLLECTION_ENTRIES)?;
    if primary_key != ["id"] {
        return Err(err(format!(
            "unsupported legacy schema: {COLLECTION_ENTRIES} requires id as its primary key"
        )));
    }

    for (table, required) in REQUIRED_COLUMNS {
        let existing = columns_of(conn, cache, table)?;
        for column in *required {
            if !existing.iter().any(|name| name == column) {
                return Err(err(format!(
                    "unsupported legacy schema: {table}.{column} is missing"
                )));
            }
        }
    }
    Ok(())
}

fn create_indexes(conn: &DatabaseConnection) -> Result<(), DbErr> {
    for (table, column, name) in INDEXES {
        execute_sql(
            conn,
            &format!("CREATE INDEX IF NOT EXISTS \"{name}\" ON \"{table}\" (\"{column}\")"),
        )?;
    }
    Ok(())
}

/// Give a pre-existing table the full single-column unique index its current
/// definition declares inline.
///
/// A duplicate value or a taken index name fails the baseline and rolls back;
/// the data is never deduplicated and custom indexes are never dropped.
fn ensure_unique_indexes(conn: &DatabaseConnection) -> Result<(), DbErr> {
    for (table, column, index) in UNIQUE_INDEXES {
        if has_single_column_unique_index(conn, table, column)? {
            continue;
        }
        // A same-named non-unique index on this column would make
        // `CREATE UNIQUE` fail; the data is what decides whether the unique
        // version can be built. An index of that name on another column is
        // someone else's and must survive.
        if index_columns(conn, index)?
            .iter()
            .any(|name| name == column)
        {
            execute_sql(conn, &format!("DROP INDEX IF EXISTS \"{index}\""))?;
        }
        execute_sql(
            conn,
            &format!("CREATE UNIQUE INDEX IF NOT EXISTS \"{index}\" ON \"{table}\" (\"{column}\")"),
        )?;
    }
    Ok(())
}

/// Rows for a probe query, naming what could not be read.
fn query_rows(
    conn: &DatabaseConnection,
    sql: &str,
    values: Vec<sea_orm::Value>,
    context: impl std::fmt::Display,
) -> Result<Vec<sea_orm::QueryResult>, DbErr> {
    conn.query_all_raw(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        sql,
        values,
    ))
    .map_err(|cause| err(format!("{context}: {cause}")))
}

fn has_single_column_unique_index(
    conn: &DatabaseConnection,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    let rows = query_rows(
        conn,
        "SELECT name FROM pragma_index_list(?) WHERE \"unique\" = 1",
        vec![table.into()],
        format_args!("cannot read indexes of {table}"),
    )?;
    for row in rows {
        let index: String = row
            .try_get("", "name")
            .map_err(|cause| err(cause.to_string()))?;
        let columns = index_columns(conn, &index)?;
        if columns.len() == 1 && columns[0] == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The `name` column of a pragma result, in row order.
fn column_names(rows: &[sea_orm::QueryResult]) -> Result<Vec<String>, DbErr> {
    rows.iter()
        .map(|row| {
            row.try_get::<String>("", "name")
                .map_err(|cause| err(format!("unreadable pragma column name: {cause}")))
        })
        .collect()
}

fn index_columns(conn: &DatabaseConnection, index: &str) -> Result<Vec<String>, DbErr> {
    let rows = query_rows(
        conn,
        "SELECT name FROM pragma_index_info(?) ORDER BY seqno",
        vec![index.into()],
        format_args!("cannot read columns of index {index}"),
    )?;
    column_names(&rows)
}

fn primary_key(conn: &DatabaseConnection, table: &str) -> Result<Vec<String>, DbErr> {
    let rows = query_rows(
        conn,
        "SELECT name, pk FROM pragma_table_info(?) WHERE pk > 0 ORDER BY pk",
        vec![table.into()],
        format_args!("cannot read columns of {table}"),
    )?;
    column_names(&rows)
}

fn table_exists(conn: &DatabaseConnection, table: &str) -> Result<bool, DbErr> {
    let row = query_rows(
        conn,
        "SELECT COUNT(*) AS count FROM sqlite_schema WHERE type = 'table' AND name = ?",
        vec![table.into()],
        "cannot read sqlite_schema",
    )?
    .into_iter()
    .next();
    Ok(row
        .and_then(|found| found.try_get::<i64>("", "count").ok())
        .unwrap_or(0)
        > 0)
}

fn read_columns(conn: &DatabaseConnection, table: &str) -> Result<Vec<String>, DbErr> {
    let rows = query_rows(
        conn,
        "SELECT name FROM pragma_table_info(?)",
        vec![table.into()],
        format_args!("cannot read columns of {table}"),
    )?;
    column_names(&rows)
}

fn columns_of<'a>(
    conn: &DatabaseConnection,
    cache: &'a mut ColumnCache,
    table: &'static str,
) -> Result<&'a mut Vec<String>, DbErr> {
    if !cache.contains_key(table) {
        cache.insert(table, read_columns(conn, table)?);
    }
    cache
        .get_mut(table)
        .ok_or_else(|| err(format!("missing column cache entry for {table}")))
}

fn execute_sql(conn: &DatabaseConnection, sql: &str) -> Result<(), DbErr> {
    conn.execute_unprepared(sql)
        .map(|_| ())
        .map_err(|cause| err(cause.to_string()))
}

fn err(message: impl Into<String>) -> DbErr {
    DbErr::Migration(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> crate::db::Db {
        crate::db::open_in_memory_unmigrated().unwrap()
    }

    #[test]
    fn legacy_collection_table_is_renamed() {
        let db = open();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        execute_sql(conn, &format!("DROP TABLE {COLLECTION_ENTRIES}")).unwrap();
        execute_sql(
            conn,
            &format!("CREATE TABLE {LEGACY_COLLECTION_TRACKS} (id TEXT PRIMARY KEY)"),
        )
        .unwrap();

        rename_legacy_collection_tracks(conn).unwrap();

        assert!(table_exists(conn, COLLECTION_ENTRIES).unwrap());
        assert!(!table_exists(conn, LEGACY_COLLECTION_TRACKS).unwrap());
    }

    #[test]
    fn both_collection_tables_are_rejected() {
        let db = open();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        execute_sql(
            conn,
            &format!("CREATE TABLE {LEGACY_COLLECTION_TRACKS} (id TEXT PRIMARY KEY)"),
        )
        .unwrap();

        assert!(rename_legacy_collection_tracks(conn).is_err());
    }

    #[test]
    fn legacy_stem_cache_is_dropped_and_the_current_one_kept() {
        let db = open();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        execute_sql(conn, &format!("DROP TABLE {TRACK_STEM}")).unwrap();
        execute_sql(
            conn,
            &format!("CREATE TABLE {TRACK_STEM} (track_id TEXT PRIMARY KEY, vocals_path TEXT)"),
        )
        .unwrap();

        drop_legacy_track_stem(conn).unwrap();

        assert!(!table_exists(conn, TRACK_STEM).unwrap());

        execute_sql(
            conn,
            &format!("CREATE TABLE {TRACK_STEM} (track_id TEXT PRIMARY KEY, path TEXT NOT NULL)"),
        )
        .unwrap();

        drop_legacy_track_stem(conn).unwrap();

        assert!(table_exists(conn, TRACK_STEM).unwrap());
    }

    #[test]
    fn entries_without_an_id_primary_key_are_rejected() {
        let db = open();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        execute_sql(conn, &format!("DROP TABLE {COLLECTION_ENTRIES}")).unwrap();
        execute_sql(
            conn,
            "CREATE TABLE collection_entries (track_id TEXT PRIMARY KEY)",
        )
        .unwrap();

        let error = validate_frozen_columns(conn, &mut BTreeMap::new()).unwrap_err();

        assert!(
            error.to_string().contains("requires id as its primary key"),
            "unexpected error: {error}"
        );
    }
}
