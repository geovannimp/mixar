//! Chunked Mixxx HTDemucs ONNX inference over interleaved stereo PCM.
//!
//! Overlap-add matches StemSplit `infer.py`: 25 % overlap, stride = N − overlap,
//! linear fade on the overlap only (complementary → weight sum ≡ 1 in seams).

use std::path::Path;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use ort::value::Tensor;
use tracing::info;

use crate::resample_pcm::resample_interleaved_stereo;
use crate::session::{self, SEGMENT_SAMPLES};

/// Model sample rate (Mixxx HTDemucs ONNX).
pub const SAMPLE_RATE: u32 = 44_100;
/// Four stems.
pub const SOURCES: usize = 4;
/// StemSplit-style OLA: `overlap = N // 4`, `stride = N - overlap`.
const OVERLAP: usize = SEGMENT_SAMPLES / 4;
const CHUNK_STRIDE: usize = SEGMENT_SAMPLES - OVERLAP;
/// ONNX stem order is drums, bass, other, vocals — same as NI / Mixar.
const SOURCE_ORDER: [usize; SOURCES] = [0, 1, 2, 3];

/// Separate `pcm` (interleaved stereo) into four interleaved stereo stems at
/// [`SAMPLE_RATE`], in [`crate::STEM_NAMES`] order.
///
/// `on_progress` receives `(chunks_done, chunks_total)`.
/// Returns the EP label used for the ORT session.
pub fn separate_interleaved(
    weights: &Path,
    pcm: &[f32],
    sample_rate: u32,
    on_progress: Option<&dyn Fn(usize, usize)>,
) -> Result<([Vec<f32>; SOURCES], &'static str)> {
    if !pcm.len().is_multiple_of(2) {
        bail!(
            "stem input PCM length {} is not interleaved stereo",
            pcm.len()
        );
    }
    if sample_rate == 0 {
        bail!("stem input sample rate must be > 0");
    }

    let pcm = resample_interleaved_stereo(pcm, sample_rate, SAMPLE_RATE)?;
    let frames = pcm.len() / 2;
    let chunks = if frames == 0 {
        0
    } else {
        frames.div_ceil(CHUNK_STRIDE)
    };
    info!(
        frames,
        chunks,
        sample_rate,
        weights = %weights.display(),
        "stem separate: start"
    );

    let started = Instant::now();
    let (stems, ep) =
        session::with_session(weights, |ort_session, ep_label, in_name, out_name| {
            info!(
                ep = ep_label,
                input = in_name,
                output = out_name,
                "stem separate: ORT session"
            );
            let stems = overlap_add(&pcm, on_progress, |chunk| {
                run_chunk(ort_session, chunk, in_name, out_name)
            })?;
            Ok((stems, ep_label))
        })?;

    info!(
        frames,
        chunks,
        ep,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "stem separate: done"
    );
    Ok((stems, ep))
}

fn run_chunk(
    session: &mut ort::session::Session,
    chunk: &[f32],
    input_name: &str,
    output_name: &str,
) -> Result<[Vec<f32>; SOURCES]> {
    debug_assert_eq!(chunk.len(), SEGMENT_SAMPLES * 2);

    // Interleaved → planar (1, 2, L) channel-major.
    let mut planar = vec![0.0f32; 2 * SEGMENT_SAMPLES];
    for i in 0..SEGMENT_SAMPLES {
        planar[i] = chunk[i * 2];
        planar[SEGMENT_SAMPLES + i] = chunk[i * 2 + 1];
    }

    let input =
        Tensor::from_array(([1usize, 2, SEGMENT_SAMPLES], planar)).context("ort mix tensor")?;
    let outputs = session
        .run(ort::inputs![input_name => input])
        .context("ort stems run")?;
    let stems_val = outputs
        .get(output_name)
        .ok_or_else(|| anyhow::anyhow!("ORT output missing '{output_name}'"))?;
    let (shape, data) = stems_val
        .try_extract_tensor::<f32>()
        .context("extract stems tensor")?;

    // Expected (1, 4, 2, L) — tolerate leading batch dim.
    let dims: Vec<usize> = shape.iter().map(|d| *d as usize).collect();
    let (n_src, n_ch, n_samp) = match dims.as_slice() {
        [1, s, c, n] => (*s, *c, *n),
        [s, c, n] => (*s, *c, *n),
        other => bail!("unexpected stems shape {other:?}"),
    };
    if n_src < SOURCES || n_ch < 2 || n_samp < SEGMENT_SAMPLES {
        bail!("stems tensor too small: shape={dims:?}");
    }

    let mut out: [Vec<f32>; SOURCES] = std::array::from_fn(|_| vec![0.0f32; SEGMENT_SAMPLES * 2]);
    for (mixar_idx, &onnx_src) in SOURCE_ORDER.iter().enumerate() {
        let stem_base = onnx_src * n_ch * n_samp;
        for t in 0..SEGMENT_SAMPLES {
            out[mixar_idx][t * 2] = data[stem_base + t];
            out[mixar_idx][t * 2 + 1] = data[stem_base + n_samp + t];
        }
    }
    Ok(out)
}

/// StemSplit-style OLA window: 1.0 in the middle, linear fade over `OVERLAP`.
fn chunk_weights() -> Vec<f32> {
    let n = SEGMENT_SAMPLES;
    let mut w = vec![1.0f32; n];
    if OVERLAP <= 1 {
        return w;
    }
    let denom = (OVERLAP - 1) as f32;
    for i in 0..OVERLAP {
        let fade = i as f32 / denom;
        w[i] = fade;
        w[n - OVERLAP + i] = (OVERLAP - 1 - i) as f32 / denom;
    }
    w
}

/// Copy `valid` frames from `pcm` at `offset`; reflect-pad the rest of the segment
/// so the model does not see a hard zero cliff (zero-pad → edge glitches).
fn fill_chunk_reflect(pcm: &[f32], frames: usize, offset: usize, valid: usize, chunk: &mut [f32]) {
    debug_assert_eq!(chunk.len(), SEGMENT_SAMPLES * 2);
    chunk.fill(0.0);
    if valid == 0 || frames == 0 {
        return;
    }
    chunk[..valid * 2].copy_from_slice(&pcm[offset * 2..(offset + valid) * 2]);
    if valid >= SEGMENT_SAMPLES {
        return;
    }
    if valid == 1 {
        let l = chunk[0];
        let r = chunk[1];
        for i in 1..SEGMENT_SAMPLES {
            chunk[i * 2] = l;
            chunk[i * 2 + 1] = r;
        }
        return;
    }
    // Numpy `mode='reflect'`: period 2*(valid-1), fold the second half back.
    let period = 2 * (valid - 1);
    for i in valid..SEGMENT_SAMPLES {
        let mut x = i % period;
        if x >= valid {
            x = period - x;
        }
        chunk[i * 2] = chunk[x * 2];
        chunk[i * 2 + 1] = chunk[x * 2 + 1];
    }
}

/// Run `separate_chunk` over overlapping [`SEGMENT_SAMPLES`] chunks and blend.
fn overlap_add(
    pcm: &[f32],
    on_progress: Option<&dyn Fn(usize, usize)>,
    mut separate_chunk: impl FnMut(&[f32]) -> Result<[Vec<f32>; SOURCES]>,
) -> Result<[Vec<f32>; SOURCES]> {
    let frames = pcm.len() / 2;
    if frames == 0 {
        return Ok(std::array::from_fn(|_| Vec::new()));
    }

    let weights = chunk_weights();
    let mut stems: [Vec<f32>; SOURCES] = std::array::from_fn(|_| vec![0.0f32; frames * 2]);
    let mut weight_sum = vec![0.0f32; frames];
    let mut chunk = vec![0.0f32; SEGMENT_SAMPLES * 2];
    let total = frames.div_ceil(CHUNK_STRIDE);

    if let Some(report) = on_progress {
        report(0, total);
    }

    for index in 0..total {
        let offset = index * CHUNK_STRIDE;
        let valid = (frames - offset).min(SEGMENT_SAMPLES);
        fill_chunk_reflect(pcm, frames, offset, valid, &mut chunk);

        info!(chunk = index + 1, total, "stem separate: chunk start");
        let chunk_started = Instant::now();
        let separated = separate_chunk(&chunk)?;
        info!(
            chunk = index + 1,
            total,
            elapsed_ms = chunk_started.elapsed().as_millis() as u64,
            "stem separate: chunk done"
        );
        for (stem, out) in separated.iter().zip(stems.iter_mut()) {
            if stem.len() < valid * 2 {
                bail!(
                    "separated chunk is {} samples, expected at least {}",
                    stem.len(),
                    valid * 2
                );
            }
            for i in 0..valid {
                let w = weights[i];
                out[(offset + i) * 2] += w * stem[i * 2];
                out[(offset + i) * 2 + 1] += w * stem[i * 2 + 1];
            }
        }
        for i in 0..valid {
            weight_sum[offset + i] += weights[i];
        }

        if let Some(report) = on_progress {
            report(index + 1, total);
        }
    }

    for out in stems.iter_mut() {
        for (i, w) in weight_sum.iter().enumerate() {
            if *w > 1e-8 {
                out[i * 2] /= w;
                out[i * 2 + 1] /= w;
            }
        }
    }
    Ok(stems)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32 * std::f32::consts::TAU;
                [(t * 35.0).sin() * 0.3, (t * 52.5).sin() * 0.2]
            })
            .collect()
    }

    #[test]
    fn overlap_add_reconstructs_a_passthrough_chunker() {
        let frames = SEGMENT_SAMPLES + CHUNK_STRIDE + 517;
        let pcm = tone(frames);
        let expected_chunks = frames.div_ceil(CHUNK_STRIDE);
        let mut seen = Vec::new();
        let stems = overlap_add(
            &pcm,
            Some(&|done, total| {
                assert_eq!(total, expected_chunks);
                assert!((0..=total).contains(&done));
            }),
            |chunk| {
                seen.push(chunk.len());
                Ok(std::array::from_fn(|s| {
                    if s == 0 {
                        chunk.to_vec()
                    } else {
                        vec![0.0; chunk.len()]
                    }
                }))
            },
        )
        .expect("overlap add");
        assert_eq!(seen, vec![SEGMENT_SAMPLES * 2; expected_chunks]);
        assert_eq!(stems[0].len(), pcm.len());
        let worst = pcm
            .iter()
            .zip(&stems[0])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-5, "worst abs err {worst}");
        assert!(stems[1].iter().all(|&s| s.abs() < 1e-8));
    }

    #[test]
    fn overlap_add_handles_audio_shorter_than_one_chunk() {
        let pcm = tone(1_000);
        let stems = overlap_add(&pcm, None, |chunk| {
            Ok(std::array::from_fn(|_| chunk.to_vec()))
        })
        .expect("overlap add");
        assert_eq!(stems[0].len(), pcm.len());
        let worst = pcm
            .iter()
            .zip(&stems[0])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-5, "worst abs err {worst}");
    }

    #[test]
    fn overlap_add_on_empty_pcm_is_empty() {
        let stems = overlap_add(&[], None, |_| unreachable!("no chunks")).expect("overlap add");
        assert!(stems.iter().all(|s| s.is_empty()));
    }

    #[test]
    fn chunk_weights_fade_only_on_overlap() {
        let w = chunk_weights();
        assert_eq!(w.len(), SEGMENT_SAMPLES);
        assert_eq!(w[0], 0.0);
        assert_eq!(w[SEGMENT_SAMPLES - 1], 0.0);
        assert_eq!(w[OVERLAP], 1.0);
        assert_eq!(w[SEGMENT_SAMPLES / 2], 1.0);
        assert!((w[OVERLAP / 2] - 0.5).abs() < 1e-5);
        assert!(w.iter().all(|v| *v >= 0.0 && *v <= 1.0));
    }

    #[test]
    fn chunk_weights_are_complementary_across_stride() {
        let w = chunk_weights();
        // In the overlap of chunk0 and chunk1, fade_out + fade_in == 1.
        for i in 0..OVERLAP {
            let a = w[CHUNK_STRIDE + i];
            let b = w[i];
            assert!((a + b - 1.0).abs() < 1e-5, "seam {i}: {a} + {b} != 1");
        }
    }

    #[test]
    fn fill_chunk_reflect_pads_past_eof() {
        let pcm = tone(4);
        let mut chunk = vec![0.0f32; SEGMENT_SAMPLES * 2];
        fill_chunk_reflect(&pcm, 4, 0, 4, &mut chunk);
        assert_eq!(&chunk[..8], &pcm[..]);
        // First reflected frame mirrors index 2 (reflect over last real frame 3).
        assert_eq!(chunk[4 * 2], chunk[2 * 2]);
        assert_eq!(chunk[4 * 2 + 1], chunk[2 * 2 + 1]);
    }

    #[test]
    fn source_order_maps_onnx_to_mixar() {
        // ONNX and Mixar STEM_NAMES: drums, bass, other, vocals
        assert_eq!(SOURCE_ORDER, [0, 1, 2, 3]);
    }
}
