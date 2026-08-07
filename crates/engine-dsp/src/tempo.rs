//! Tempo fader `0..1` ↔ playback ratio using track BPM ± `tempo_range` (BPM).

const FALLBACK_BPM: f64 = 120.0;

fn usable_bpm(track_bpm: Option<f64>) -> f64 {
    track_bpm
        .filter(|b| b.is_finite() && *b > 0.0)
        .unwrap_or(FALLBACK_BPM)
}

/// Position `0` = +range (faster), `0.5` = 1.0, `1` = −range (slower).
pub fn norm_to_playback_ratio(norm: f32, track_bpm: Option<f64>, tempo_range: f32) -> f32 {
    let b = usable_bpm(track_bpm);
    let range = f64::from(tempo_range.max(0.0));
    let n = f64::from(norm.clamp(0.0, 1.0));
    let effective = b + (0.5 - n) * 2.0 * range;
    (effective / b).max(0.01) as f32
}

/// Saturates outside ±tempo_range.
pub fn playback_ratio_to_norm(ratio: f32, track_bpm: Option<f64>, tempo_range: f32) -> f32 {
    let b = usable_bpm(track_bpm);
    let range = f64::from(tempo_range.max(1e-6));
    let effective = f64::from(ratio) * b;
    let n = 0.5 - (effective - b) / (2.0 * range);
    n.clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tempo_bpm_range_at_150() {
        let bpm = Some(150.0);
        let range = 8.0;
        assert!((norm_to_playback_ratio(0.5, bpm, range) - 1.0).abs() < 1e-5);
        assert!((norm_to_playback_ratio(0.0, bpm, range) - 158.0 / 150.0).abs() < 1e-5);
        assert!((norm_to_playback_ratio(1.0, bpm, range) - 142.0 / 150.0).abs() < 1e-5);
    }
}
