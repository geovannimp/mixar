//! Persisted sampler banks and slots.

use sea_orm::sea_query::OnConflict;
use sea_orm::ColumnTrait;
use sea_orm::{EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use library_core::{LibraryError, Result, TrackId};

use crate::db::{self, Db};
use crate::entity::{
    sampler_bank, sampler_slot, tracks, SamplerBankEntity, SamplerSlotEntity, TrackEntity,
};

pub use crate::entity::SamplerPlayMode;

/// Fixed pad count per bank (validated in app code, not SQL CHECK).
pub const BANK_SIZE: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct SamplerBankRecord {
    pub id: String,
    pub name: String,
    /// `None` = inherit settings.
    pub play_mode: Option<SamplerPlayMode>,
    pub sort_index: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SamplerSlotRecord {
    pub slot_index: u8,
    pub track_id: Option<String>,
    pub path: Option<String>,
    pub label: Option<String>,
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn new_bank_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("bank-{nanos}")
}

fn validate_slot(slot_index: u8) -> Result<()> {
    if usize::from(slot_index) >= BANK_SIZE {
        return Err(LibraryError::Backend {
            backend: "sampler",
            message: format!("sampler slot must be 0..={}", BANK_SIZE - 1),
        });
    }
    Ok(())
}

pub fn list_banks(db: &Db) -> Result<Vec<SamplerBankRecord>> {
    let rows = SamplerBankEntity::find()
        .order_by_asc(sampler_bank::Column::SortIndex)
        .order_by_asc(sampler_bank::Column::Name)
        .all(db.conn()?.as_connection())
        .map_err(db::db_err)?;

    Ok(rows
        .into_iter()
        .map(|row| SamplerBankRecord {
            id: row.id,
            name: row.name,
            play_mode: row.play_mode,
            sort_index: row.sort_index,
        })
        .collect())
}

pub fn get_bank(db: &Db, bank_id: &str) -> Result<Option<SamplerBankRecord>> {
    let Some(row) = SamplerBankEntity::find_by_id(bank_id.to_string())
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?
    else {
        return Ok(None);
    };
    Ok(Some(SamplerBankRecord {
        id: row.id,
        name: row.name,
        play_mode: row.play_mode,
        sort_index: row.sort_index,
    }))
}

pub fn create_bank(
    db: &Db,
    name: &str,
    play_mode: Option<SamplerPlayMode>,
) -> Result<SamplerBankRecord> {
    let name = name.trim();
    if name.is_empty() {
        return Err(LibraryError::Backend {
            backend: "sampler",
            message: "bank name must not be empty".into(),
        });
    }

    let next_sort = SamplerBankEntity::find()
        .count(db.conn()?.as_connection())
        .map_err(db::db_err)? as i32;

    let id = new_bank_id();

    SamplerBankEntity::insert(sampler_bank::ActiveModel {
        id: Set(id.clone()),
        name: Set(name.to_string()),
        play_mode: Set(play_mode),
        sort_index: Set(next_sort),
        updated_at: Set(now_iso()),
    })
    .exec(db.conn()?.as_connection())
    .map_err(db::db_err)?;

    Ok(SamplerBankRecord {
        id,
        name: name.to_string(),
        play_mode,
        sort_index: next_sort,
    })
}

pub fn update_bank(
    db: &Db,
    bank_id: &str,
    name: &str,
    play_mode: Option<SamplerPlayMode>,
) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(LibraryError::Backend {
            backend: "sampler",
            message: "bank name must not be empty".into(),
        });
    }
    let play_mode_value = match play_mode {
        Some(mode) => sea_orm::Value::from(mode),
        None => sea_orm::Value::String(None),
    };
    let result = SamplerBankEntity::update_many()
        .col_expr(sampler_bank::Column::Name, name.into())
        .col_expr(sampler_bank::Column::PlayMode, play_mode_value.into())
        .col_expr(sampler_bank::Column::UpdatedAt, now_iso().into())
        .filter(sampler_bank::Column::Id.eq(bank_id))
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    if result.rows_affected == 0 {
        return Err(LibraryError::Backend {
            backend: "sampler",
            message: format!("sampler bank not found: {bank_id}"),
        });
    }
    Ok(())
}

pub fn delete_bank(db: &Db, bank_id: &str) -> Result<()> {
    let result = SamplerBankEntity::delete_by_id(bank_id.to_string())
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    if result.rows_affected == 0 {
        return Err(LibraryError::Backend {
            backend: "sampler",
            message: format!("sampler bank not found: {bank_id}"),
        });
    }
    Ok(())
}

pub fn list_slots(db: &Db, bank_id: &str) -> Result<Vec<SamplerSlotRecord>> {
    let rows = SamplerSlotEntity::find()
        .filter(sampler_slot::Column::BankId.eq(bank_id))
        .all(db.conn()?.as_connection())
        .map_err(db::db_err)?;

    let mut slots: Vec<SamplerSlotRecord> = rows
        .into_iter()
        .map(|row| SamplerSlotRecord {
            slot_index: row.slot_index as u8,
            track_id: row.track_id,
            path: row.path,
            label: row.label,
        })
        .collect();
    slots.sort_by_key(|s| s.slot_index);
    Ok(slots)
}

pub fn assign_slot(
    db: &Db,
    bank_id: &str,
    slot_index: u8,
    track_id: Option<String>,
    path: Option<String>,
    label: Option<String>,
) -> Result<()> {
    validate_slot(slot_index)?;
    if get_bank(db, bank_id)?.is_none() {
        return Err(LibraryError::Backend {
            backend: "sampler",
            message: format!("sampler bank not found: {bank_id}"),
        });
    }
    if track_id.is_none() && path.is_none() {
        return Err(LibraryError::Backend {
            backend: "sampler",
            message: "track_id or path is required".into(),
        });
    }

    SamplerSlotEntity::insert(sampler_slot::ActiveModel {
        bank_id: Set(bank_id.to_string()),
        slot_index: Set(i32::from(slot_index)),
        track_id: Set(track_id),
        path: Set(path),
        label: Set(label),
        updated_at: Set(now_iso()),
    })
    .on_conflict(
        OnConflict::columns([
            sampler_slot::Column::BankId,
            sampler_slot::Column::SlotIndex,
        ])
        .update_columns([
            sampler_slot::Column::TrackId,
            sampler_slot::Column::Path,
            sampler_slot::Column::Label,
            sampler_slot::Column::UpdatedAt,
        ])
        .to_owned(),
    )
    .exec(db.conn()?.as_connection())
    .map_err(db::db_err)?;
    Ok(())
}

pub fn clear_slot(db: &Db, bank_id: &str, slot_index: u8) -> Result<()> {
    validate_slot(slot_index)?;
    SamplerSlotEntity::delete_many()
        .filter(sampler_slot::Column::BankId.eq(bank_id))
        .filter(sampler_slot::Column::SlotIndex.eq(i32::from(slot_index)))
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

/// Bank last used with this track (written on successful sampler pad trigger).
pub fn get_track_last_sampler_bank_id(db: &Db, track_id: &TrackId) -> Result<Option<String>> {
    let row = TrackEntity::find_by_id(track_id.as_str().to_string())
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(row.and_then(|t| t.last_sampler_bank_id))
}

/// Persist which sampler bank was active when a pad was triggered for this track.
pub fn set_track_last_sampler_bank_id(
    db: &Db,
    track_id: &TrackId,
    bank_id: Option<&str>,
) -> Result<()> {
    let result = TrackEntity::update_many()
        .col_expr(
            tracks::Column::LastSamplerBankId,
            bank_id.map(|s| s.to_string()).into(),
        )
        .filter(tracks::Column::Id.eq(track_id.as_str()))
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    if result.rows_affected == 0 {
        return Err(LibraryError::Backend {
            backend: "sampler",
            message: format!("track not found: {}", track_id.as_str()),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    #[test]
    fn create_list_assign_and_clear() {
        let db = open_in_memory().unwrap();
        let bank = create_bank(&db, "Bank 1", None).unwrap();
        assert_eq!(bank.name, "Bank 1");
        assert!(bank.play_mode.is_none());

        assign_slot(
            &db,
            &bank.id,
            0,
            None,
            Some("/tmp/kick.wav".into()),
            Some("Kick".into()),
        )
        .unwrap();
        let slots = list_slots(&db, &bank.id).unwrap();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].label.as_deref(), Some("Kick"));

        clear_slot(&db, &bank.id, 0).unwrap();
        assert!(list_slots(&db, &bank.id).unwrap().is_empty());
    }

    #[test]
    fn play_mode_round_trips() {
        let db = open_in_memory().unwrap();
        let bank = create_bank(&db, "Hold bank", Some(SamplerPlayMode::Hold)).unwrap();
        assert_eq!(bank.play_mode, Some(SamplerPlayMode::Hold));

        update_bank(&db, &bank.id, &bank.name, Some(SamplerPlayMode::Loop)).unwrap();
        let loaded = get_bank(&db, &bank.id).unwrap().unwrap();
        assert_eq!(loaded.play_mode, Some(SamplerPlayMode::Loop));

        update_bank(&db, &bank.id, &bank.name, None).unwrap();
        let cleared = get_bank(&db, &bank.id).unwrap().unwrap();
        assert!(cleared.play_mode.is_none());
    }

    #[test]
    fn rejects_out_of_range_slot() {
        let db = open_in_memory().unwrap();
        let bank = create_bank(&db, "A", None).unwrap();
        let err = assign_slot(&db, &bank.id, 8, None, Some("/x".into()), None).unwrap_err();
        assert!(err.to_string().contains("slot"));
    }
}
