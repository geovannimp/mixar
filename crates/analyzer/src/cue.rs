//! Track cue detection: the first audible frame of the analyzed audio.

use analyzer_core::DEFAULT_CUE_THRESHOLD_DB;

/// Lowest and highest threshold a caller may pass in (dBFS).
const MIN_THRESHOLD_DB: f32 = -96.0;
const MAX_THRESHOLD_DB: f32 = -6.0;

/// First frame whose absolute value reaches `threshold_db`, in milliseconds.
///
/// `None` when `sample_rate == 0`, the buffer is empty, or nothing in the
/// analyzed window reaches the level (digital silence).
pub fn first_sound_ms(samples: &[f32], sample_rate: u32, threshold_db: f32) -> Option<i32> {
    if sample_rate == 0 || samples.is_empty() {
        return None;
    }
    let db = if threshold_db.is_finite() {
        threshold_db.clamp(MIN_THRESHOLD_DB, MAX_THRESHOLD_DB)
    } else {
        DEFAULT_CUE_THRESHOLD_DB
    };
    let threshold = 10f32.powf(db / 20.0);
    let frame = samples
        .iter()
        .position(|sample| sample.abs() >= threshold)?;
    let ms = (frame as u64 * 1000) / u64::from(sample_rate);
    i32::try_from(ms).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    fn silence(ms: u32) -> Vec<f32> {
        vec![0.0; (SR as u64 * u64::from(ms) / 1000) as usize]
    }

    fn level(ms: u32, amplitude: f32) -> Vec<f32> {
        vec![amplitude; (SR as u64 * u64::from(ms) / 1000) as usize]
    }

    fn concat(parts: &[Vec<f32>]) -> Vec<f32> {
        parts.iter().flatten().copied().collect()
    }

    #[test]
    fn silence_then_tone_cues_at_the_tone() {
        let samples = concat(&[silence(1000), level(500, 0.5)]);
        assert_eq!(first_sound_ms(&samples, SR, -60.0), Some(1000));
    }

    #[test]
    fn below_threshold_lead_in_is_silence_at_minus_60() {
        // 0.0005 is about −66 dBFS, under the −60 dBFS default threshold.
        let samples = concat(&[level(1000, 0.0005), level(500, 0.5)]);
        assert_eq!(first_sound_ms(&samples, SR, -60.0), Some(1000));
    }

    #[test]
    fn below_threshold_lead_in_is_sound_at_minus_72() {
        // At −72 dBFS (≈ 0.000 25) the same lead-in counts as sound.
        let samples = concat(&[level(1000, 0.0005), level(500, 0.5)]);
        assert_eq!(first_sound_ms(&samples, SR, -72.0), Some(0));
    }

    #[test]
    fn first_frame_above_threshold_cues_at_zero() {
        let samples = concat(&[level(500, 0.002)]);
        assert_eq!(first_sound_ms(&samples, SR, -60.0), Some(0));
    }

    #[test]
    fn digital_silence_has_no_cue() {
        let samples = silence(1000);
        assert_eq!(first_sound_ms(&samples, SR, -60.0), None);
    }

    #[test]
    fn empty_buffer_has_no_cue() {
        assert_eq!(first_sound_ms(&[], SR, -60.0), None);
    }

    #[test]
    fn zero_sample_rate_has_no_cue() {
        let samples = level(100, 0.5);
        assert_eq!(first_sound_ms(&samples, 0, -60.0), None);
    }
}
