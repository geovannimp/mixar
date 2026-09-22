use num_complex::Complex32;

/// Pack mono STFT specs into HTDemucs CaC planar `[4, F, T]`.
///
/// Channel order: left real, left imag, right real, right imag.
pub fn stft_to_cac_planar(
    left_spec: &[Complex32],
    right_spec: &[Complex32],
    n_fft: usize,
) -> Vec<f32> {
    debug_assert_eq!(left_spec.len(), right_spec.len());
    let bins = n_fft / 2 + 1;
    let frames = left_spec.len() / bins;
    let plane = bins * frames;
    let mut out = vec![0.0f32; 4 * plane];
    for t in 0..frames {
        for f in 0..bins {
            let idx = f * frames + t;
            out[idx] = left_spec[idx].re;
            out[plane + idx] = left_spec[idx].im;
            out[2 * plane + idx] = right_spec[idx].re;
            out[3 * plane + idx] = right_spec[idx].im;
        }
    }
    out
}

/// Unpack the first stereo channel (planes 0/1) from CaC planar `[4, F, T]` or `[2, F, T]`.
pub fn cac_planar_to_complex(slice: &[f32], bins: usize, frames: usize) -> Vec<Complex32> {
    let plane = bins * frames;
    debug_assert!(slice.len() >= 2 * plane);
    let mut out = vec![Complex32::default(); plane];
    for t in 0..frames {
        for f in 0..bins {
            let base = f * frames + t;
            out[base] = Complex32::new(slice[base], slice[plane + base]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_complex::Complex32;

    #[test]
    fn cac_pack_unpack_shape_and_invertibility() {
        let bins = 8;
        let frames = 4;
        let n_fft = (bins - 1) * 2;
        let left: Vec<Complex32> = (0..bins * frames)
            .map(|i| Complex32::new(i as f32 * 0.1, -(i as f32) * 0.05))
            .collect();
        let right: Vec<Complex32> = (0..bins * frames)
            .map(|i| Complex32::new(i as f32 * 0.2, i as f32 * 0.03))
            .collect();

        let packed = stft_to_cac_planar(&left, &right, n_fft);
        assert_eq!(packed.len(), 4 * bins * frames);

        let back_left = cac_planar_to_complex(&packed, bins, frames);
        assert_eq!(back_left, left);

        // Right channel lives in planes 2/3.
        let plane = bins * frames;
        let back_right: Vec<Complex32> = (0..plane)
            .map(|base| Complex32::new(packed[2 * plane + base], packed[3 * plane + base]))
            .collect();
        assert_eq!(back_right, right);
    }
}
