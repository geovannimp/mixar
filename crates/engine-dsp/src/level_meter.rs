//! Pre-fader peak detection for deck VU meters.

/// Measure absolute peak L/R from interleaved stereo samples.
pub fn measure_stereo_peaks(interleaved: &[f32]) -> (f32, f32) {
    let mut peak_l = 0.0f32;
    let mut peak_r = 0.0f32;
    let mut i = 0;
    while i + 1 < interleaved.len() {
        peak_l = peak_l.max(interleaved[i].abs());
        peak_r = peak_r.max(interleaved[i + 1].abs());
        i += 2;
    }
    (peak_l, peak_r)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LevelPeaks {
    pub peak_l: f32,
    pub peak_r: f32,
}

impl LevelPeaks {
    pub fn from_buffer(interleaved: &[f32]) -> Self {
        let (peak_l, peak_r) = measure_stereo_peaks(interleaved);
        Self { peak_l, peak_r }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaks_from_interleaved_stereo_buffer() {
        // L R L R … → peak_l=0.5, peak_r=0.8
        let buf = [0.1, -0.8, 0.5, 0.2, -0.3, 0.4];
        let (peak_l, peak_r) = measure_stereo_peaks(&buf);
        assert!((peak_l - 0.5).abs() < 1e-6);
        assert!((peak_r - 0.8).abs() < 1e-6);
    }

    #[test]
    fn empty_buffer_is_zero() {
        assert_eq!(measure_stereo_peaks(&[]), (0.0, 0.0));
    }
}
