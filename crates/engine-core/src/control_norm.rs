//! Absolute strip-control `0..1` ↔ dB for EQ / filter / gain.
//!
//! Tempo fader `0..1` ↔ playback ratio lives in `engine_dsp::tempo` (deck owns `tempo_range`).

const STRIP_DB_MIN: f32 = -24.0;
const STRIP_DB_MAX: f32 = 24.0;

pub fn strip_db_to_norm(db: f32) -> f32 {
    ((db - STRIP_DB_MIN) / (STRIP_DB_MAX - STRIP_DB_MIN)).clamp(0.0, 1.0)
}

pub fn norm_to_strip_db(norm: f32) -> f32 {
    STRIP_DB_MIN + norm.clamp(0.0, 1.0) * (STRIP_DB_MAX - STRIP_DB_MIN)
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
}
