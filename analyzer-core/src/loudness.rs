//! Loudness normalization helpers: ReplayGain → LUFS and auto-gain computation.

/// ReplayGain 2.0 reference loudness (LUFS).
pub const REPLAYGAIN_REFERENCE_LUFS: f64 = -18.0;

/// Maximum auto-gain applied when normalizing (± dB).
pub const AUTO_GAIN_CLAMP_DB: f32 = 12.0;

/// Convert ReplayGain track gain (dB) to integrated loudness (LUFS).
///
/// RG2: `track_gain_db` is the gain to apply to reach −18 LUFS reference.
pub fn loudness_lufs_from_replaygain_track_gain_db(track_gain_db: f64) -> f64 {
    REPLAYGAIN_REFERENCE_LUFS - track_gain_db
}

/// Compute auto-gain (dB) to reach `target_lufs` from measured `loudness_lufs`.
///
/// Result is clamped to ± [`AUTO_GAIN_CLAMP_DB`].
pub fn auto_gain_db(target_lufs: f32, loudness_lufs: f64) -> f32 {
    (target_lufs - loudness_lufs as f32).clamp(-AUTO_GAIN_CLAMP_DB, AUTO_GAIN_CLAMP_DB)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaygain_plus_3_means_loudness_minus_21() {
        // +3 dB gain needed → track is 3 dB under −18 reference
        let l = loudness_lufs_from_replaygain_track_gain_db(3.0);
        assert!((l - (-21.0)).abs() < 1e-9);
    }

    #[test]
    fn auto_gain_matches_difference_and_clamps() {
        assert!((auto_gain_db(-18.0, -18.0) - 0.0).abs() < 1e-5);
        assert!((auto_gain_db(-18.0, -24.0) - 6.0).abs() < 1e-5);
        assert!((auto_gain_db(-18.0, 0.0) - (-12.0)).abs() < 1e-5); // clamp
        assert!((auto_gain_db(-18.0, -40.0) - 12.0).abs() < 1e-5); // clamp
    }
}
