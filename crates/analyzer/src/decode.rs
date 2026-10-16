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
    /// Mono samples downmixed from the file's channels, ordered by time.
    ///
    /// Raw channel-averaged f32 in the file's native scale, so a hot master can
    /// carry samples past ±1.0. Not peak-normalized: the crate's own entry
    /// points run the buffer through the preprocess stage, which scales it into
    /// ±1.0, before a backend sees it.
    pub samples: Vec<f32>,
    /// Sample rate the samples were decoded at (the file's native rate).
    pub sample_rate: u32,
}

/// Frames pulled from the decoder per read.
///
/// Large enough that a packet is filled in one go, small enough that the
/// interleaved scratch stays in L2 while it is being averaged into the output.
const DECODE_CHUNK_FRAMES: usize = 8192;

/// Largest frame count the output buffer is sized for up front.
///
/// Derived from the analysis memory budget in `docs/audio-analyzer-spec.md` §12
/// (< 500 MB peak per concurrent analysis job) rather than from a duration: the
/// mono buffer is the job's largest single consumer, so it is capped at 100 Mi
/// frames × 4 bytes = 400 MiB (419 MB), leaving ~80 MB inside the budget for the
/// decode scratch, the backend's working set, and the loudness meter. Any cap up
/// to ~39.6 minutes at 44.1 kHz (36.4 min at 48 kHz) is pre-sized exactly and
/// never grows.
///
/// The ceiling bounds the *request*. On a platform with lazy mapping the
/// untouched capacity costs address space rather than resident memory, but that
/// cannot be relied on: under strict overcommit, or on a platform that commits
/// eagerly, the reservation *is* the peak — which is why it is sized against the
/// budget instead of being treated as a free hint.
///
/// A cap above the ceiling still grows, by doubling, because the file's true
/// length is not known until EOF. It is deliberately not avoided by clamping the
/// cap itself: the cap is a contract (decode this much) and silently truncating
/// a long track would be worse than a later doubling. So the reservation is a
/// bounded hint and the cap stays exact.
const MAX_RESERVED_FRAMES: usize = 100 * 1024 * 1024;

/// Frames the output buffer is sized for, clamped to [`MAX_RESERVED_FRAMES`].
///
/// Split out from `decode_mono` so the clamp — the only reason an absurd
/// `max_duration_ms` cannot become a huge `Vec::with_capacity` — is testable
/// as arithmetic, without a decode that would really request the allocation.
fn reserved_frames(max_frames: Option<usize>) -> usize {
    max_frames.map_or(0, |limit| limit.min(MAX_RESERVED_FRAMES))
}

/// Decode `path` to mono f32, honouring the length cap in `config`.
///
/// The decoder is read one chunk at a time and each chunk is averaged into the
/// output as it arrives, so the interleaved samples are never all resident at
/// once — only the mono result is.
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
    let mut mono = Vec::with_capacity(reserved_frames(max_frames));
    let mut chunk = vec![0.0f32; DECODE_CHUNK_FRAMES * channels];

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
        downmix_append(&mut mono, &chunk[..room * channels], channels);
    }

    debug_assert!(max_frames.is_none_or(|limit| mono.len() <= limit));

    Ok(DecodedMono {
        samples: mono,
        sample_rate,
    })
}

/// Average `interleaved` into `mono`.
///
/// A partial trailing frame — fewer than `channels` samples left over — is
/// dropped rather than averaged against missing input. This is the streaming
/// form of the test-only `downmix_to_mono` oracle; the arithmetic per frame is
/// identical.
fn downmix_append(mono: &mut Vec<f32>, interleaved: &[f32], channels: usize) {
    if channels <= 1 {
        mono.extend_from_slice(interleaved);
        return;
    }
    let divisor = channels as f32;
    for frame in interleaved.chunks_exact(channels) {
        let sum: f32 = frame.iter().copied().sum();
        mono.push(sum / divisor);
    }
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
        // Both channels identical, so the mean is the channel value itself.
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

    /// Write a stereo WAV whose two channels carry independent values, so the
    /// downmix mean cannot cancel by symmetry.
    fn write_distinct_stereo_wav(path: &Path, frames: u32) {
        write_multichannel_wav(path, frames, 2);
    }

    /// Write `frames` frames of `channels` channels, with channel `c` carrying
    /// `(i * (c + 1)) % 101` so every channel is distinguishable and the mean is
    /// independently computable.
    fn write_multichannel_wav(path: &Path, frames: u32, channels: u16) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        let channel_count = usize::from(channels);
        for i in 0..frames {
            for channel in 0..channel_count {
                let value = (i as usize * (channel + 1)) % 101;
                writer.write_sample(value as i16).unwrap();
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

    /// The averaging must be over every channel, for every channel count the
    /// decoder can hand us — not just the stereo path.
    #[test]
    fn downmix_averages_every_channel() {
        for channels in [1u16, 2, 3, 4, 5, 6, 8] {
            let dir = tempfile::tempdir().unwrap();
            let wav = dir.path().join("multi.wav");
            write_multichannel_wav(&wav, 512, channels);

            let decoded = decode_mono(&wav, &AnalysisConfig::default()).unwrap();
            assert_eq!(decoded.samples.len(), 512);
            for (index, &sample) in decoded.samples.iter().enumerate() {
                let channel_count = usize::from(channels) as f32;
                let sum: f32 = (0..usize::from(channels))
                    .map(|channel| (index * (channel + 1)) % 101)
                    .sum::<usize>() as f32;
                let expected = sum / channel_count / 32768.0;
                assert!(
                    (sample - expected).abs() < 1e-6,
                    "channels={channels} frame {index}: got {sample}, expected {expected}"
                );
            }
        }
    }

    /// A partial trailing frame — a decoder read that stops mid-frame — is
    /// dropped, which is what the whole-buffer downmix it replaced did too.
    /// Mono has nothing to average, so a lone sample still counts as a frame.
    #[test]
    fn partial_trailing_frame_is_dropped() {
        for channels in [2usize, 3, 8] {
            let mut mono = Vec::new();
            // Two whole frames plus a partial one.
            let mut interleaved = (0..2 * channels).map(|i| i as f32).collect::<Vec<_>>();
            interleaved.extend(std::iter::once(99.0));
            downmix_append(&mut mono, &interleaved, channels);
            assert_eq!(
                mono.len(),
                2,
                "channels={channels}: a partial frame must not become a sample"
            );
        }

        let mut mono = Vec::new();
        downmix_append(&mut mono, &[1.0, 2.0, 3.0], 1);
        assert_eq!(mono, vec![1.0, 2.0, 3.0]);
    }

    /// A cap larger than the file must still return the whole file: the read
    /// loop stops at EOF rather than at the cap.
    #[test]
    fn a_cap_beyond_the_file_returns_the_whole_file() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("long.wav");
        write_distinct_stereo_wav(&wav, 44_100);

        let config = AnalysisConfig {
            // A minute of 44.1 kHz audio: well past the file, well under
            // `MAX_RESERVED_FRAMES`, so the reservation stays small.
            max_duration_ms: Some(60_000),
            ..Default::default()
        };
        let decoded = decode_mono(&wav, &config).unwrap();
        assert_eq!(decoded.samples.len(), 44_100);
    }

    /// The clamp that keeps an absurd cap from becoming an allocation request.
    ///
    /// Tested as arithmetic rather than through `decode_mono`, which would
    /// really issue the clamped `Vec::with_capacity` (a 512 MiB request) just
    /// to observe the result.
    #[test]
    fn the_reservation_is_clamped_to_the_ceiling() {
        assert_eq!(reserved_frames(None), 0);
        assert_eq!(reserved_frames(Some(1)), 1);
        assert_eq!(
            reserved_frames(Some(MAX_RESERVED_FRAMES)),
            MAX_RESERVED_FRAMES
        );
        assert_eq!(
            reserved_frames(Some(MAX_RESERVED_FRAMES + 1)),
            MAX_RESERVED_FRAMES
        );
        assert_eq!(reserved_frames(Some(usize::MAX)), MAX_RESERVED_FRAMES);
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
                for piece in interleaved.chunks(chunk_frames * channels) {
                    downmix_append(&mut streamed, piece, channels);
                }
                assert_eq!(
                    streamed, expected,
                    "channels={channels} chunk_frames={chunk_frames} diverged"
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
        // Channel c carries (i * (c + 1)) % 101, so the mean is computed from
        // the inputs rather than cancelled by symmetry.
        for (index, &sample) in decoded.samples.iter().enumerate() {
            let sum: f32 = (0..2).map(|c| (index * (c + 1)) % 101).sum::<usize>() as f32;
            let expected = sum / 2.0 / 32768.0;
            assert!(
                (sample - expected).abs() < 1e-6,
                "frame {index}: got {sample}, expected {expected} (from {sum})"
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
        write_multichannel_wav(&wav, 10_000, 1);

        let decoded = decode_mono(&wav, &AnalysisConfig::default()).unwrap();
        assert_eq!(decoded.samples.len(), 10_000);
        for (index, &sample) in decoded.samples.iter().enumerate() {
            assert_eq!(sample, ((index % 101) as f32) / 32768.0);
        }
    }
}
