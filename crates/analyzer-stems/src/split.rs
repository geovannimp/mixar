//! HTDemucs split of interleaved stereo PCM via stem-splitter-core.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use stem_splitter_core::core::engine;
use stem_splitter_core::{ensure_model as ssc_ensure_model, ModelHandle};

use crate::window::{audio_frame_count, fill_stereo_window};

/// Default ONNX model id (stem-splitter-core registry).
pub const DEFAULT_MODEL: &str = "htdemucs_ort_v1";

/// Canonical stem file order / names.
pub const STEM_NAMES: [&str; 4] = ["vocals", "drums", "bass", "other"];

/// Request to separate already-decoded interleaved stereo PCM.
pub struct StemSplitRequest<'a> {
    pub interleaved_stereo: &'a [f32],
    pub sample_rate: u32,
    pub output_dir: &'a Path,
    pub model_name: &'a str,
}

/// Paths to written stem WAVs (same order as [`STEM_NAMES`]).
pub struct StemSplitResult {
    pub paths: [PathBuf; 4],
    pub sample_rate: u32,
    pub backend: String,
}

/// Download / verify model weights into stem-splitter-core's cache and preload the session.
pub fn ensure_model(model_name: &str) -> Result<()> {
    let handle = ssc_ensure_model(model_name, None).map_err(ssc_err)?;
    engine::preload(&handle).map_err(ssc_err)?;
    Ok(())
}

/// Separate interleaved stereo PCM into four stem WAV files under `output_dir`.
pub fn split_interleaved_stereo(req: StemSplitRequest<'_>) -> Result<StemSplitResult> {
    let model_name = if req.model_name.is_empty() {
        DEFAULT_MODEL
    } else {
        req.model_name
    };

    let handle: ModelHandle = ssc_ensure_model(model_name, None).map_err(ssc_err)?;
    engine::preload(&handle).map_err(ssc_err)?;
    let mf = engine::manifest();

    if mf.sample_rate != 44_100 {
        return Err(anyhow!(
            "stem model sample rate must be 44100 (got {})",
            mf.sample_rate
        ));
    }

    let samples = if req.sample_rate == mf.sample_rate {
        req.interleaved_stereo.to_vec()
    } else {
        // ponytail: linear resample ceiling — upgrade to rubato FFT when stem quality audits need it.
        resample_interleaved_linear(req.interleaved_stereo, req.sample_rate, mf.sample_rate)
    };

    let n = audio_frame_count(&samples, 2);
    if n == 0 {
        return Err(anyhow!("empty audio"));
    }

    let win = mf.window;
    let hop = mf.hop;
    if !(win > 0 && hop > 0 && hop <= win) {
        return Err(anyhow!("bad win/hop in model manifest"));
    }

    let names = if mf.stems.is_empty() {
        STEM_NAMES.map(str::to_string).to_vec()
    } else {
        mf.stems.clone()
    };

    std::fs::create_dir_all(req.output_dir)
        .with_context(|| format!("create stem output dir {}", req.output_dir.display()))?;

    let paths = [
        req.output_dir.join("vocals.wav"),
        req.output_dir.join("drums.wav"),
        req.output_dir.join("bass.wav"),
        req.output_dir.join("other.wav"),
    ];

    let mut writers = open_stem_writers(&names, mf.sample_rate, &paths)?;

    let mut left_raw = vec![0f32; win];
    let mut right_raw = vec![0f32; win];
    let mut pos = 0usize;

    while pos < n {
        fill_stereo_window(&samples, 2, pos, &mut left_raw, &mut right_raw);
        let out = engine::run_window_demucs(&left_raw, &right_raw).map_err(ssc_err)?;
        let (stems_count, _ch, t_out) = (out.shape()[0], out.shape()[1], out.shape()[2]);
        let copy_len = hop.min(t_out).min(n - pos);

        for writer in &mut writers {
            let idx = writer.stem_idx.min(stems_count.saturating_sub(1));
            for i in 0..copy_len {
                writer
                    .wav
                    .write_sample(sample_to_i16(out[(idx, 0, i)]))
                    .context("write stem sample")?;
                writer
                    .wav
                    .write_sample(sample_to_i16(out[(idx, 1, i)]))
                    .context("write stem sample")?;
            }
        }

        if pos + hop >= n {
            break;
        }
        pos += hop;
    }

    for writer in writers {
        writer.wav.finalize().context("finalize stem wav")?;
    }

    Ok(StemSplitResult {
        paths,
        sample_rate: mf.sample_rate,
        backend: model_name.to_string(),
    })
}

struct StemWriter {
    stem_idx: usize,
    wav: WavWriter<BufWriter<File>>,
}

fn open_stem_writers(
    names: &[String],
    sample_rate: u32,
    paths: &[PathBuf; 4],
) -> Result<Vec<StemWriter>> {
    let mut name_idx: HashMap<String, usize> = HashMap::new();
    for (i, name) in names.iter().enumerate() {
        name_idx.insert(name.to_lowercase(), i);
    }
    let get_idx =
        |key: &str, fallback: usize| -> usize { name_idx.get(key).copied().unwrap_or(fallback) };

    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let order = [("vocals", 0usize), ("drums", 1), ("bass", 2), ("other", 3)];

    let mut out = Vec::with_capacity(4);
    for (i, (key, fallback)) in order.iter().enumerate() {
        let file =
            File::create(&paths[i]).with_context(|| format!("create {}", paths[i].display()))?;
        let wav = WavWriter::new(BufWriter::new(file), spec)?;
        out.push(StemWriter {
            stem_idx: get_idx(key, *fallback),
            wav,
        });
    }
    Ok(out)
}

fn sample_to_i16(sample: f32) -> i16 {
    (sample * f32::from(i16::MAX)).clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
}

fn ssc_err(err: impl std::fmt::Display) -> anyhow::Error {
    anyhow!("{err}")
}

fn resample_interleaved_linear(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == 0 || to_hz == 0 || input.len() < 2 {
        return Vec::new();
    }
    let in_frames = input.len() / 2;
    let last = in_frames.saturating_sub(1);
    let out_frames = (in_frames as u64 * u64::from(to_hz) / u64::from(from_hz)) as usize;
    let ratio = f64::from(from_hz) / f64::from(to_hz);
    let mut out = Vec::with_capacity(out_frames.saturating_mul(2));
    for i in 0..out_frames {
        let src = i as f64 * ratio;
        let i0 = (src.floor() as usize).min(last);
        let frac = (src - i0 as f64) as f32;
        let i1 = (i0 + 1).min(last);
        for ch in 0..2 {
            let a = input[i0 * 2 + ch];
            let b = input[i1 * 2 + ch];
            out.push(a + (b - a) * frac);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_resample_48000_to_44100_changes_frame_count() {
        let frames = 4800usize;
        let input = vec![0.25f32; frames * 2];
        let out = resample_interleaved_linear(&input, 48_000, 44_100);
        assert_eq!(out.len() / 2, 4410);
    }
}
