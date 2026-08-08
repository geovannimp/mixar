//! Tempo fader `0..1` ↔ playback ratio using ±`tempo_range` (pitch fraction).

/// Default half-span: ±6% (Mixxx / DDJ-400 first step).
pub const DEFAULT_TEMPO_RANGE: f32 = 0.06;

/// Pioneer-style cycle steps (fraction of rate).
pub const TEMPO_RANGE_STEPS: &[f32] = &[0.06, 0.10, 0.16, 0.25];

/// Next step after `current` in [`TEMPO_RANGE_STEPS`] (wraps). Unknown → first step.
pub fn next_tempo_range(current: f32) -> f32 {
    const EPS: f32 = 1e-4;
    if let Some(i) = TEMPO_RANGE_STEPS
        .iter()
        .position(|s| (*s - current).abs() < EPS)
    {
        return TEMPO_RANGE_STEPS[(i + 1) % TEMPO_RANGE_STEPS.len()];
    }
    TEMPO_RANGE_STEPS[0]
}

/// Position `0` = +range (faster), `0.5` = 1.0, `1` = −range (slower).
pub fn norm_to_playback_ratio(norm: f32, tempo_range: f32) -> f32 {
    let range = f64::from(tempo_range.max(0.0));
    let n = f64::from(norm.clamp(0.0, 1.0));
    (1.0 + (0.5 - n) * 2.0 * range).max(0.01) as f32
}

/// Saturates outside ±tempo_range.
pub fn playback_ratio_to_norm(ratio: f32, tempo_range: f32) -> f32 {
    let range = f64::from(tempo_range.max(1e-6));
    let n = 0.5 - (f64::from(ratio) - 1.0) / (2.0 * range);
    n.clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_range_maps_ends() {
        let range = 0.06;
        assert!((norm_to_playback_ratio(0.5, range) - 1.0).abs() < 1e-5);
        assert!((norm_to_playback_ratio(0.0, range) - 1.06).abs() < 1e-5);
        assert!((norm_to_playback_ratio(1.0, range) - 0.94).abs() < 1e-5);
    }

    #[test]
    fn next_steps_cycle() {
        assert!((next_tempo_range(0.06) - 0.10).abs() < 1e-5);
        assert!((next_tempo_range(0.10) - 0.16).abs() < 1e-5);
        assert!((next_tempo_range(0.16) - 0.25).abs() < 1e-5);
        assert!((next_tempo_range(0.25) - 0.06).abs() < 1e-5);
        assert!((next_tempo_range(0.08) - 0.06).abs() < 1e-5);
    }

    #[test]
    fn set_range_keeps_speed_changes_ratio() {
        let speed = 0.0;
        let r6 = norm_to_playback_ratio(speed, 0.06);
        let r10 = norm_to_playback_ratio(speed, 0.10);
        assert!((r6 - 1.06).abs() < 1e-5);
        assert!((r10 - 1.10).abs() < 1e-5);
    }
}
