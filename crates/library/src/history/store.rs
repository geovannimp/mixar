//! SQLite index for history sessions.

use std::path::{Path, PathBuf};

use library_core::Result;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::db::{self, Db};
use crate::entity::{history_sessions, HistorySessionEntity};
use crate::history::xspf::{self, HistoryDocument};

#[derive(Clone, Debug, PartialEq)]
pub struct HistorySessionRow {
    pub id: String,
    pub xspf_path: String,
    pub title: String,
    pub started_at: String,
    pub last_activity_at: String,
    pub closed: bool,
    pub entry_count: u32,
}

pub fn history_dir_for_db(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("history")
}

pub fn list_sessions(db: &Db) -> Result<Vec<HistorySessionRow>> {
    let rows = HistorySessionEntity::find()
        .order_by_desc(history_sessions::Column::StartedAt)
        .all(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(rows.into_iter().map(row_from_model).collect())
}

pub fn get_session(db: &Db, id: &str) -> Result<Option<HistorySessionRow>> {
    let row = HistorySessionEntity::find_by_id(id)
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(row.map(row_from_model))
}

pub fn active_session(db: &Db) -> Result<Option<HistorySessionRow>> {
    let row = HistorySessionEntity::find()
        .filter(history_sessions::Column::Closed.eq(0))
        .order_by_desc(history_sessions::Column::StartedAt)
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(row.map(row_from_model))
}

pub fn upsert_session_index(db: &Db, doc: &HistoryDocument, xspf_path: &Path) -> Result<()> {
    let active = history_sessions::ActiveModel {
        id: Set(doc.session.id.clone()),
        xspf_path: Set(xspf_path.to_string_lossy().into_owned()),
        title: Set(doc.session.title.clone()),
        started_at: Set(doc.session.started_at.clone()),
        last_activity_at: Set(doc.session.last_activity_at.clone()),
        closed: Set(if doc.session.closed { 1 } else { 0 }),
        entry_count: Set(doc.entries.len() as i32),
    };
    HistorySessionEntity::insert(active)
        .on_conflict(
            OnConflict::column(history_sessions::Column::Id)
                .update_columns([
                    history_sessions::Column::Title,
                    history_sessions::Column::LastActivityAt,
                    history_sessions::Column::Closed,
                    history_sessions::Column::EntryCount,
                ])
                .to_owned(),
        )
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

pub fn delete_session_index(db: &Db, id: &str) -> Result<()> {
    HistorySessionEntity::delete_by_id(id)
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

pub fn load_document(path: &Path) -> Result<HistoryDocument> {
    xspf::read_document(path)
}

pub fn save_document(path: &Path, doc: &HistoryDocument) -> Result<()> {
    xspf::write_document(path, doc)
}

fn row_from_model(model: history_sessions::Model) -> HistorySessionRow {
    HistorySessionRow {
        id: model.id,
        xspf_path: model.xspf_path,
        title: model.title,
        started_at: model.started_at,
        last_activity_at: model.last_activity_at,
        closed: model.closed != 0,
        entry_count: model.entry_count.max(0) as u32,
    }
}
