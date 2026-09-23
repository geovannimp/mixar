//! Interleaved stereo PCM resampling via workspace `resampler`.

use anyhow::{Context, Result};
use resampler::{Resampler, RubatoResampler};

/// Resample interleaved stereo `f32` PCM from `from_hz` to `to_hz`.
pub fn resample_interleaved_stereo(pcm: &[f32], from_hz: u32, to_hz: u32) -> Result<Vec<f32>> {
    if from_hz == to_hz {
        return Ok(pcm.to_vec());
    }

    let chunk = 512usize;
    let mut resampler =
        RubatoResampler::new(from_hz, to_hz, 2, chunk, "high").context("create pcm resampler")?;
    let delay_frames = resampler.output_delay();
    let in_frames = pcm.len() / 2;
    let expected_frames = ((in_frames as u64) * u64::from(to_hz) / u64::from(from_hz)) as usize;
    let expected_samples = expected_frames * 2;

    let mut out = Vec::with_capacity(expected_samples + delay_frames * 2 + chunk * 2);
    let mut out_chunk = vec![0.0f32; chunk * 2];
    let mut pos = 0usize;
    let total = pcm.len();

    while pos < total {
        let need_frames = resampler.input_frames_next().max(1);
        let need_samples = need_frames * 2;
        let remain = total - pos;
        if remain >= need_samples {
            let (written, consumed) =
                resampler.process(&pcm[pos..pos + need_samples], &mut out_chunk, 2);
            out.extend_from_slice(&out_chunk[..written]);
            pos += consumed * 2;
            if consumed == 0 {
                break;
            }
        } else {
            let mut padded = vec![0.0f32; need_samples];
            padded[..remain].copy_from_slice(&pcm[pos..]);
            let (written, _consumed) = resampler.process(&padded, &mut out_chunk, 2);
            out.extend_from_slice(&out_chunk[..written]);
            break;
        }
    }

    // Flush rubato FIR/FFT delay with silence so the tail is not truncated.
    let zeros = vec![0.0f32; resampler.input_frames_next().max(1) * 2];
    while out.len() < expected_samples + delay_frames * 2 {
        let (written, _) = resampler.process(&zeros, &mut out_chunk, 2);
        if written == 0 {
            break;
        }
        out.extend_from_slice(&out_chunk[..written]);
    }

    let delay_samples = delay_frames * 2;
    if out.len() > delay_samples {
        out.drain(..delay_samples);
    }
    if out.len() > expected_samples {
        out.truncate(expected_samples);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_48000_to_44100_changes_frame_count() {
        let frames = 4800usize;
        let input = vec![0.25f32; frames * 2];
        let out = resample_interleaved_stereo(&input, 48_000, 44_100).expect("resample");
        let out_frames = out.len() / 2;
        assert!(
            (4410..=4412).contains(&out_frames),
            "expected ~4410 frames, got {out_frames}"
        );
    }

    #[test]
    fn resample_same_rate_is_copy() {
        let input = vec![0.1f32, 0.2f32, 0.3f32, 0.4f32];
        let out = resample_interleaved_stereo(&input, 48_000, 48_000).expect("passthrough");
        assert_eq!(out, input);
    }

    #[test]
    fn resample_impulse_is_not_all_leading_silence() {
        // Without trimming rubato's output_delay, a mid-track impulse lands late
        // and truncating to expected length drops the tail.
        let frames = 4800usize;
        let mut input = vec![0.0f32; frames * 2];
        input[frames] = 1.0; // left channel, halfway
        input[frames + 1] = 1.0;
        let out = resample_interleaved_stereo(&input, 48_000, 44_100).expect("resample");
        let peak = out
            .iter()
            .copied()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .expect("peak");
        let expected_mid = out.len() / 2;
        assert!(
            (peak.0 as isize - expected_mid as isize).unsigned_abs() < out.len() / 5,
            "impulse peak at {} of {}, energy={}; delay trim likely broken",
            peak.0,
            out.len(),
            peak.1
        );
        assert!(peak.1 > 0.1, "expected audible peak, got {}", peak.1);
    }
}
