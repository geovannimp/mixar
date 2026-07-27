//! Tempo/beat sync follow helpers for the engine control path.

use engine_api::SyncMode;

#[derive(Clone, Debug, Default)]
pub(crate) struct DeckControlState {
    pub sync_mode: SyncMode,
    pub bpm: Option<f64>,
    pub quantize: bool,
}

impl DeckControlState {
    pub fn reset_for_load(&mut self, bpm: Option<f64>) {
        self.bpm = bpm.filter(|b| b.is_finite() && *b > 0.0);
        self.sync_mode = SyncMode::Off;
    }
}

pub(crate) fn snap_secs(secs: f64, bpm: Option<f64>, quantize: bool) -> f64 {
    if !quantize {
        return secs.max(0.0);
    }
    let Some(bpm) = bpm else {
        return secs.max(0.0);
    };
    if bpm <= 0.0 {
        return secs.max(0.0);
    }
    let beat = 60.0 / bpm;
    ((secs / beat).round() * beat).max(0.0)
}

pub(crate) fn target_sync_speed(master_bpm: f64, master_speed: f32, slave_bpm: f64) -> f32 {
    let master_effective = master_bpm * f64::from(master_speed);
    ((master_effective / slave_bpm) as f32).clamp(0.5, 2.0)
}

pub(crate) fn beat_align_target(
    master_pos: f64,
    slave_pos: f64,
    duration: f64,
    master_bpm: f64,
    slave_bpm: f64,
    quantize: bool,
) -> f64 {
    let master_beat = 60.0 / master_bpm;
    let slave_beat = 60.0 / slave_bpm;
    let master_phase = master_pos % master_beat;

    let slave_beat_index = (slave_pos / slave_beat).floor();
    let mut target = slave_beat_index * slave_beat + master_phase;
    if target + slave_beat * 0.5 < slave_pos {
        target += slave_beat;
    }

    snap_secs(target.min(duration), Some(slave_bpm), quantize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_sync_speed_matches_master_effective_tempo() {
        // Master 120 BPM at 1.0 → slave 100 BPM needs 1.2×
        assert!((target_sync_speed(120.0, 1.0, 100.0) - 1.2).abs() < f32::EPSILON);
    }
}
