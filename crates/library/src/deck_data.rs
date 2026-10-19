//! Hot cue and loop persistence for deck performance data.

use sea_orm::sea_query::OnConflict;
use sea_orm::ColumnTrait;
use sea_orm::{EntityTrait, QueryFilter, Set};

use library_core::{Library, LibraryError, Result, TrackId};

use crate::db::{self, Db};
use crate::entity::{track_hot_cue, track_loop, TrackHotCueEntity, TrackLoopEntity};
use crate::{tags, unix_secs_string, LibraryManager, TrackMetadataEdit};

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
        updated_at: Set(crate::unix_secs_string()),
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

    // Preserve label/color when the caller omits them (None) on update.
    let existing = TrackLoopEntity::find()
        .filter(track_loop::Column::TrackId.eq(track_id.as_str()))
        .filter(track_loop::Column::SlotIndex.eq(i32::from(slot_index)))
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    let label = label.or_else(|| existing.as_ref().and_then(|row| row.label.clone()));
    let color = color.or_else(|| existing.as_ref().and_then(|row| row.color.clone()));

    TrackLoopEntity::insert(track_loop::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        slot_index: Set(i32::from(slot_index)),
        in_ms: Set(in_ms),
        out_ms: Set(out_ms),
        label: Set(label),
        color: Set(color),
        updated_at: Set(crate::unix_secs_string()),
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

impl LibraryManager {
    /// Overwrite the user-editable metadata of a track in the library DB.
    ///
    /// Strings are trimmed and blanked to `None`; `key` is normalized to
    /// musical notation (Camelot/Open Key input is accepted). An invalid
    /// non-`None` `bpm` returns an error rather than silently clearing the
    /// column (enforced by `crate::store::Store::update_track_metadata`, which
    /// owns the column invariant). Unknown track ids are a no-op.
    pub fn update_track_metadata(&self, track_id: &TrackId, edit: TrackMetadataEdit) -> Result<()> {
        let normalized = TrackMetadataEdit {
            title: normalize_optional_text(edit.title),
            artist: normalize_optional_text(edit.artist),
            album: normalize_optional_text(edit.album),
            genre: normalize_optional_text(edit.genre),
            bpm: edit.bpm,
            // Trim/blank first, then normalize notation, so there is a single
            // trim (and a Camelot code is never normalized from whitespace).
            key: normalize_optional_text(edit.key).map(|v| tags::normalize_key_notation(&v)),
            isrc: normalize_optional_text(edit.isrc),
        };
        self.store()
            .update_track_metadata(track_id, &normalized, &unix_secs_string())
    }
}

/// Trim; an empty (or whitespace-only) value clears the field.
///
/// An already-trimmed value keeps its allocation, so the common path (code
/// that sent a clean value) does not copy.
fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed.len() == value.len() {
            return Some(value);
        }
        Some(trimmed.to_string())
    })
}

impl LibraryManager {
    /// Load hot cues and saved loops for a track.
    pub fn list_track_hot_cues(&self, id: &TrackId) -> Result<Vec<HotCueRecord>> {
        crate::deck_data::list_hot_cues(&self.db, id)
    }

    /// Read a track's embedded album artwork as a UI thumbnail, if present.
    ///
    /// JPEG bytes at most [`tags::ARTWORK_UI_EDGE_PX`] on the longest edge
    /// (never the full-resolution cover). `None` when the track is a stream,
    /// the file is gone, or it carries no decodable cover.
    pub fn get_track_artwork(&self, id: &TrackId) -> Result<Option<Vec<u8>>> {
        let Some(source) = self.get_track(id)? else {
            return Ok(None);
        };
        let Some(file) = source.file() else {
            return Ok(None);
        };
        tags::read_artwork(file.path())
    }

    pub fn list_track_loops(&self, id: &TrackId) -> Result<Vec<LoopRecord>> {
        crate::deck_data::list_loops(&self.db, id)
    }

    pub fn save_track_hot_cue(
        &self,
        id: &TrackId,
        slot_index: u8,
        position_ms: i32,
        loop_length_beats: Option<i32>,
        color: Option<String>,
        label: Option<String>,
    ) -> Result<()> {
        crate::deck_data::save_hot_cue(
            &self.db,
            id,
            slot_index,
            position_ms,
            loop_length_beats,
            color,
            label,
        )
    }

    pub fn delete_track_hot_cue(&self, id: &TrackId, slot_index: u8) -> Result<()> {
        crate::deck_data::delete_hot_cue(&self.db, id, slot_index)
    }

    pub fn save_track_loop(
        &self,
        id: &TrackId,
        slot_index: u8,
        in_ms: i32,
        out_ms: i32,
        label: Option<String>,
        color: Option<String>,
    ) -> Result<()> {
        crate::deck_data::save_loop(&self.db, id, slot_index, in_ms, out_ms, label, color)
    }

    pub fn delete_track_loop(&self, id: &TrackId, slot_index: u8) -> Result<()> {
        crate::deck_data::delete_loop(&self.db, id, slot_index)
    }
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

    #[test]
    fn save_loop_preserves_label_color_when_omitted_on_update() {
        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/b.wav");
        store
            .upsert_file_track(
                &id,
                Path::new("/music/b.wav"),
                &TrackMetadata::default(),
                "1",
            )
            .unwrap();

        save_loop(
            &db,
            &id,
            0,
            0,
            1000,
            Some("intro".into()),
            Some("#ff00ff".into()),
        )
        .unwrap();
        save_loop(&db, &id, 0, 100, 2000, None, None).unwrap();

        let loops = list_loops(&db, &id).unwrap();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].in_ms, 100);
        assert_eq!(loops[0].out_ms, 2000);
        assert_eq!(loops[0].label.as_deref(), Some("intro"));
        assert_eq!(loops[0].color.as_deref(), Some("#ff00ff"));
    }
}
