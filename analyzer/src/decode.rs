use std::path::Path;

use analyzer_core::AnalysisConfig;
use anyhow::Result;
use codec::AudioDecoder;

pub struct DecodedMono {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

pub fn decode_mono(path: &Path, config: &AnalysisConfig) -> Result<DecodedMono> {
    let mut decoder = AudioDecoder::from_file(path)?;
    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels().max(1) as usize;

    let max_samples = config
        .max_duration_secs
        .map(|secs| (secs * f64::from(sample_rate)).ceil() as usize)
        .unwrap_or(usize::MAX);

    let mut interleaved = Vec::new();
    let mut chunk = vec![0.0f32; 8192 * channels];

    while interleaved.len() / channels < max_samples {
        let read = decoder.read_frames(&mut chunk)?;
        if read == 0 {
            break;
        }
        interleaved.extend_from_slice(&chunk[..read]);
    }

    if interleaved.len() > max_samples * channels {
        interleaved.truncate(max_samples * channels);
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

    #[test]
    fn downmix_averages_channels() {
        let interleaved = vec![1.0, -1.0, 0.5, 0.5];
        let mono = downmix_to_mono(&interleaved, 2);
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.0).abs() < f32::EPSILON);
        assert!((mono[1] - 0.5).abs() < f32::EPSILON);
    }
}
