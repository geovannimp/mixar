//! File decode stage of the offline analysis pipeline.
//!
//! [`decode_mono`] turns a file on disk into the mono f32 buffer every analyzer
//! backend expects. It is part of the crate's public API so callers that only
//! need the decoded buffer (for example to inspect it, or to reuse the decode
//! stage across analyses) do not have to re-implement the downmix.

use std::path::Path;

use crate::preprocess::peak_of;

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
    /// Largest absolute value in `samples`, or `0.0` when it is empty.
    ///
    /// Measured while the channels were averaged, so the preprocess stage does
    /// not have to walk the buffer again to find it.
    pub peak: f32,
}

/// Frames pulled from the decoder per read.
///
/// Large enough that a packet is filled in one go, small enough that the
/// interleaved scratch stays in L2 while it is being averaged into the output.
const DECODE_CHUNK_FRAMES: usize = 8192;

/// Decodes `path` into `mono`, honouring the length cap in `config`.
///
/// The decoder is read one chunk at a time and each chunk is averaged into the
/// output as it arrives, so the interleaved samples are never all resident at
/// once — only the mono result is. `peak` accumulates over the same values the
/// averaging produces.
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

    // The cap is an exact frame count, so the output buffer can be sized once
    // instead of grown by doubling. Without a cap the true length is not known
    // up front, so the vector grows geometrically.
    let mut mono = Vec::with_capacity(max_frames.unwrap_or(0));
    let mut chunk = vec![0.0f32; DECODE_CHUNK_FRAMES * channels];
    let mut peak: f32 = 0.0;

    while max_frames.is_none_or(|limit| mono.len() < limit) {
        let read = decoder.read_frames(&mut chunk)?;
        if read == 0 {
            break;
        }
        // `read_frames` reports a sample count with no promise that it is a
        // whole number of frames; a partial trailing frame is dropped, exactly
        // as the previous whole-buffer downmix did.
        let frames = read / channels;
        let room = max_frames.map_or(frames, |limit| (limit - mono.len()).min(frames));
        if room == 0 {
            break;
        }
        let usable = &chunk[..room * channels];
        mono.reserve(room);
        downmix_append(&mut mono, usable, channels, &mut peak);
    }

    debug_assert!(max_frames.is_none_or(|limit| mono.len() <= limit));

    Ok(DecodedMono {
        samples: mono,
        sample_rate,
        peak,
    })
}

/// Average `interleaved` into `mono`, tracking the largest absolute result.
///
/// `interleaved` must contain `channels * n` samples. This is the streaming
/// form of [`downmix_to_mono`]; the arithmetic per frame is identical. The peak
/// is taken over the frames this call just wrote, which is the same value a
/// separate pass over the finished buffer would find, but stays in cache.
fn downmix_append(mono: &mut Vec<f32>, interleaved: &[f32], channels: usize, peak: &mut f32) {
    let start = mono.len();
    if channels <= 1 {
        mono.extend_from_slice(interleaved);
    } else {
        let divisor = channels as f32;
        for frame in interleaved.chunks_exact(channels) {
            let sum: f32 = frame.iter().copied().sum();
            mono.push(sum / divisor);
        }
    }
    *peak = peak.max(peak_of(&mono[start..]));
}

/// Average the channels of `interleaved` down to one sample per frame.
#[cfg(test)]
pub(super) fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
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

    /// Deterministic xorshift so fixtures are fixed across runs.
    struct Rng(u64);

    impl Rng {
        fn next_f32(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            // Map to (-1.5, 1.5) so the peak-normalize branch is exercised.
            (self.0 as f32 / u64::MAX as f32) * 3.0 - 1.5
        }
    }

    fn write_stereo_wav(path: &Path, frames: u32) {
        write_wav(path, frames, 2, move |i| {
            let sample = (i % 100) as i16;
            (sample, sample)
        });
    }

    /// Write a stereo WAV whose two channels carry independent values, so the
    /// downmix mean cannot cancel by symmetry.
    fn write_distinct_stereo_wav(path: &Path, frames: u32) {
        write_wav(path, frames, 2, |i| {
            ((i % 101) as i16, ((i * 7) % 101) as i16)
        });
    }

    /// Write `frames` frames; `channels` picks mono or stereo, and a channel the
    /// closure does not fill is zero.
    fn write_wav(
        path: &Path,
        frames: u32,
        channels: u16,
        mut sample: impl FnMut(u32) -> (i16, i16),
    ) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..frames {
            let (left, right) = sample(i);
            writer.write_sample(left).unwrap();
            if channels == 2 {
                writer.write_sample(right).unwrap();
            }
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
    fn streaming_downmix_matches_whole_buffer_downmix() {
        let mut rng = Rng(0x2545F491_4F6CDD1D);
        for channels in [1usize, 2, 3, 5, 8] {
            let mut interleaved = Vec::new();
            for _ in 0..(4 * DECODE_CHUNK_FRAMES + 37) * channels {
                interleaved.push(rng.next_f32());
            }
            let expected = downmix_to_mono(&interleaved, channels);

            // Feed the same data in differently-sized whole frames: the result
            // must not depend on where the decoder's reads happen to land.
            for chunk_frames in [1usize, 7, 8192, DECODE_CHUNK_FRAMES] {
                let mut streamed = Vec::new();
                let mut peak = 0.0f32;
                for piece in interleaved.chunks(chunk_frames * channels) {
                    downmix_append(&mut streamed, piece, channels, &mut peak);
                }
                assert_eq!(
                    streamed, expected,
                    "channels={channels} chunk_frames={chunk_frames} diverged"
                );

                // Peak is over the same values, so it is independent of chunking.
                let expected_peak = crate::preprocess::peak_of(&expected);
                assert!(
                    (peak - expected_peak).abs() <= f32::EPSILON * expected_peak.abs(),
                    "channels={channels} chunk_frames={chunk_frames} peak {peak} != {expected_peak}"
                );
            }
        }
    }

    #[test]
    fn decode_stereo_averages_channels() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("stereo.wav");
        write_distinct_stereo_wav(&wav, 1000);

        let decoded = decode_mono(&wav, &AnalysisConfig::default()).unwrap();
        assert_eq!(decoded.samples.len(), 1000);
        // L and R are independent pseudo-random values, so the expected mean has
        // to be computed from the inputs rather than cancelled by symmetry.
        for (index, &sample) in decoded.samples.iter().enumerate() {
            let left = (index % 101) as f32;
            let right = ((index * 7) % 101) as f32;
            let expected = (left + right) / 2.0 / 32768.0;
            assert!(
                (sample - expected).abs() < 1e-6,
                "frame {index}: got {sample}, expected {expected} (from {left}, {right})"
            );
        }
    }

    #[test]
    fn decode_unlimited_duration_stereo_does_not_overflow() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("stereo.wav");
        write_stereo_wav(&wav, 2048);

        let decoded = decode_mono(&wav, &AnalysisConfig::default()).unwrap();
        assert_eq!(decoded.samples.len(), 2048);
        assert_eq!(decoded.sample_rate, 44100);
        // Identical channels average to the channel value itself.
        for (index, &sample) in decoded.samples.iter().enumerate() {
            let expected = (index % 100) as f32 / 32768.0;
            assert!(
                (sample - expected).abs() < 1e-7,
                "frame {index}: got {sample}, expected {expected}"
            );
        }
    }

    #[test]
    fn decode_honours_duration_cap() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("long.wav");
        write_distinct_stereo_wav(&wav, 44_100 * 10);

        let config = AnalysisConfig {
            max_duration_ms: Some(2500),
            ..Default::default()
        };
        let decoded = decode_mono(&wav, &config).unwrap();
        let expected = (f64::from(2500) / 1000.0 * 44100.0).ceil() as usize;
        assert_eq!(decoded.samples.len(), expected);
        assert_eq!(
            decoded.peak,
            decoded
                .samples
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()))
        );
    }

    #[test]
    fn decode_cap_of_one_second_reads_exact_frame_count() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("exact.wav");
        write_distinct_stereo_wav(&wav, 44_100 * 10);

        let config = AnalysisConfig {
            max_duration_ms: Some(1000),
            ..Default::default()
        };
        let decoded = decode_mono(&wav, &config).unwrap();
        assert_eq!(decoded.samples.len(), 44100);
    }

    #[test]
    fn decode_mono_is_passed_through_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("mono.wav");
        write_wav(&wav, 10_000, 1, |i| ((i % 200) as i16, 0));

        let decoded = decode_mono(&wav, &AnalysisConfig::default()).unwrap();
        assert_eq!(decoded.samples.len(), 10_000);
        for (index, &sample) in decoded.samples.iter().enumerate() {
            assert!(sample == (index % 200) as f32 / 32768.0);
        }
        assert!(decoded.peak > 0.0);
    }
}
