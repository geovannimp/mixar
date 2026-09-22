//! Stem cache audio formats (Opus / FLAC).

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use flacenc::bitsink::ByteSink;
use flacenc::component::BitRepr;
use flacenc::error::Verify;
use flacenc::source::MemSource;
use resampler::{Resampler, RubatoResampler};
use ruopus::encode_ogg_opus;

/// Target Opus bitrate from the #46 brainstorm (160 kbps).
pub const OPUS_BITRATE_BPS: u32 = 160_000;

/// Sample rate required by [`encode_ogg_opus`] / ruopus.
pub const OPUS_SAMPLE_RATE: u32 = 48_000;

/// On-disk stem cache codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StemAudioFormat {
    Opus,
    Flac,
}

impl StemAudioFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Opus => "opus",
            Self::Flac => "flac",
        }
    }

    pub fn extension(self) -> &'static str {
        self.as_str()
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "opus" => Some(Self::Opus),
            "flac" => Some(Self::Flac),
            _ => None,
        }
    }
}

/// Encode interleaved stereo `f32` PCM to `path`.
///
/// Returns the sample rate written into the file (always [`OPUS_SAMPLE_RATE`]
/// for Opus after resampling; `sample_rate` for FLAC).
///
/// // ponytail: full-stem f32 in RAM for encode; streaming writers if cache size hurts.
pub fn encode_stem_file(
    path: &Path,
    format: StemAudioFormat,
    interleaved_stereo: &[f32],
    sample_rate: u32,
) -> Result<u32> {
    if !interleaved_stereo.len().is_multiple_of(2) {
        return Err(anyhow!(
            "stem PCM length {} is not interleaved stereo",
            interleaved_stereo.len()
        ));
    }
    if sample_rate == 0 {
        return Err(anyhow!("sample_rate must be > 0"));
    }

    match format {
        StemAudioFormat::Opus => {
            let pcm_48k = resample_to_opus_rate(interleaved_stereo, sample_rate)?;
            let bytes = encode_ogg_opus(&pcm_48k, 2, OPUS_BITRATE_BPS);
            std::fs::write(path, bytes)
                .with_context(|| format!("write opus {}", path.display()))?;
            Ok(OPUS_SAMPLE_RATE)
        }
        StemAudioFormat::Flac => {
            let i32s: Vec<i32> = interleaved_stereo
                .iter()
                .map(|&s| {
                    (s * f32::from(i16::MAX)).clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i32
                })
                .collect();
            let config = flacenc::config::Encoder::default()
                .into_verified()
                .map_err(|e| anyhow!("flacenc config: {e:?}"))?;
            let source = MemSource::from_samples(&i32s, 2, 16, sample_rate as usize);
            let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
                .map_err(|e| anyhow!("flac encode: {e:?}"))?;
            let mut sink = ByteSink::new();
            stream
                .write(&mut sink)
                .map_err(|e| anyhow!("flac serialize: {e:?}"))?;
            std::fs::write(path, sink.as_slice())
                .with_context(|| format!("write flac {}", path.display()))?;
            Ok(sample_rate)
        }
    }
}

fn resample_to_opus_rate(pcm: &[f32], sample_rate: u32) -> Result<Vec<f32>> {
    if sample_rate == OPUS_SAMPLE_RATE {
        return Ok(pcm.to_vec());
    }

    let chunk = 512usize;
    let mut resampler = RubatoResampler::new(sample_rate, OPUS_SAMPLE_RATE, 2, chunk, "high")
        .context("create opus resampler")?;
    let mut out = Vec::with_capacity(
        ((pcm.len() as u64) * u64::from(OPUS_SAMPLE_RATE) / u64::from(sample_rate) + 64) as usize,
    );
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

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_and_roundtrip_str() {
        assert_eq!(StemAudioFormat::Opus.extension(), "opus");
        assert_eq!(StemAudioFormat::Flac.extension(), "flac");
        assert_eq!(StemAudioFormat::parse("opus"), Some(StemAudioFormat::Opus));
        assert_eq!(StemAudioFormat::parse("flac"), Some(StemAudioFormat::Flac));
        assert_eq!(StemAudioFormat::parse("wav"), None);
    }

    #[test]
    fn encode_opus_and_flac_smoke() {
        let dir = tempfile::tempdir().unwrap();
        let n = 48_000 / 4;
        let pcm = vec![0.0f32; n * 2];

        let opus = dir.path().join("t.opus");
        let rate = encode_stem_file(&opus, StemAudioFormat::Opus, &pcm, 48_000).unwrap();
        assert_eq!(rate, 48_000);
        assert!(opus.metadata().unwrap().len() > 0);

        let flac = dir.path().join("t.flac");
        let rate = encode_stem_file(&flac, StemAudioFormat::Flac, &pcm, 48_000).unwrap();
        assert_eq!(rate, 48_000);
        assert!(flac.metadata().unwrap().len() > 0);
    }

    #[test]
    fn opus_resamples_from_44100() {
        let dir = tempfile::tempdir().unwrap();
        let n = 44_100 / 4;
        let pcm = vec![0.1f32; n * 2];
        let path = dir.path().join("t.opus");
        let rate = encode_stem_file(&path, StemAudioFormat::Opus, &pcm, 44_100).unwrap();
        assert_eq!(rate, 48_000);
        assert!(path.is_file());
    }
}
