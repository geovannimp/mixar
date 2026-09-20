//! Stereo window helpers (unit-tested without a model).

/// Number of frames in an interleaved buffer.
pub fn audio_frame_count(samples: &[f32], channels: u16) -> usize {
    let channels = usize::from(channels.max(1));
    samples.len() / channels
}

/// Fill planar left/right windows from interleaved PCM (zero-pad past EOF).
pub fn fill_stereo_window(
    samples: &[f32],
    channels: u16,
    start_frame: usize,
    left_raw: &mut [f32],
    right_raw: &mut [f32],
) {
    debug_assert_eq!(left_raw.len(), right_raw.len());
    let channels = usize::from(channels.max(1));

    for i in 0..left_raw.len() {
        let frame = start_frame + i;
        let base = frame * channels;
        if base >= samples.len() {
            left_raw[i] = 0.0;
            right_raw[i] = 0.0;
            continue;
        }

        let left = samples[base];
        let right = if channels == 1 {
            left
        } else {
            samples.get(base + 1).copied().unwrap_or(left)
        };

        left_raw[i] = left;
        right_raw[i] = right;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_count_stereo() {
        assert_eq!(audio_frame_count(&[0.0; 8], 2), 4);
        assert_eq!(audio_frame_count(&[0.0; 3], 2), 1);
    }

    #[test]
    fn fill_window_reads_interleaved_and_pads() {
        // frames: (1,2), (3,4)
        let samples = [1.0f32, 2.0, 3.0, 4.0];
        let mut left = [0.0; 4];
        let mut right = [0.0; 4];
        fill_stereo_window(&samples, 2, 0, &mut left, &mut right);
        assert_eq!(left, [1.0, 3.0, 0.0, 0.0]);
        assert_eq!(right, [2.0, 4.0, 0.0, 0.0]);
    }
}
