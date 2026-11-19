//! Import embedded (Serato / Mixed In Key) performance metadata at discovery.
//!
//! The reader is [`codec::read_file_serato`]; this module decides when its
//! result becomes library rows and writes everything atomically:
//!
//! - **Once per row.** A new row imports everything the reader recovered and
//!   gets the adoption marker; an existing row without a marker backfills only
//!   its vacancies through [`crate::serato_adoption`], and a marked row is never
//!   re-read. That is what stops a deleted cue from resurrecting on a rescan.
//! - **All or nothing.** The creation check, the track row, every auxiliary row
//!   and the adoption marker share one [`Db::transaction`](crate::db::Db::transaction);
//!   a database error leaves no new track, cue, loop or grid behind.
//! - **Validated before writing.** Every projected row is range-checked first, so
//!   an unusable individual record is dropped instead of failing the import
//!   halfway through the transaction.

use std::path::Path;

use library_core::{ColorLabel, ImportedTrack, Library, LibraryError, Result, TrackMetadata};
use sea_orm::EntityTrait;

use crate::beat_grid::{self, BeatSegment, CanonicalBeatGrid, SOURCE_IMPORT};
use crate::entity::TrackEntity;
use crate::serato_adoption::{self, ApplyOutcome, VendorRead};
use crate::{deck_data, model, store, tags, unix_secs_string, LibraryManager};

/// Projected rows for a newly discovered track.
///
/// Also the single projection adoption (#743) reuses, so discovery and lazy
/// backfill can never disagree about a cue, a loop, the grid or the colour.
pub(crate) struct ImportPlan {
    pub(crate) metadata: TrackMetadata,
    pub(crate) cues: Vec<codec::serato::SeratoCue>,
    pub(crate) loops: Vec<library_core::LoopImport>,
    pub(crate) grid: Option<CanonicalBeatGrid>,
}

impl LibraryManager {
    /// Persist a discovered file: ordinary metadata for every row, embedded
    /// performance data for a new row, and the once-only Serato adoption for a
    /// row that has not adopted it yet. Returns the authoritative stored source
    /// and whether it was created.
    ///
    /// Existing rows refresh ordinary metadata (plus the user-owned and
    /// raw-colour preservation that every refresh needs). One deliberate
    /// exception keeps an imported value instead of erasing it: when the fresh
    /// tags carry no key, the key already on the row survives, because it may
    /// have come from a vendor frame and a missing tag is not evidence of
    /// removal. `force` analysis and the editor can still change or clear it.
    ///
    /// Adoption itself is never re-run once the row carries its marker, so a
    /// rescan cannot resurrect a cue the user deleted.
    pub(crate) fn persist_discovered_file(
        &self,
        path: &Path,
        metadata: &TrackMetadata,
        vendor: &VendorRead,
    ) -> Result<ImportedTrack> {
        store::validate_color_rgb(metadata)?;
        let id = Self::track_id_for(path);
        let now = unix_secs_string();
        let created = self.db.transaction(|conn| {
            let existing = TrackEntity::find_by_id(id.as_str())
                .one(conn)
                .map_err(crate::db::db_err)?;
            let created = match existing {
                None => {
                    let plan = match vendor.payload() {
                        Some(payload) => build_import_plan(metadata, payload),
                        None => build_import_plan(metadata, &codec::SeratoPayload::default()),
                    };
                    persist_new_track(conn, &id, path, &plan, &now)?;
                    true
                }
                Some(row) => {
                    let mut merged = metadata.clone();
                    let stored = model::track_metadata(&row);
                    tags::merge_user_metadata(
                        &mut merged,
                        &stored,
                        row.metadata_edited.unwrap_or(false),
                    );
                    // An ordinary tag key wins over everything. When the tags
                    // carry none, the key the row already has survives: it may
                    // have been imported from a vendor frame that existing rows
                    // deliberately do not re-adopt, so a missing (or newly
                    // unreadable) tag is not evidence of removal, and there is no
                    // provenance marker that could tell an imported key from a
                    // curated one. A key cleared in the editor stays cleared
                    // because the stored row is already empty here.
                    if merged
                        .key
                        .as_deref()
                        .is_none_or(|key| key.trim().is_empty())
                    {
                        if let Some(key) = nonblank_key(&stored) {
                            merged.key = Some(key);
                        }
                    }
                    store::upsert_file_track_tx(conn, &id, path, &merged, &now)?;
                    // A canonical grid owns the BPM mirror; a tag refresh must
                    // not move it back to the file's tag value.
                    if let Some(bpm) =
                        beat_grid::get_track_beat_grid_tx(conn, &id)?.and_then(|g| g.bpm())
                    {
                        beat_grid::set_track_bpm_tx(conn, &id, bpm)?;
                    }
                    false
                }
            };

            // One adoption path for both branches: for a new row it fills
            // nothing (the import above already wrote every row) and only
            // records the marker; for an existing unmarked row it backfills the
            // vacancies. An already adopted row is a no-op.
            match vendor.payload() {
                Some(payload) => {
                    if let ApplyOutcome::Deferred(reason) =
                        serato_adoption::apply_payload_in_tx(conn, &id, path, payload)?
                    {
                        tracing::warn!(
                            track_id = %id,
                            error = reason,
                            "Serato adoption deferred"
                        );
                    }
                }
                None => {
                    if let Some(error) = vendor.failure() {
                        tracing::warn!(
                            track_id = %id,
                            error = error,
                            "Serato adoption deferred"
                        );
                    }
                }
            }
            Ok(created)
        })?;

        // The stored row is authoritative: `tracks.bpm` may have been fixed by a
        // grid, and returning the input clone would hand callers a stale BPM.
        let source = self
            .get_track(&id)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;
        Ok(ImportedTrack { source, created })
    }
}

fn persist_new_track(
    conn: &sea_orm::DatabaseConnection,
    id: &library_core::TrackId,
    path: &Path,
    plan: &ImportPlan,
    now: &str,
) -> Result<()> {
    store::upsert_file_track_tx(conn, id, path, &plan.metadata, now)?;

    for cue in &plan.cues {
        // Filtered by `build_import_plan`; skipping here keeps this total without
        // ever clamping a position to a plausible-looking wrong one.
        let Some(position_ms) = cue_position_ms(cue.position_ms) else {
            continue;
        };
        deck_data::save_hot_cue_tx(
            conn,
            id,
            cue.slot,
            position_ms,
            None,
            cue.color_rgb.map(hex_color),
            cue.label.clone(),
        )?;
    }
    for import in &plan.loops {
        deck_data::import_loop_tx(conn, id, import)?;
    }
    if let Some(grid) = &plan.grid {
        beat_grid::write_track_beat_grid_tx(conn, id, grid)?;
    }
    Ok(())
}

/// Build the validated projection of `metadata` plus `payload`.
///
/// Everything that cannot be represented is dropped here, before any write, so a
/// partially importable payload never leaves a half-written track behind.
pub(crate) fn build_import_plan(
    metadata: &TrackMetadata,
    payload: &codec::SeratoPayload,
) -> ImportPlan {
    let grid = payload.beatgrid.as_ref().and_then(canonical_grid);
    let mut projected = metadata.clone();

    // Mixed In Key's key is a fallback only: an ordinary tag key always wins
    // (a blank tag is not a key). Normalizing stays the library's job.
    let tag_key_missing = projected
        .key
        .as_deref()
        .is_none_or(|key| key.trim().is_empty());
    if tag_key_missing {
        if let Some(mik_key) = payload.mik_key.as_deref() {
            let normalized = tags::normalize_key_notation(mik_key);
            if !normalized.trim().is_empty() {
                projected.key = Some(normalized);
            }
        }
    }

    // A valid native grid owns BPM; without one the ordinary file BPM stays.
    if let Some(bpm) = grid.as_ref().and_then(CanonicalBeatGrid::bpm) {
        projected.bpm = Some(bpm);
    }

    // Exact track colour: the vendor RGB wins, its exact-match label is derived,
    // and no RGB value is discarded (white is not "no colour").
    let valid_rgb = |rgb: Option<u32>| rgb.filter(|value| *value <= 0x00FF_FFFF);
    let (color_rgb, color_label) = match valid_rgb(payload.color_rgb) {
        Some(rgb) => (Some(rgb), ColorLabel::from_rgb(rgb)),
        None => match valid_rgb(projected.color_rgb) {
            Some(rgb) => (Some(rgb), ColorLabel::from_rgb(rgb)),
            None => (
                projected.color_label.map(ColorLabel::rgb),
                projected.color_label,
            ),
        },
    };
    projected.color_rgb = color_rgb;
    projected.color_label = color_label;

    let cues = payload
        .cues
        .iter()
        .filter(|cue| cue_position_ms(cue.position_ms).is_some())
        .cloned()
        .collect();
    let loops = payload.loops.iter().filter_map(loop_import).collect();

    ImportPlan {
        metadata: projected,
        cues,
        loops,
        grid,
    }
}

/// The stored key when it is non-blank.
///
/// Used by a refresh whose ordinary tags carry no key: the value on the row may
/// be an imported vendor key, and re-reading the vendor frame is not required
/// (it may be corrupt, absent or skipped by the scan bound) for the row to keep
/// what it already owns.
pub(crate) fn nonblank_key(metadata: &TrackMetadata) -> Option<String> {
    metadata
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string)
}

/// A cue position that survives the stored `INTEGER` column.
///
/// The single accepted range, shared by the import plan and the writer so one
/// copy can never drop what the other accepts.
pub(crate) fn cue_position_ms(position_ms: u32) -> Option<i32> {
    i32::try_from(position_ms).ok()
}

/// A savable loop, or `None` when its bounds cannot be stored.
///
/// The plan uses this as its loop predicate and the writer takes the resulting
/// value as-is, so the range rule lives in one place.
fn loop_import(saved: &codec::serato::SeratoLoop) -> Option<library_core::LoopImport> {
    if saved.out_ms <= saved.in_ms {
        return None;
    }
    Some(library_core::LoopImport {
        slot: saved.slot,
        in_ms: i32::try_from(saved.in_ms).ok()?,
        out_ms: i32::try_from(saved.out_ms).ok()?,
        label: saved.label.clone(),
        color: saved.color_rgb.map(hex_color),
    })
}

/// Exact uppercase `#RRGGBB`, the deck's existing cue/loop colour format.
pub(crate) fn hex_color(rgb: u32) -> String {
    format!("#{:06X}", rgb & 0x00FF_FFFF)
}

/// Convert a native grid into a canonical segmented grid.
///
/// Non-terminal markers carry `beats_to_next`, so each segment's tempo follows
/// from the *unrounded* source times; the terminal marker's BPM comes from its
/// wire float. Segment starts are rounded to whole milliseconds exactly once,
/// and the wire has no meter, so 4/4 with a global downbeat phase is used.
fn canonical_grid(grid: &codec::serato::SeratoBeatGrid) -> Option<CanonicalBeatGrid> {
    let markers = &grid.markers;
    if markers.is_empty() {
        return None;
    }
    let mut segments = Vec::with_capacity(markers.len());
    let mut start_beat_index: i64 = 0;
    for (index, marker) in markers.iter().enumerate() {
        let (position_seconds, beats_to_next, bpm) = match *marker {
            codec::serato::SeratoGridMarker::NonTerminal {
                position_seconds,
                beats_to_next,
            } => {
                let next = marker_seconds(markers.get(index + 1)?);
                let span = next - position_seconds;
                if span <= 0.0 {
                    return None;
                }
                (
                    position_seconds,
                    Some(i64::from(beats_to_next)),
                    60.0 * f64::from(beats_to_next) / span,
                )
            }
            codec::serato::SeratoGridMarker::Terminal {
                position_seconds,
                bpm,
            } => (position_seconds, None, bpm),
        };
        segments.push(BeatSegment {
            segment_index: index as i32,
            start_time_ms: (position_seconds * 1000.0).round() as i64,
            bpm,
            beats_per_bar: 4,
            beat_unit: 4,
            start_beat_index,
            beats_to_next,
            downbeat_offset_beats: ((4 - start_beat_index.rem_euclid(4)) % 4) as u8,
        });
        start_beat_index += beats_to_next.unwrap_or(0);
    }
    let grid = CanonicalBeatGrid {
        source: SOURCE_IMPORT.to_string(),
        covered_to_ms: None,
        segments,
    };
    // A grid outside the canonical constraints is omitted; its reader data stays.
    grid.validate().ok()?;
    Some(grid)
}

fn marker_seconds(marker: &codec::serato::SeratoGridMarker) -> f64 {
    match *marker {
        codec::serato::SeratoGridMarker::NonTerminal {
            position_seconds, ..
        }
        | codec::serato::SeratoGridMarker::Terminal {
            position_seconds, ..
        } => position_seconds,
    }
}
