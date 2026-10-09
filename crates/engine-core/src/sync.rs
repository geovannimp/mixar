//! Tempo/beat sync follow helpers for the engine control path.

use crate::pads::{DEFAULT_PITCH_PAGE, HOT_CUE_SLOT_COUNT};
use engine_api::{LoopRegion, PadMode, SyncMode};
use library_core::TrackId;

#[derive(Clone, Debug)]
pub(crate) struct DeckControlState {
    pub sync_mode: SyncMode,
    pub bpm: Option<f64>,
    pub quantize: bool,
    pub pad_mode: PadMode,
    pub loop_roll_restore: Option<LoopRegion>,
    /// Pending manual Loop In (ms) before Loop Out completes the region.
    pub pending_loop_in_ms: Option<i32>,
    /// Library track id when the deck holds a library-backed (or id'd) load.
    pub track_id: Option<TrackId>,
    /// Filesystem path or stream URI for the loaded source.
    pub track_path: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub key: Option<String>,
    /// International Standard Recording Code when known at load time.
    pub isrc: Option<String>,
    /// Runtime hot-cue positions (library hydrate + in-session save/delete).
    pub hot_cues: [Option<i32>; HOT_CUE_SLOT_COUNT],
    /// Library sampler bank currently loaded onto this deck's pads.
    pub active_sampler_bank_id: Option<String>,
    /// Session key-shift offset in semitones (`-16..=16`; `0` = bypass).
    pub key_shift_semitones: f32,
    /// Keyboard / Key Shift semitone page (`1..=5`).
    pub pitch_page: u8,
    /// Hot-cue slot used as the Keyboard pad root.
    pub keyboard_root_hot_cue: u8,
    /// Key-shift offset latched before the first Keyboard pad press, restored on
    /// the last Keyboard pad release (Keyboard is momentary, not destructive).
    pub keyboard_restore_semitones: Option<f32>,
    /// Which Keyboard pads are currently held (momentary note bank).
    pub keyboard_held: [bool; 8],
}

impl Default for DeckControlState {
    fn default() -> Self {
        Self {
            sync_mode: SyncMode::Off,
            bpm: None,
            quantize: false,
            pad_mode: PadMode::HotCue,
            loop_roll_restore: None,
            pending_loop_in_ms: None,
            track_id: None,
            track_path: None,
            title: None,
            artist: None,
            album: None,
            key: None,
            isrc: None,
            hot_cues: [None; HOT_CUE_SLOT_COUNT],
            active_sampler_bank_id: None,
            key_shift_semitones: 0.0,
            pitch_page: DEFAULT_PITCH_PAGE,
            keyboard_root_hot_cue: 0,
            keyboard_restore_semitones: None,
            keyboard_held: [false; 8],
        }
    }
}

impl DeckControlState {
    pub fn clear_loaded_track(&mut self) {
        self.bpm = None;
        self.sync_mode = SyncMode::Off;
        self.loop_roll_restore = None;
        self.pending_loop_in_ms = None;
        self.track_id = None;
        self.track_path = None;
        self.title = None;
        self.artist = None;
        self.album = None;
        self.key = None;
        self.isrc = None;
        self.hot_cues = [None; HOT_CUE_SLOT_COUNT];
        self.key_shift_semitones = 0.0;
        self.pitch_page = DEFAULT_PITCH_PAGE;
        self.keyboard_root_hot_cue = 0;
        self.keyboard_restore_semitones = None;
        self.keyboard_held = [false; 8];
    }

    pub fn apply_source_load(&mut self, source: &library_core::AudioSource, track_id: TrackId) {
        self.apply_loaded_metadata(track_id, source.source_ref(), source.metadata());
    }

    pub fn apply_loaded_metadata(
        &mut self,
        track_id: TrackId,
        track_path: String,
        metadata: &library_core::TrackMetadata,
    ) {
        self.bpm = metadata.bpm.filter(|b| b.is_finite() && *b > 0.0);
        self.sync_mode = SyncMode::Off;
        self.loop_roll_restore = None;
        self.pending_loop_in_ms = None;
        self.track_id = Some(track_id);
        self.track_path = Some(track_path);
        self.title = non_empty_opt(metadata.title.clone());
        self.artist = non_empty_opt(metadata.artist.clone());
        self.album = non_empty_opt(metadata.album.clone());
        self.key = non_empty_opt(metadata.key.clone());
        self.isrc = non_empty_opt(metadata.isrc.clone());
        self.hot_cues = [None; HOT_CUE_SLOT_COUNT];
        // A newly loaded track must not inherit a stale session shift/page.
        self.key_shift_semitones = 0.0;
        self.pitch_page = DEFAULT_PITCH_PAGE;
        self.keyboard_root_hot_cue = 0;
        self.keyboard_restore_semitones = None;
        self.keyboard_held = [false; 8];
    }
}

fn non_empty_opt(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.is_empty())
}

/// Beat length in milliseconds for a given BPM.
fn beat_ms(bpm: f64) -> f64 {
    60_000.0 / bpm
}

/// Require a finite BPM > 0 for loop / beat-timed ops.
pub(crate) fn require_positive_bpm(bpm: Option<f64>, op: &str) -> anyhow::Result<f64> {
    match bpm {
        Some(b) if b.is_finite() && b > 0.0 => Ok(b),
        _ => Err(anyhow::anyhow!("Track BPM is required for {op}.")),
    }
}

/// Snap media time to the nearest beat when quantize is on. Stays in ms end-to-end.
pub(crate) fn snap_ms(ms: i32, bpm: Option<f64>, quantize: bool) -> i32 {
    if !quantize {
        return ms;
    }
    let Some(bpm) = bpm else {
        return ms;
    };
    if bpm <= 0.0 {
        return ms;
    }
    let beat = beat_ms(bpm);
    ((f64::from(ms) / beat).round() * beat).round() as i32
}

pub(crate) fn target_sync_speed(master_bpm: f64, master_speed: f32, slave_bpm: f64) -> f32 {
    let master_effective = master_bpm * f64::from(master_speed);
    ((master_effective / slave_bpm) as f32).clamp(0.5, 2.0)
}

pub(crate) fn beat_align_target(
    master_pos_ms: i32,
    slave_pos_ms: i32,
    duration_ms: i32,
    master_bpm: f64,
    slave_bpm: f64,
    quantize: bool,
) -> i32 {
    let master_pos = f64::from(master_pos_ms);
    let slave_pos = f64::from(slave_pos_ms);
    let duration = f64::from(duration_ms);
    let master_beat = beat_ms(master_bpm);
    let slave_beat = beat_ms(slave_bpm);
    let master_phase = master_pos % master_beat;

    let slave_beat_index = (slave_pos / slave_beat).floor();
    let mut target = slave_beat_index * slave_beat + master_phase;
    if target + slave_beat * 0.5 < slave_pos {
        target += slave_beat;
    }

    snap_ms(
        target.min(duration).round() as i32,
        Some(slave_bpm),
        quantize,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_sync_speed_matches_master_effective_tempo() {
        // Master 120 BPM at 1.0 → slave 100 BPM needs 1.2×
        assert!((target_sync_speed(120.0, 1.0, 100.0) - 1.2).abs() < f32::EPSILON);
    }

    #[test]
    fn snap_ms_without_quantize_passes_through_including_negatives() {
        assert_eq!(snap_ms(500, Some(120.0), false), 500);
        assert_eq!(snap_ms(-100, None, false), -100);
    }

    #[test]
    fn snap_ms_quantizes_to_nearest_beat_in_ms() {
        // 120 BPM → 500 ms per beat; 620 → 500, 760 → 1000
        assert_eq!(snap_ms(620, Some(120.0), true), 500);
        assert_eq!(snap_ms(760, Some(120.0), true), 1000);
    }

    #[test]
    fn require_positive_bpm_rejects_missing_zero_and_non_finite() {
        assert!(require_positive_bpm(Some(128.0), "auto loop").is_ok());
        assert!(require_positive_bpm(None, "auto loop").is_err());
        assert!(require_positive_bpm(Some(0.0), "auto loop").is_err());
        assert!(require_positive_bpm(Some(-1.0), "loop in").is_err());
        assert!(require_positive_bpm(Some(f64::NAN), "loop out").is_err());
        assert!(require_positive_bpm(Some(f64::INFINITY), "auto loop").is_err());
        let err = require_positive_bpm(None, "loop in")
            .unwrap_err()
            .to_string();
        assert!(err.contains("loop in"));
    }
}
