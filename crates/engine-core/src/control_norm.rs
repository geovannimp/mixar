//! Absolute control `0..1` ↔ DSP unit maps (engine edge only).

const STRIP_DB_MIN: f32 = -24.0;
const STRIP_DB_MAX: f32 = 24.0;
/// Pioneer-style tempo span matching former controller mapping (±16%).
const SPEED_MIN: f32 = 0.84;
const SPEED_MAX: f32 = 1.16;

pub fn strip_db_to_norm(db: f32) -> f32 {
    ((db - STRIP_DB_MIN) / (STRIP_DB_MAX - STRIP_DB_MIN)).clamp(0.0, 1.0)
}

pub fn norm_to_strip_db(norm: f32) -> f32 {
    STRIP_DB_MIN + norm.clamp(0.0, 1.0) * (STRIP_DB_MAX - STRIP_DB_MIN)
}

/// Playback ratio → tempo fader position (`0` = fastest / top on Pioneer HW after invert).
pub fn speed_ratio_to_norm(speed: f32) -> f32 {
    let s = speed.clamp(SPEED_MIN, SPEED_MAX);
    (1.0 - (s - SPEED_MIN) / (SPEED_MAX - SPEED_MIN)).clamp(0.0, 1.0)
}

/// Tempo fader position → playback ratio.
pub fn norm_to_speed_ratio(norm: f32) -> f32 {
    SPEED_MAX - norm.clamp(0.0, 1.0) * (SPEED_MAX - SPEED_MIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_center_is_zero_db() {
        assert!((norm_to_strip_db(0.5) - 0.0).abs() < 1e-5);
        assert!((strip_db_to_norm(0.0) - 0.5).abs() < 1e-5);
        assert!((norm_to_strip_db(1.0) - 24.0).abs() < 1e-5);
    }

    #[test]
    fn tempo_position_roundtrips() {
        assert!((norm_to_speed_ratio(0.5) - 1.0).abs() < 1e-5);
        assert!((speed_ratio_to_norm(1.0) - 0.5).abs() < 1e-5);
        assert!((norm_to_speed_ratio(0.0) - 1.16).abs() < 1e-5);
        assert!((norm_to_speed_ratio(1.0) - 0.84).abs() < 1e-5);
    }
}
