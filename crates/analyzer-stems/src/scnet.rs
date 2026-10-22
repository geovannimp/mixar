//! SCNet int8 offline separation (spectrogram I/O, 11 s segments, 75 % overlap).
//!
//! Mirrors Music-Source-Separation-Training's `demix()` for this model:
//! whole-track mean/std normalization, reflect border padding, 4-way overlap
//! with a 10 % linear fade, rectangular 4096/1024 STFT, and per-stem
//! denormalization. The ONNX graph consumes `mix_spec [1,4,2049,476]`
//! (L re/im, R re/im) and returns `stems_spec [1,16,2049,476]`
//! (drums, bass, other, vocals × channel × re/im).

use std::path::Path;

use anyhow::{bail, Context, Result};
use ort::session::Session;
use ort::value::Tensor;
use realfft::num_complex::Complex32;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use tracing::info;

use crate::resample_pcm::resample_interleaved_stereo;
use crate::session;

/// Model sample rate.
const SAMPLE_RATE: u32 = 44_100;
/// 11 s at 44.1 kHz.
const CHUNK: usize = 485_100;
/// STFT hop.
const HOP: usize = 1024;
/// STFT size.
const N_FFT: usize = 4096;
/// One-sided bins (`0..=N_FFT/2`).
const N_BINS: usize = N_FFT / 2 + 1;
/// Frames after the model's zero pad to 486 400 samples.
const FRAMES: usize = 476;
/// Zero pad the model applies before its STFT (`486 400 - 485 100`).
const ZERO_PAD: usize = 1300;
/// MSST `inference.num_overlap` for SCNet.
const NUM_OVERLAP: usize = 4;
/// Stride between segments.
const STEP: usize = CHUNK / NUM_OVERLAP;
/// MSST fade region (`chunk_size // 10`).
const FADE: usize = CHUNK / 10;
/// MSST whole-track reflect border (`chunk_size - step`).
const BORDER: usize = CHUNK - STEP;
/// Four sources in NI order.
const SOURCES: usize = 4;
/// Stereo.
const CHANNELS: usize = 2;
/// Full iSTFT timeline length.
const FULL: usize = (FRAMES - 1) * HOP + N_FFT;
/// Length after centre-trim, before the model's trailing zero pad is removed.
const OUT_LEN: usize = CHUNK + ZERO_PAD;

/// Separate interleaved stereo PCM with the SCNet int8 ONNX.
pub(crate) fn separate(
    weights: &Path,
    pcm: &[f32],
    sample_rate: u32,
    on_progress: Option<&dyn Fn(usize, usize)>,
) -> Result<([Vec<f32>; SOURCES], &'static str)> {
    if !pcm.len().is_multiple_of(2) {
        bail!(
            "SCNet input PCM length {} is not interleaved stereo",
            pcm.len()
        );
    }
    if sample_rate == 0 {
        bail!("SCNet input sample rate must be > 0");
    }

    let pcm = resample_interleaved_stereo(pcm, sample_rate, SAMPLE_RATE)?;
    let frames = pcm.len() / 2;
    if frames == 0 {
        let ep = session::with_session(weights, |_, ep, _, _| Ok(ep))?;
        return Ok((std::array::from_fn(|_| Vec::new()), ep));
    }

    info!(frames, "scnet: start");

    // MSST `normalize_audio`: mono mean/std over the whole track.
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);
    for i in (0..pcm.len()).step_by(2) {
        left.push(pcm[i]);
        right.push(pcm[i + 1]);
    }
    let mono_sum: f64 = left
        .iter()
        .zip(&right)
        .map(|(l, r)| (f64::from(*l) + f64::from(*r)) * 0.5)
        .sum();
    let mean = mono_sum / frames as f64;
    let variance: f64 = left
        .iter()
        .zip(&right)
        .map(|(l, r)| {
            let m = (f64::from(*l) + f64::from(*r)) * 0.5 - mean;
            m * m
        })
        .sum::<f64>()
        / frames as f64;
    let std = variance.sqrt().max(1e-8);
    for sample in left.iter_mut().chain(right.iter_mut()) {
        *sample = ((f64::from(*sample) - mean) / std) as f32;
    }

    // MSST pads the whole track by `border` on both sides for long inputs.
    let (padded_left, padded_right) = if frames > 2 * BORDER {
        (
            pad_reflect_both(&left, BORDER),
            pad_reflect_both(&right, BORDER),
        )
    } else {
        (left.clone(), right.clone())
    };
    let length = padded_left.len();

    let mut planner = RealFftPlanner::<f32>::new();
    let forward = planner.plan_fft_forward(N_FFT);
    let inverse = planner.plan_fft_inverse(N_FFT);
    let mut forward_scratch = vec![Complex32::ZERO; forward.get_scratch_len()];
    let mut inverse_scratch = vec![Complex32::ZERO; inverse.get_scratch_len()];

    let window = chunk_window();
    // Interleaved per source: four buffers instead of eight channel buffers
    // plus a second full output copy at the end.
    let mut result: [Vec<f32>; SOURCES] = std::array::from_fn(|_| vec![0.0f32; length * 2]);
    let mut counter = vec![0.0f32; length];
    let total = length.div_ceil(STEP);
    if let Some(report) = on_progress {
        report(0, total);
    }

    let mut ep_label = None;
    let mut done = 0usize;
    let mut offset = 0usize;
    while offset < length {
        let chunk_len = (length - offset).min(CHUNK);
        let chunk_left = fit_chunk(&padded_left, offset, chunk_len);
        let chunk_right = fit_chunk(&padded_right, offset, chunk_len);

        let stems = session::with_session(weights, |sess, ep, input, output| {
            ep_label = Some(ep);
            run_segment(
                sess,
                &chunk_left,
                &chunk_right,
                input,
                output,
                forward.as_ref(),
                inverse.as_ref(),
                &mut forward_scratch,
                &mut inverse_scratch,
            )
        })?;

        let first = offset == 0;
        let last = offset + STEP >= length;
        let mut weight = window.clone();
        if first {
            weight[..FADE].fill(1.0);
        }
        if last {
            weight[CHUNK - FADE..].fill(1.0);
        }
        for (out, channels) in result.iter_mut().zip(stems.iter()) {
            for i in 0..chunk_len {
                out[(offset + i) * 2] += channels[0][i] * weight[i];
                out[(offset + i) * 2 + 1] += channels[1][i] * weight[i];
            }
        }
        for i in 0..chunk_len {
            counter[offset + i] += weight[i];
        }

        done += 1;
        if let Some(report) = on_progress {
            report(done, total);
        }
        offset += STEP;
    }

    let start = if length == frames { 0 } else { BORDER };
    let mut stems = result;
    for out in stems.iter_mut() {
        for frame in 0..frames {
            let idx = start + frame;
            let w = counter[idx].max(1e-8);
            let left = f64::from(out[idx * 2] / w) * std + mean;
            let right = f64::from(out[idx * 2 + 1] / w) * std + mean;
            out[frame * 2] = left as f32;
            out[frame * 2 + 1] = right as f32;
        }
        out.truncate(frames * 2);
    }

    let ep = ep_label.context("SCNet separation produced no segments")?;
    info!(frames, ep, "scnet: done");
    Ok((stems, ep))
}

/// One MSST chunk window: 1.0 in the middle, linear `FADE` fades at the edges.
fn chunk_window() -> Vec<f32> {
    let mut window = vec![1.0f32; CHUNK];
    let denom = (FADE - 1) as f32;
    for i in 0..FADE {
        window[i] = i as f32 / denom;
        window[CHUNK - FADE + i] = (FADE - 1 - i) as f32 / denom;
    }
    window
}

/// PyTorch `F.pad(mode="reflect")` for a whole signal, both sides.
fn pad_reflect_both(signal: &[f32], pad: usize) -> Vec<f32> {
    let n = signal.len();
    debug_assert!(pad > 0 && pad < n.saturating_sub(1));
    let mut out = vec![0.0f32; n + 2 * pad];
    out[pad..pad + n].copy_from_slice(signal);
    for k in 0..pad {
        out[pad - 1 - k] = signal[k + 1];
        out[pad + n + k] = signal[n - 2 - k];
    }
    out
}

/// PyTorch `F.pad(..., mode="reflect")` on the right edge only.
fn pad_reflect_right(signal: &[f32], target: usize) -> Vec<f32> {
    let n = signal.len();
    let pad = target.saturating_sub(n);
    debug_assert!(pad == 0 || pad < n.saturating_sub(1));
    let mut out = vec![0.0f32; target];
    out[..n].copy_from_slice(signal);
    for k in 0..pad {
        out[n + k] = signal[n - 2 - k];
    }
    out
}

/// Copy or reflect-pad a chunk into the model's [`CHUNK`]-sample window.
fn fit_chunk(signal: &[f32], offset: usize, len: usize) -> Vec<f32> {
    if len > CHUNK / 2 {
        pad_reflect_right(&signal[offset..offset + len], CHUNK)
    } else {
        let mut out = vec![0.0f32; CHUNK];
        out[..len].copy_from_slice(&signal[offset..offset + len]);
        out
    }
}

/// Run one 11 s segment through the ONNX and return four stereo stems.
#[allow(clippy::too_many_arguments)]
fn run_segment(
    sess: &mut Session,
    left: &[f32],
    right: &[f32],
    input_name: &str,
    output_name: &str,
    forward: &dyn RealToComplex<f32>,
    inverse: &dyn ComplexToReal<f32>,
    forward_scratch: &mut [Complex32],
    inverse_scratch: &mut [Complex32],
) -> Result<[[Vec<f32>; CHANNELS]; SOURCES]> {
    debug_assert_eq!(left.len(), CHUNK);
    debug_assert_eq!(right.len(), CHUNK);

    // Model pre-pads to 486 400 samples before its centered STFT.
    let mut padded_left = vec![0.0f32; OUT_LEN];
    let mut padded_right = vec![0.0f32; OUT_LEN];
    padded_left[..CHUNK].copy_from_slice(left);
    padded_right[..CHUNK].copy_from_slice(right);

    let left_spec = stft(&padded_left, forward, forward_scratch)?;
    let right_spec = stft(&padded_right, forward, forward_scratch)?;
    let mut spec_tensor = vec![0.0f32; 4 * N_BINS * FRAMES];
    fill_tensor_channel(&mut spec_tensor, 0, &left_spec, false);
    fill_tensor_channel(&mut spec_tensor, 1, &left_spec, true);
    fill_tensor_channel(&mut spec_tensor, 2, &right_spec, false);
    fill_tensor_channel(&mut spec_tensor, 3, &right_spec, true);

    let tensor = Tensor::from_array(([1usize, 4, N_BINS, FRAMES], spec_tensor))
        .context("scnet input tensor")?;
    let outputs = sess
        .run(ort::inputs![input_name => tensor])
        .context("scnet ort run")?;
    let value = outputs
        .get(output_name)
        .ok_or_else(|| anyhow::anyhow!("SCNet output missing '{output_name}'"))?;
    let (shape, data) = value
        .try_extract_tensor::<f32>()
        .context("extract SCNet output")?;
    let dims: Vec<usize> = shape.iter().map(|d| *d as usize).collect();
    if dims != [1, 16, N_BINS, FRAMES] {
        bail!("unexpected SCNet output shape {dims:?}");
    }

    let plane = N_BINS * FRAMES;
    let mut stems: [[Vec<f32>; CHANNELS]; SOURCES] =
        std::array::from_fn(|_| std::array::from_fn(|_| Vec::new()));
    for (stem, channels) in stems.iter_mut().enumerate() {
        for (channel, out) in channels.iter_mut().enumerate() {
            let re = (stem * CHANNELS + channel) * 2;
            let im = re + 1;
            let mut spec = vec![Complex32::ZERO; plane];
            for f in 0..N_BINS {
                for t in 0..FRAMES {
                    spec[f * FRAMES + t] = Complex32::new(
                        data[re * plane + f * FRAMES + t],
                        data[im * plane + f * FRAMES + t],
                    );
                }
            }
            *out = istft(&spec, inverse, inverse_scratch)?;
        }
    }
    Ok(stems)
}

fn fill_tensor_channel(out: &mut [f32], channel: usize, spec: &[Complex32], imaginary: bool) {
    let plane = N_BINS * FRAMES;
    let base = channel * plane;
    for f in 0..N_BINS {
        for t in 0..FRAMES {
            let value = spec[f * FRAMES + t];
            out[base + f * FRAMES + t] = if imaginary { value.im } else { value.re };
        }
    }
}

/// Rectangular centered STFT, `normalized=True` (divide by sqrt(N)).
fn stft(
    signal: &[f32],
    fft: &dyn RealToComplex<f32>,
    scratch: &mut [Complex32],
) -> Result<Vec<Complex32>> {
    debug_assert_eq!(signal.len(), OUT_LEN);
    let padded = pad_reflect_both(signal, N_FFT / 2);
    let scale = 1.0 / (N_FFT as f32).sqrt();
    let mut input = vec![0.0f32; N_FFT];
    let mut spec = vec![Complex32::ZERO; N_BINS];
    let mut out = vec![Complex32::ZERO; N_BINS * FRAMES];
    for t in 0..FRAMES {
        let start = t * HOP;
        input.copy_from_slice(&padded[start..start + N_FFT]);
        fft.process_with_scratch(&mut input, &mut spec, scratch)?;
        for f in 0..N_BINS {
            out[f * FRAMES + t] = spec[f] * scale;
        }
    }
    Ok(out)
}

/// Inverse of [`stft`] with overlap-add and per-sample frame-count normalization.
fn istft(
    spec: &[Complex32],
    fft: &dyn ComplexToReal<f32>,
    scratch: &mut [Complex32],
) -> Result<Vec<f32>> {
    debug_assert_eq!(spec.len(), N_BINS * FRAMES);
    let sqrt_n = (N_FFT as f32).sqrt();
    let inv_n = 1.0 / N_FFT as f32;
    let mut input = vec![Complex32::ZERO; N_BINS];
    let mut frame = vec![0.0f32; N_FFT];
    let mut acc = vec![0.0f32; FULL];
    let mut count = vec![0u16; FULL];
    for t in 0..FRAMES {
        for f in 0..N_BINS {
            let value = spec[f * FRAMES + t] * sqrt_n;
            // The DC and Nyquist bins must be real for an inverse real FFT.
            input[f] = if f == 0 || f == N_BINS - 1 {
                Complex32::new(value.re, 0.0)
            } else {
                value
            };
        }
        fft.process_with_scratch(&mut input, &mut frame, scratch)?;
        let start = t * HOP;
        for k in 0..N_FFT {
            acc[start + k] += frame[k] * inv_n;
            count[start + k] += 1;
        }
    }
    let mut out = Vec::with_capacity(CHUNK);
    let start = N_FFT / 2;
    for i in 0..OUT_LEN {
        let idx = start + i;
        out.push(if count[idx] > 0 {
            acc[idx] / f32::from(count[idx])
        } else {
            0.0
        });
    }
    out.truncate(CHUNK);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stft_istft_round_trip() {
        let mut planner = RealFftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(N_FFT);
        let inverse = planner.plan_fft_inverse(N_FFT);
        let mut fscratch = vec![Complex32::ZERO; forward.get_scratch_len()];
        let mut iscratch = vec![Complex32::ZERO; inverse.get_scratch_len()];
        let signal: Vec<f32> = (0..OUT_LEN)
            .map(|i| ((i as f32) * 0.013).sin() * 0.3 + ((i as f32) * 0.0031).cos() * 0.1)
            .collect();
        let spec = stft(&signal, forward.as_ref(), &mut fscratch).expect("stft");
        let back = istft(&spec, inverse.as_ref(), &mut iscratch).expect("istft");
        let worst = signal[..CHUNK]
            .iter()
            .zip(&back)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-4, "round-trip worst {worst}");
    }

    #[test]
    fn reflect_padding_matches_pytorch() {
        let input = [1.0f32, 2.0, 3.0, 4.0];
        assert_eq!(
            pad_reflect_both(&input, 2),
            vec![3.0, 2.0, 1.0, 2.0, 3.0, 4.0, 3.0, 2.0]
        );
        assert_eq!(
            pad_reflect_right(&input, 6),
            vec![1.0, 2.0, 3.0, 4.0, 3.0, 2.0]
        );
    }

    #[test]
    fn chunk_window_fades_only_at_edges() {
        let w = chunk_window();
        assert_eq!(w.len(), CHUNK);
        assert_eq!(w[0], 0.0);
        assert_eq!(w[FADE - 1], 1.0);
        assert_eq!(w[CHUNK / 2], 1.0);
        assert_eq!(w[CHUNK - 1], 0.0);
    }
}
