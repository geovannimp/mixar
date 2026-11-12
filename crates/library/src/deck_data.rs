//! Hot cue and loop persistence for deck performance data.

use sea_orm::sea_query::OnConflict;
use sea_orm::ColumnTrait;
use sea_orm::{EntityTrait, QueryFilter, Set};
use std::collections::HashSet;

use library_core::{Library, LibraryError, LoopImport, Result, TrackId};

use crate::db::{self, Db};
use crate::entity::{track_hot_cue, track_loop, TrackHotCueEntity, TrackLoopEntity};
use crate::{tags, unix_millis_string, unix_secs_string, LibraryManager, TrackMetadataEdit};

/// Maximum automatic loop rows kept per track (rekordbox parity).
pub const MAX_AUTO_LOOPS: usize = 10;

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
    pub auto_saved: bool,
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
            auto_saved: row.auto_saved,
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
    upsert_loop(
        db,
        track_id,
        &LoopImport {
            slot: slot_index,
            in_ms,
            out_ms,
            label,
            color,
        },
        true,
        false,
    )
}

/// Write a loop imported from another library, exactly as it arrived.
///
/// Unlike [`save_loop`], a `None` label or colour clears the column instead of
/// keeping the target's previous value: an import is authoritative, and a
/// stale label from an earlier import of a different library would survive
/// otherwise.
pub fn import_loop(db: &Db, track_id: &TrackId, saved: &LoopImport) -> Result<()> {
    upsert_loop(db, track_id, saved, false, false)
}

/// Least-recently-updated automatic loop, breaking exact timestamp ties by slot.
fn oldest_auto_slot(rows: &[&track_loop::Model]) -> Option<i32> {
    rows.iter()
        .min_by(|a, b| {
            a.updated_at
                .cmp(&b.updated_at)
                .then(a.slot_index.cmp(&b.slot_index))
        })
        .map(|row| row.slot_index)
}

/// Remember a loop the deck just activated. At most [`MAX_AUTO_LOOPS`] auto
/// rows are kept; imported/explicit rows are never matched or evicted.
pub fn auto_save_loop(db: &Db, track_id: &TrackId, in_ms: i32, out_ms: i32) -> Result<()> {
    if out_ms <= in_ms {
        return Err(LibraryError::Backend {
            backend: "deck_data",
            message: "loop out must be after loop in".into(),
        });
    }

    let rows = TrackLoopEntity::find()
        .filter(track_loop::Column::TrackId.eq(track_id.as_str()))
        .all(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    let new_span = i64::from(out_ms) - i64::from(in_ms);
    let mut best: Option<(i32, i64)> = None;
    for row in rows.iter().filter(|row| row.auto_saved) {
        let overlap = (out_ms.min(row.out_ms) - in_ms.max(row.in_ms)).max(0) as i64;
        let row_span = i64::from(row.out_ms) - i64::from(row.in_ms);
        let min_span = new_span.min(row_span);
        if min_span > 0 && overlap * 2 >= min_span {
            let better = match best {
                Some((_, best_overlap)) => overlap > best_overlap,
                None => true,
            };
            if better {
                best = Some((row.slot_index, overlap));
            }
        }
    }

    let slot = if let Some((slot, _)) = best {
        slot
    } else {
        let auto_rows: Vec<&track_loop::Model> = rows.iter().filter(|row| row.auto_saved).collect();
        if auto_rows.len() >= MAX_AUTO_LOOPS {
            oldest_auto_slot(&auto_rows).expect("auto rows is non-empty")
        } else {
            let used: HashSet<i32> = rows.iter().map(|row| row.slot_index).collect();
            match (0..=15).find(|slot| !used.contains(slot)) {
                Some(slot) => slot,
                // Imported/explicit rows can occupy every slot even when the
                // auto cap is not reached. Reclaim the oldest auto row instead
                // of silently disabling loop memory for this track.
                None => oldest_auto_slot(&auto_rows).ok_or_else(|| LibraryError::Backend {
                    backend: "deck_data",
                    message: "loop memory full".into(),
                })?,
            }
        }
    };

    upsert_loop(
        db,
        track_id,
        &LoopImport {
            slot: slot as u8,
            in_ms,
            out_ms,
            label: None,
            color: None,
        },
        true,
        true,
    )
}

fn upsert_loop(
    db: &Db,
    track_id: &TrackId,
    saved: &LoopImport,
    preserve: bool,
    auto_saved: bool,
) -> Result<()> {
    let (slot_index, in_ms, out_ms) = (saved.slot, saved.in_ms, saved.out_ms);
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

    // A deck edit preserves label/color when the caller omits them (None); an
    // import writes the imported value verbatim.
    let (label, color) = if preserve {
        let existing = TrackLoopEntity::find()
            .filter(track_loop::Column::TrackId.eq(track_id.as_str()))
            .filter(track_loop::Column::SlotIndex.eq(i32::from(slot_index)))
            .one(db.conn()?.as_connection())
            .map_err(db::db_err)?;
        (
            saved
                .label
                .clone()
                .or_else(|| existing.as_ref().and_then(|row| row.label.clone())),
            saved
                .color
                .clone()
                .or_else(|| existing.as_ref().and_then(|row| row.color.clone())),
        )
    } else {
        (saved.label.clone(), saved.color.clone())
    };

    TrackLoopEntity::insert(track_loop::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        slot_index: Set(i32::from(slot_index)),
        in_ms: Set(in_ms),
        out_ms: Set(out_ms),
        label: Set(label),
        color: Set(color),
        auto_saved: Set(auto_saved),
        updated_at: Set(unix_millis_string()),
    })
    .on_conflict(
        OnConflict::columns([track_loop::Column::TrackId, track_loop::Column::SlotIndex])
            .update_columns([
                track_loop::Column::InMs,
                track_loop::Column::OutMs,
                track_loop::Column::Label,
                track_loop::Column::Color,
                track_loop::Column::AutoSaved,
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
            // 0 means "unrated" in every source that stores a numeric rating.
            rating: edit.rating.filter(|rating| (1..=5).contains(rating)),
            comment: normalize_optional_text(edit.comment),
            year: edit.year.filter(|year| *year > 0),
            track_number: edit.track_number.filter(|number| *number > 0),
            disc_number: edit.disc_number.filter(|number| *number > 0),
            color_label: edit.color_label,
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

    pub fn auto_save_track_loop(&self, id: &TrackId, in_ms: i32, out_ms: i32) -> Result<()> {
        crate::deck_data::auto_save_loop(&self.db, id, in_ms, out_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::store::Store;
    use library_core::TrackMetadata;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    use std::path::Path;

    fn seed_track(db: &Db, path: &str) -> TrackId {
        let store = Store::new(db);
        let id = TrackId::new(path);
        store
            .upsert_file_track(&id, Path::new(path), &TrackMetadata::default(), "1")
            .unwrap();
        id
    }

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
    fn imported_loop_overwrites_label_and_color_verbatim() {
        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/import.wav");
        store
            .upsert_file_track(
                &id,
                Path::new("/music/import.wav"),
                &TrackMetadata::default(),
                "1",
            )
            .unwrap();

        save_loop(
            &db,
            &id,
            0,
            0,
            1_000,
            Some("Drop".into()),
            Some("#FF0000".into()),
        )
        .unwrap();
        // A deck edit with None keeps what is there...
        save_loop(&db, &id, 0, 0, 2_000, None, None).unwrap();
        let kept = &list_loops(&db, &id).unwrap()[0];
        assert_eq!(kept.label.as_deref(), Some("Drop"));

        // ...an import with None clears it, because the source has no label.
        import_loop(
            &db,
            &id,
            &LoopImport {
                slot: 0,
                in_ms: 0,
                out_ms: 2_000,
                label: None,
                color: None,
            },
        )
        .unwrap();
        let cleared = &list_loops(&db, &id).unwrap()[0];
        assert_eq!(cleared.label, None);
        assert_eq!(cleared.color, None);
        assert_eq!(cleared.out_ms, 2_000);
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

    #[test]
    fn auto_save_allocates_lowest_free_slot_and_marks_auto() {
        let db = db::open_in_memory().unwrap();
        let id = seed_track(&db, "/music/auto-slot.wav");
        save_loop(&db, &id, 0, 0, 1000, None, None).unwrap();

        auto_save_loop(&db, &id, 1000, 3000).unwrap();

        let loops = list_loops(&db, &id).unwrap();
        assert_eq!(loops.len(), 2);
        let auto = loops.iter().find(|row| row.auto_saved).expect("auto row");
        assert_eq!(auto.slot_index, 1);
        assert_eq!((auto.in_ms, auto.out_ms), (1000, 3000));
        let manual = loops
            .iter()
            .find(|row| !row.auto_saved)
            .expect("manual row");
        assert_eq!(manual.slot_index, 0);
    }

    #[test]
    fn auto_save_updates_overlapping_auto_loop_in_place() {
        let db = db::open_in_memory().unwrap();
        let id = seed_track(&db, "/music/auto-resize.wav");

        auto_save_loop(&db, &id, 1000, 3000).unwrap();
        auto_save_loop(&db, &id, 1000, 5000).unwrap();

        let loops = list_loops(&db, &id).unwrap();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].slot_index, 0);
        assert_eq!(loops[0].out_ms, 5000);
        assert!(loops[0].auto_saved);
    }

    #[test]
    fn auto_save_never_updates_an_imported_loop() {
        let db = db::open_in_memory().unwrap();
        let id = seed_track(&db, "/music/auto-imported.wav");
        import_loop(
            &db,
            &id,
            &LoopImport {
                slot: 0,
                in_ms: 1000,
                out_ms: 3000,
                label: Some("Drop".into()),
                color: Some("#FF0000".into()),
            },
        )
        .unwrap();

        auto_save_loop(&db, &id, 1000, 3000).unwrap();

        let loops = list_loops(&db, &id).unwrap();
        assert_eq!(loops.len(), 2);
        let imported = loops.iter().find(|row| !row.auto_saved).unwrap();
        assert_eq!(imported.slot_index, 0);
        assert_eq!(imported.label.as_deref(), Some("Drop"));
        let auto = loops.iter().find(|row| row.auto_saved).unwrap();
        assert_eq!(auto.slot_index, 1);
    }

    #[test]
    fn auto_save_keeps_at_most_ten_auto_rows_and_replaces_the_oldest() {
        let db = db::open_in_memory().unwrap();
        let id = seed_track(&db, "/music/auto-cap.wav");
        for i in 0..10 {
            auto_save_loop(&db, &id, i * 1000, i * 1000 + 500).unwrap();
        }

        {
            use sea_orm::EntityName;
            let table = track_loop::Entity.table_name();
            let conn = db.conn().unwrap();
            for i in 0..10 {
                conn.as_connection()
                    .execute_raw(Statement::from_string(
                        DatabaseBackend::Sqlite,
                        format!("UPDATE {table} SET updated_at = '{i:02}' WHERE slot_index = {i}"),
                    ))
                    .unwrap();
            }
        }

        auto_save_loop(&db, &id, 100_000, 101_000).unwrap();

        let loops = list_loops(&db, &id).unwrap();
        assert_eq!(loops.len(), 10);
        let replaced = loops.iter().find(|row| row.slot_index == 0).unwrap();
        assert_eq!((replaced.in_ms, replaced.out_ms), (100_000, 101_000));
        assert!(replaced.auto_saved);
    }

    #[test]
    fn auto_save_rejects_inverted_region() {
        let db = db::open_in_memory().unwrap();
        let id = seed_track(&db, "/music/auto-invalid.wav");
        assert!(auto_save_loop(&db, &id, 3000, 1000).is_err());
    }
}
