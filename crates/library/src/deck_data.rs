//! Hot cue and loop persistence for deck performance data.

use sea_orm::sea_query::OnConflict;
use sea_orm::ColumnTrait;
use sea_orm::{EntityTrait, QueryFilter, Set};

use library_core::{LibraryError, Result, TrackId};

use crate::db::{self, Db};
use crate::entity::{track_hot_cue, track_loop, TrackHotCueEntity, TrackLoopEntity};

#[derive(Debug, Clone, PartialEq)]
pub struct HotCueRecord {
    pub slot_index: u8,
    pub position_ms: i32,
    pub loop_length_beats: Option<i32>,
    pub color: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopRecord {
    pub slot_index: u8,
    pub in_ms: i32,
    pub out_ms: i32,
    pub label: Option<String>,
    pub color: Option<String>,
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

pub fn list_hot_cues(db: &Db, track_id: &TrackId) -> Result<Vec<HotCueRecord>> {
    let rows = TrackHotCueEntity::find()
        .filter(track_hot_cue::Column::TrackId.eq(track_id.as_str()))
        .all(db.conn()?.as_connection())
        .map_err(db::db_err)?;

    let mut cues: Vec<HotCueRecord> = rows
        .into_iter()
        .map(|row| HotCueRecord {
            slot_index: row.slot_index as u8,
            position_ms: row.position_ms,
            loop_length_beats: row.loop_length_beats,
            color: row.color,
            label: row.label,
        })
        .collect();
    cues.sort_by_key(|cue| cue.slot_index);
    Ok(cues)
}

pub fn save_hot_cue(
    db: &Db,
    track_id: &TrackId,
    slot_index: u8,
    position_ms: i32,
    loop_length_beats: Option<i32>,
    color: Option<String>,
    label: Option<String>,
) -> Result<()> {
    if slot_index > 15 {
        return Err(LibraryError::Backend {
            backend: "deck_data",
            message: "hot cue slot must be 0..=15".into(),
        });
    }

    TrackHotCueEntity::insert(track_hot_cue::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        slot_index: Set(i32::from(slot_index)),
        position_ms: Set(position_ms),
        loop_length_beats: Set(loop_length_beats),
        color: Set(color),
        label: Set(label),
        updated_at: Set(now_iso()),
    })
    .on_conflict(
        OnConflict::columns([
            track_hot_cue::Column::TrackId,
            track_hot_cue::Column::SlotIndex,
        ])
        .update_columns([
            track_hot_cue::Column::PositionMs,
            track_hot_cue::Column::LoopLengthBeats,
            track_hot_cue::Column::Color,
            track_hot_cue::Column::Label,
            track_hot_cue::Column::UpdatedAt,
        ])
        .to_owned(),
    )
    .exec(db.conn()?.as_connection())
    .map_err(db::db_err)?;
    Ok(())
}

pub fn delete_hot_cue(db: &Db, track_id: &TrackId, slot_index: u8) -> Result<()> {
    TrackHotCueEntity::delete_many()
        .filter(track_hot_cue::Column::TrackId.eq(track_id.as_str()))
        .filter(track_hot_cue::Column::SlotIndex.eq(i32::from(slot_index)))
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

pub fn list_loops(db: &Db, track_id: &TrackId) -> Result<Vec<LoopRecord>> {
    let rows = TrackLoopEntity::find()
        .filter(track_loop::Column::TrackId.eq(track_id.as_str()))
        .all(db.conn()?.as_connection())
        .map_err(db::db_err)?;

    let mut loops: Vec<LoopRecord> = rows
        .into_iter()
        .map(|row| LoopRecord {
            slot_index: row.slot_index as u8,
            in_ms: row.in_ms,
            out_ms: row.out_ms,
            label: row.label,
            color: row.color,
        })
        .collect();
    loops.sort_by_key(|row| row.slot_index);
    Ok(loops)
}

pub fn save_loop(
    db: &Db,
    track_id: &TrackId,
    slot_index: u8,
    in_ms: i32,
    out_ms: i32,
    label: Option<String>,
    color: Option<String>,
) -> Result<()> {
    if slot_index > 15 {
        return Err(LibraryError::Backend {
            backend: "deck_data",
            message: "loop slot must be 0..=15".into(),
        });
    }
    if out_ms <= in_ms {
        return Err(LibraryError::Backend {
            backend: "deck_data",
            message: "loop out must be after loop in".into(),
        });
    }

    TrackLoopEntity::insert(track_loop::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        slot_index: Set(i32::from(slot_index)),
        in_ms: Set(in_ms),
        out_ms: Set(out_ms),
        label: Set(label),
        color: Set(color),
        updated_at: Set(now_iso()),
    })
    .on_conflict(
        OnConflict::columns([track_loop::Column::TrackId, track_loop::Column::SlotIndex])
            .update_columns([
                track_loop::Column::InMs,
                track_loop::Column::OutMs,
                track_loop::Column::Label,
                track_loop::Column::Color,
                track_loop::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec(db.conn()?.as_connection())
    .map_err(db::db_err)?;
    Ok(())
}

pub fn delete_loop(db: &Db, track_id: &TrackId, slot_index: u8) -> Result<()> {
    TrackLoopEntity::delete_many()
        .filter(track_loop::Column::TrackId.eq(track_id.as_str()))
        .filter(track_loop::Column::SlotIndex.eq(i32::from(slot_index)))
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::store::Store;
    use library_core::TrackMetadata;
    use std::path::Path;

    #[test]
    fn hot_cue_and_loop_roundtrip_ms() {
        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/a.wav");
        store
            .upsert_file_track(
                &id,
                Path::new("/music/a.wav"),
                &TrackMetadata::default(),
                "1",
            )
            .unwrap();

        save_hot_cue(&db, &id, 0, 12_500, None, None, Some("drop".into())).unwrap();
        save_loop(&db, &id, 1, 0, 1000, None, None).unwrap();

        let cues = list_hot_cues(&db, &id).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].position_ms, 12_500);

        let loops = list_loops(&db, &id).unwrap();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].in_ms, 0);
        assert_eq!(loops[0].out_ms, 1000);

        assert!(save_loop(&db, &id, 2, 1000, 1000, None, None).is_err());
        assert!(save_hot_cue(&db, &id, 0, -500, None, None, None).is_ok());
        assert_eq!(list_hot_cues(&db, &id).unwrap()[0].position_ms, -500);
    }
}
