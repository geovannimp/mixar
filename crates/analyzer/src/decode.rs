//! File decode stage of the offline analysis pipeline.
//!
//! [`decode_mono`] turns a file on disk into the mono f32 buffer every analyzer
//! backend expects. It is part of the crate's public API so callers that only
//! need the decoded buffer (for example to inspect it, or to reuse the decode
//! stage across analyses) do not have to re-implement the downmix.

use std::path::Path;

use analyzer_core::AnalysisConfig;
use anyhow::Result;
use codec::AudioDecoder;

/// A decoded file, downmixed to mono and truncated to the configured duration.
#[derive(Clone, Debug)]
pub struct DecodedMono {
    /// Mono samples normalized to ±1.0, ordered by time.
    pub samples: Vec<f32>,
    /// Sample rate the samples were decoded at (the file's native rate).
    pub sample_rate: u32,
}

/// Decode `path` to mono f32, honouring the length cap in `config`.
///
/// Channels are averaged (simple arithmetic mean), matching the downmix the
/// library applies elsewhere.
///
/// # Errors
///
/// Returns an error when the file cannot be opened, probed, or decoded, or when
/// the decoder reports an unsupported format.
pub fn decode_mono(path: &Path, config: &AnalysisConfig) -> Result<DecodedMono> {
    let mut decoder = AudioDecoder::from_file(path)?;
    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels().max(1) as usize;

    let max_frames = config
        .max_duration_ms
        .map(|ms| (f64::from(ms) / 1000.0 * f64::from(sample_rate)).ceil() as usize);

    let mut interleaved = Vec::new();
    let mut chunk = vec![0.0f32; 8192 * channels];

    while max_frames.is_none_or(|limit| interleaved.len() / channels < limit) {
        let read = decoder.read_frames(&mut chunk)?;
        if read == 0 {
            break;
        }
        interleaved.extend_from_slice(&chunk[..read]);
    }

    if let Some(limit) = max_frames {
        let max_len = limit.saturating_mul(channels);
        if interleaved.len() > max_len {
            interleaved.truncate(max_len);
        }
    }

    let mono = downmix_to_mono(&interleaved, channels);
    Ok(DecodedMono {
        samples: mono,
        sample_rate,
    })
}

fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let frames = interleaved.len() / channels;
    let mut mono = Vec::with_capacity(frames);
    for frame in 0..frames {
        let base = frame * channels;
        let sum: f32 = (0..channels).map(|ch| interleaved[base + ch]).sum();
        mono.push(sum / channels as f32);
    }
    mono
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_stereo_wav(path: &Path, frames: u32) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..frames {
            let sample = (i % 100) as i16;
            writer.write_sample(sample).unwrap();
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn downmix_averages_channels() {
        let interleaved = vec![1.0, -1.0, 0.5, 0.5];
        let mono = downmix_to_mono(&interleaved, 2);
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.0).abs() < f32::EPSILON);
        assert!((mono[1] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn decode_unlimited_duration_stereo_does_not_overflow() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("stereo.wav");
        write_stereo_wav(&wav, 2048);

        let config = AnalysisConfig::default();
        let decoded = decode_mono(&wav, &config).unwrap();
        assert_eq!(decoded.samples.len(), 2048);
    }
}
