//! Tempo/beat sync follow helpers for the engine control path.

use crate::pads::HOT_CUE_SLOT_COUNT;
use engine_api::{LoopRegion, PadMode, SyncMode};
use library_core::TrackId;

#[derive(Clone, Debug, Default)]
pub(crate) struct DeckControlState {
    pub sync_mode: SyncMode,
    pub bpm: Option<f64>,
    pub quantize: bool,
    pub pad_mode: PadMode,
    pub loop_roll_restore: Option<LoopRegion>,
    /// Library track id when the deck holds a library-backed (or id'd) load.
    pub track_id: Option<TrackId>,
    /// Runtime hot-cue positions (library hydrate + in-session save/delete).
    pub hot_cues: [Option<i32>; HOT_CUE_SLOT_COUNT],
}

impl DeckControlState {
    pub fn reset_for_load(&mut self, bpm: Option<f64>) {
        self.bpm = bpm.filter(|b| b.is_finite() && *b > 0.0);
        self.sync_mode = SyncMode::Off;
        self.loop_roll_restore = None;
        self.track_id = None;
        self.hot_cues = [None; HOT_CUE_SLOT_COUNT];
    }
}

/// Beat length in milliseconds for a given BPM.
fn beat_ms(bpm: f64) -> f64 {
    60_000.0 / bpm
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
}
