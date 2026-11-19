//! Per-file Serato adoption state and lazy backfill (#743).
//!
//! A file track adopts the vendor metadata #[742](https://github.com/geovannimp/mixar/issues/742)
//! reads exactly once. The decision is stored on the row
//! ([`tracks::Model::serato_imported_at`] plus a mask of adopted kinds), so a
//! later reload, rescan or forced analysis cannot resurrect cues or loops the
//! user deleted, and no second import path can bypass that guard.
//!
//! The rules, in one place:
//!
//! - **Once.** A marked row is never read again for adoption, even when all its
//!   cues are gone or the file is missing. A deleted and recreated track is a
//!   new row, so it gets a new adoption opportunity.
//! - **Non-destructive.** Occupied hot-cue and loop slots, an existing grid and
//!   an existing (or editor-owned) colour are kept verbatim; adoption fills
//!   vacancies only. DB-only cue slots 8–15 always survive.
//! - **Atomic.** Mapped rows and the marker share one `BEGIN IMMEDIATE`
//!   transaction that re-reads the row first, and the marker is written last.
//! - **Honest about failure.** A supported readable file with no Serato frames
//!   is adopted with zero kinds. An I/O failure, an incomplete scan or an
//!   unsupported container leaves the row unmarked, so a later touch retries.
//!
//! The marker is library-internal state for the write-back work in #548: it is
//! not exposed through FRB. Writes of the vendor frames stay in #548; adoption
//! never writes to the file.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use library_core::{LibraryError, Result, TrackId};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use crate::beat_grid;
use crate::embedded_metadata;
use crate::entity::{tracks, TrackEntity};
use crate::{deck_data, model, unix_secs_string, LibraryManager};

/// The file carried readable hot cues (Serato markers descriptor).
pub const SERATO_KIND_CUES: u8 = 0x01;
/// The file carried readable saved loops (Serato markers descriptor).
pub const SERATO_KIND_LOOPS: u8 = 0x02;
/// The file carried a readable native beat grid.
pub const SERATO_KIND_GRID: u8 = 0x04;
/// The file carried an exact track colour.
pub const SERATO_KIND_COLOUR: u8 = 0x08;

/// Every bit this build gives a meaning. A stored mask with another bit set is
/// corrupt, and is reported rather than silently ignored.
const KNOWN_KINDS: u8 =
    SERATO_KIND_CUES | SERATO_KIND_LOOPS | SERATO_KIND_GRID | SERATO_KIND_COLOUR;

/// Stored adoption state of one file track.
///
/// `imported_at` is `None` until a successful adoption; `adopted_kinds` is the
/// mask of Serato kinds that adoption understood from the file (`0` is a real
/// value: a readable supported file with no vendor frames).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeratoAdoptionState {
    /// Unix-seconds string of the successful adoption.
    pub imported_at: Option<String>,
    /// Adopted kind bits (`SERATO_KIND_*`); `0` before adoption and for a
    /// successfully adopted file without Serato frames.
    pub adopted_kinds: u8,
}

/// Outcome of one discovery read of the vendor metadata.
///
/// The reader runs during discovery, before the library lock is taken; this
/// keeps a failure distinguishable from a successful read with no frames, so a
/// failed read never stores an adoption marker.
#[derive(Clone, Debug)]
pub(crate) enum VendorRead {
    /// The reader ran to completion (the scan itself may still be incomplete).
    Payload(codec::SeratoPayload),
    /// File I/O or the tag walk failed; the file stays unadopted.
    Failed(String),
}

impl VendorRead {
    pub(crate) fn payload(&self) -> Option<&codec::SeratoPayload> {
        match self {
            Self::Payload(payload) => Some(payload),
            Self::Failed(_) => None,
        }
    }

    pub(crate) fn failure(&self) -> Option<&str> {
        match self {
            Self::Payload(_) => None,
            Self::Failed(err) => Some(err),
        }
    }
}

/// What one adoption attempt did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ApplyOutcome {
    /// Marker written with this mask.
    Adopted(u8),
    /// The row was adopted by someone else first (or by an earlier touch).
    AlreadyAdopted,
    /// Not a file track, or a container the reader does not understand.
    NotApplicable,
    /// A supported container whose tag could not be fully trusted, so the row
    /// stays unmarked and a later touch retries.
    Deferred(&'static str),
}

/// Split a payload into "adopt this", "not our container" and "do not trust it".
enum Candidate<'a> {
    Adopt(&'a codec::SeratoPayload),
    Unsupported,
    Incomplete,
}

fn inspect(payload: &codec::SeratoPayload) -> Candidate<'_> {
    if payload.container.is_none() {
        Candidate::Unsupported
    } else if !payload.scan_complete {
        // A skipped frame or an exceeded bound means absence of a frame proves
        // nothing; marking now could permanently freeze a partial read.
        Candidate::Incomplete
    } else {
        Candidate::Adopt(payload)
    }
}

/// Kinds the file actually made readable, derived from the reader's per-frame
/// provenance — never from how many DB rows ended up filled, because a slot
/// occupied before adoption says nothing about whether the file carried it.
pub(crate) fn eligible_kinds(payload: &codec::SeratoPayload) -> u8 {
    use codec::serato::{FrameStatus, SeratoFrameKind};

    let status = |kind: SeratoFrameKind| {
        payload
            .frames
            .iter()
            .find(|frame| frame.kind == kind)
            .map(|frame| frame.status)
    };
    let valid = |kind: SeratoFrameKind| matches!(status(kind), Some(FrameStatus::Valid));

    let mut kinds = 0;
    // Both cues and loops come from the markers descriptors; a legacy descriptor
    // is enough even when the Markers2 frame is unreadable, and vice versa.
    if valid(SeratoFrameKind::Markers2) || valid(SeratoFrameKind::MarkersLegacy) {
        kinds |= SERATO_KIND_CUES | SERATO_KIND_LOOPS;
    }
    if valid(SeratoFrameKind::BeatGrid) {
        kinds |= SERATO_KIND_GRID;
    }
    // Colour comes from Markers2's `COLOR` record only; an absent record is not
    // an understood kind, and Mixed In Key has no colour.
    if payload.color_rgb.is_some() {
        kinds |= SERATO_KIND_COLOUR;
    }
    kinds
}

#[cfg(test)]
mod tests;

fn invalid_mask() -> LibraryError {
    LibraryError::Backend {
        backend: "library",
        message: "invalid Serato adoption mask".into(),
    }
}

// Test-only count of adoption reader calls, so "one attempt per entry point"
// is asserted by behavior instead of by log text. Thread-scoped so tests
// running in parallel cannot perturb the measurement.
#[cfg(test)]
thread_local! {
    static READ_ATTEMPTS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// The adapter's single reader-call site.
///
/// The error is stringified so this module does not have to name the reader's
/// `anyhow` type; the message is all the caller needs.
fn read_vendor(path: &Path) -> std::result::Result<codec::SeratoPayload, String> {
    #[cfg(test)]
    READ_ATTEMPTS.with(|counter| {
        if let Some(attempts) = counter.get() {
            counter.set(Some(attempts + 1));
        }
    });
    codec::read_file_serato(path).map_err(|err| err.to_string())
}

/// Begin counting adoption reads on the calling thread.
#[cfg(test)]
pub(crate) fn start_counting() {
    READ_ATTEMPTS.with(|counter| counter.set(Some(0)));
}

/// Stop counting and return how many adoption reads happened meanwhile.
#[cfg(test)]
pub(crate) fn stop_counting() -> usize {
    READ_ATTEMPTS.with(|counter| counter.replace(None).unwrap_or(0))
}

fn state_from_row(row: &tracks::Model) -> Result<SeratoAdoptionState> {
    let adopted_kinds = match row.serato_adopted_kinds {
        None => 0,
        Some(raw) => {
            let Ok(raw) = u8::try_from(raw) else {
                return Err(invalid_mask());
            };
            if raw & !KNOWN_KINDS != 0 {
                return Err(invalid_mask());
            }
            raw
        }
    };
    Ok(SeratoAdoptionState {
        imported_at: row.serato_imported_at.clone(),
        adopted_kinds,
    })
}

fn read_error(id: &TrackId, path: &Path, err: impl std::fmt::Display) -> LibraryError {
    LibraryError::Backend {
        backend: "library",
        message: format!("read Serato metadata for {id} at {}: {err}", path.display()),
    }
}

/// Where a track's adoption has to read from, captured without the DB guard.
struct AdoptionTarget {
    path: PathBuf,
    marked: bool,
}

impl LibraryManager {
    /// Stored adoption state of any track row.
    ///
    /// Unknown ids are [`LibraryError::NotFound`]. A `NULL` mask maps to `0`;
    /// a mask with unknown or negative bits is corrupt and reports the
    /// [`LibraryError::Backend`] `invalid Serato adoption mask`.
    pub fn serato_adoption_state(&self, id: &TrackId) -> Result<SeratoAdoptionState> {
        let row = TrackEntity::find_by_id(id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(crate::db::db_err)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;
        state_from_row(&row)
    }

    /// Adopt the file's vendor metadata if this row has not adopted it yet.
    ///
    /// Strict: an unreadable supported file returns the reader error, and an
    /// incomplete scan is an error too, so a first edit cannot run against a
    /// marker that a later touch would write. Unsupported containers and stream
    /// tracks are not applicable and return `Ok`.
    pub fn ensure_serato_adopted(&self, id: &TrackId) -> Result<()> {
        let Some(target) = self.adoption_target(id)? else {
            return Ok(());
        };
        if target.marked {
            return Ok(());
        }
        let payload = read_vendor(&target.path).map_err(|err| read_error(id, &target.path, err))?;
        match self.apply_payload(id, &target.path, &payload)? {
            ApplyOutcome::Deferred(reason) => Err(LibraryError::Backend {
                backend: "library",
                message: format!("Serato adoption deferred: {reason}"),
            }),
            _ => Ok(()),
        }
    }

    /// Same as [`Self::ensure_serato_adopted`], but performs the reader I/O
    /// outside the manager lock and serializes it against tag rewrites with the
    /// shared tag-write gate.
    ///
    /// The DB guard is never held across the read: the target is copied out,
    /// the lock and gate are released, then the manager is reacquired to apply.
    pub fn ensure_serato_adopted_off_mutex(library: &Mutex<Self>, id: &TrackId) -> Result<()> {
        let (target, gate) = {
            let lib = Self::lock_library(library)?;
            let Some(target) = lib.adoption_target(id)? else {
                return Ok(());
            };
            if target.marked {
                return Ok(());
            }
            (target, lib.tag_write_gate())
        };

        let payload = {
            // Same gate as the deferred tag writers: a file being rewritten must
            // not be read as vendor metadata halfway through the rewrite.
            let _gate = gate.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            read_vendor(&target.path)
        }
        .map_err(|err| read_error(id, &target.path, err))?;

        let lib = Self::lock_library(library)?;
        match lib.apply_payload(id, &target.path, &payload)? {
            ApplyOutcome::Deferred(reason) => Err(LibraryError::Backend {
                backend: "library",
                message: format!("Serato adoption deferred: {reason}"),
            }),
            _ => Ok(()),
        }
    }

    /// Optional adoption for import/rescan/load/analysis: a deferred read is
    /// logged and the base operation continues without a marker.
    pub(crate) fn adopt_serato_off_mutex(library: &Mutex<Self>, id: &TrackId) {
        if let Err(err) = Self::ensure_serato_adopted_off_mutex(library, id) {
            tracing::warn!(
                track_id = %id,
                error = %err,
                "Serato adoption deferred"
            );
        }
    }

    fn adoption_target(&self, id: &TrackId) -> Result<Option<AdoptionTarget>> {
        let row = TrackEntity::find_by_id(id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(crate::db::db_err)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;
        if row.source_type != "file" {
            return Ok(None);
        }
        Ok(Some(AdoptionTarget {
            path: PathBuf::from(&row.source_ref),
            marked: row.serato_imported_at.is_some(),
        }))
    }

    fn apply_payload(
        &self,
        id: &TrackId,
        path: &Path,
        payload: &codec::SeratoPayload,
    ) -> Result<ApplyOutcome> {
        self.db
            .transaction(|conn| apply_payload_in_tx(conn, id, path, payload))
    }
}

/// Adoption core, shared by the transaction-owning entries and by discovery
/// (which already runs inside its own transaction).
///
/// The row is re-read here so a competing adoption, a deletion or a path change
/// cannot be overwritten with data read against the old row. A DB error rolls
/// back every mapped row and the marker, so a later touch retries.
pub(crate) fn apply_payload_in_tx(
    conn: &sea_orm::DatabaseConnection,
    id: &TrackId,
    path: &Path,
    payload: &codec::SeratoPayload,
) -> Result<ApplyOutcome> {
    let payload = match inspect(payload) {
        Candidate::Adopt(payload) => payload,
        Candidate::Unsupported => return Ok(ApplyOutcome::NotApplicable),
        Candidate::Incomplete => return Ok(ApplyOutcome::Deferred("tag scan incomplete")),
    };

    let Some(row) = TrackEntity::find_by_id(id.as_str())
        .one(conn)
        .map_err(crate::db::db_err)?
    else {
        return Err(LibraryError::NotFound(id.to_string()));
    };
    if row.source_type != "file" {
        return Ok(ApplyOutcome::NotApplicable);
    }
    if row.serato_imported_at.is_some() {
        return Ok(ApplyOutcome::AlreadyAdopted);
    }
    // Same rule as `ensure_track_still_at_path`: never apply a payload read for
    // another file (or a track that was deleted and recreated elsewhere).
    if Path::new(&row.source_ref) != path {
        return Err(LibraryError::NotFound(id.to_string()));
    }

    let stored = model::track_metadata(&row);
    let plan = embedded_metadata::build_import_plan(&stored, payload);

    for cue in &plan.cues {
        let Some(position_ms) = embedded_metadata::cue_position_ms(cue.position_ms) else {
            continue;
        };
        deck_data::fill_hot_cue_tx(
            conn,
            id,
            cue.slot,
            position_ms,
            cue.color_rgb.map(embedded_metadata::hex_color),
            cue.label.clone(),
        )?;
    }
    for saved in &plan.loops {
        deck_data::fill_loop_tx(conn, id, saved)?;
    }

    // Fill-only: an existing grid — imported, analyzed or manual — is kept with
    // every segment verbatim, and its BPM mirror owns `tracks.bpm`.
    if let Some(grid) = plan.grid.as_ref() {
        if beat_grid::get_track_beat_grid_tx(conn, id)?.is_none() {
            beat_grid::write_track_beat_grid_tx(conn, id, grid)?;
        }
    }

    // Colour is one value (label + exact RGB). Fill it only when the row stores
    // none and no metadata edit owns the row: an edited `NULL` colour is a
    // deliberate clear, which this conservative authority keeps.
    let stores_colour = row.color_label.is_some() || row.color_rgb.is_some();
    if !stores_colour && row.metadata_edited != Some(true) {
        let (label, rgb) = (plan.metadata.color_label, plan.metadata.color_rgb);
        if label.is_some() || rgb.is_some() {
            tracks::ActiveModel {
                id: Set(id.as_str().to_string()),
                color_label: Set(label.map(|value| value.as_str().to_string())),
                color_rgb: Set(rgb.map(|value| value as i32)),
                ..Default::default()
            }
            .update(conn)
            .map_err(crate::db::db_err)?;
        }
    }

    let kinds = eligible_kinds(payload);
    // Last: the marker commits only with every mapped row above it.
    tracks::ActiveModel {
        id: Set(id.as_str().to_string()),
        serato_imported_at: Set(Some(unix_secs_string())),
        serato_adopted_kinds: Set(Some(i32::from(kinds))),
        ..Default::default()
    }
    .update(conn)
    .map_err(crate::db::db_err)?;
    Ok(ApplyOutcome::Adopted(kinds))
}
