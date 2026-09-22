//! Chunked HTDemucs inference over interleaved stereo PCM.
//!
//! Mirrors `demucs.apply.apply_model` (75 % overlap, triangular fade) and
//! `HTDemucs._spec` / `._ispec` (extra padding, Nyquist bin dropped, two guard
//! frames trimmed per edge) so the released weights see the layout they were
//! trained on.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use anyhow::{anyhow, bail, Context, Result};
use burn::prelude::{Backend, Tensor, TensorData};
use num_complex::Complex32;
use tracing::info;

use crate::backend::ComputeBackend;
use crate::dsp::{cac_planar_to_complex, reflect_pad, stft_to_cac_planar, Stft, HOP_LENGTH, N_FFT};
use crate::model::{HTDemucs, AUDIO_CHANNELS, SAMPLE_RATE, SOURCES, TRAINING_LENGTH};
use crate::resample_pcm::resample_interleaved_stereo;
use crate::weights::load_htdemucs_from_safetensors;
use crate::window::fill_stereo_window;

/// Frequency rows the model sees: Demucs drops the Nyquist bin.
const MODEL_BINS: usize = N_FFT / 2;
/// Frames per chunk spectrogram, `ceil(TRAINING_LENGTH / HOP_LENGTH)`.
const MODEL_FRAMES: usize = TRAINING_LENGTH.div_ceil(HOP_LENGTH);
/// Guard frames Demucs trims from each spectrogram edge.
const EDGE_FRAMES: usize = 2;
/// Frames the STFT of a [`SPEC_LEN`] buffer produces, i.e. `MODEL_FRAMES + 4`.
const SPEC_FRAMES: usize = MODEL_FRAMES + 2 * EDGE_FRAMES;
/// `hop_length // 2 * 3`: Demucs' extra pre-STFT padding.
const SPEC_PAD: usize = HOP_LENGTH / 2 * 3;
/// Length of the padded chunk that is transformed.
const SPEC_LEN: usize = MODEL_FRAMES * HOP_LENGTH + 2 * SPEC_PAD;
/// Tail padding that brings the chunk up to [`SPEC_LEN`].
const SPEC_PAD_RIGHT: usize = SPEC_LEN - SPEC_PAD - TRAINING_LENGTH;
/// 75 % overlap between consecutive chunks.
const CHUNK_STRIDE: usize = TRAINING_LENGTH * 3 / 4;
/// The checkpoint's `sources` are `drums, bass, other, vocals`;
/// [`crate::STEM_NAMES`] is `vocals, drums, bass, other`.
const SOURCE_ORDER: [usize; SOURCES] = [3, 0, 1, 2];

/// Separate `pcm` (interleaved stereo) into four interleaved stereo stems at
/// [`SAMPLE_RATE`], in [`crate::STEM_NAMES`] order.
///
/// `on_progress` receives `(chunks_done, chunks_total)`.
pub fn separate_interleaved(
    weights: &Path,
    pcm: &[f32],
    sample_rate: u32,
    backend: ComputeBackend,
    on_progress: Option<&dyn Fn(usize, usize)>,
) -> Result<[Vec<f32>; SOURCES]> {
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
        backend = backend.label(),
        weights = %weights.display(),
        "stem separate: start"
    );

    let started = Instant::now();
    let stems = match backend {
        #[cfg(feature = "burn-ndarray")]
        ComputeBackend::NdArray => separate_on_backend::<crate::NdArrayBackend>(
            weights,
            &pcm,
            false,
            ndarray_session(),
            on_progress,
        )?,
        #[cfg(not(feature = "burn-ndarray"))]
        ComputeBackend::NdArray => {
            bail!("analyzer-stems was built without `burn-ndarray`")
        }
        #[cfg(feature = "burn-wgpu")]
        ComputeBackend::Wgpu => separate_on_backend::<crate::WgpuBackend>(
            weights,
            &pcm,
            true,
            wgpu_session(),
            on_progress,
        )?,
        #[cfg(not(feature = "burn-wgpu"))]
        ComputeBackend::Wgpu => {
            bail!("analyzer-stems was built without `burn-wgpu`")
        }
    };
    info!(
        frames,
        chunks,
        backend = backend.label(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "stem separate: done"
    );
    Ok(stems)
}

/// Loaded model, reused across splits. One per process per backend: the
/// checkpoint is 84 MB and `library::stems` already serialises separations
/// behind its own lock.
struct Session<B: Backend> {
    path: PathBuf,
    device: B::Device,
    model: HTDemucs<B>,
    /// wgpu needs async readback (`pollster`); ndarray stays sync.
    async_readback: bool,
}

#[cfg(feature = "burn-ndarray")]
fn ndarray_session() -> &'static Mutex<Option<Session<crate::NdArrayBackend>>> {
    static SESSION: OnceLock<Mutex<Option<Session<crate::NdArrayBackend>>>> = OnceLock::new();
    SESSION.get_or_init(|| Mutex::new(None))
}

#[cfg(feature = "burn-wgpu")]
fn wgpu_session() -> &'static Mutex<Option<Session<crate::WgpuBackend>>> {
    static SESSION: OnceLock<Mutex<Option<Session<crate::WgpuBackend>>>> = OnceLock::new();
    SESSION.get_or_init(|| Mutex::new(None))
}

fn separate_on_backend<B: Backend>(
    weights: &Path,
    pcm: &[f32],
    async_readback: bool,
    slot: &Mutex<Option<Session<B>>>,
    on_progress: Option<&dyn Fn(usize, usize)>,
) -> Result<[Vec<f32>; SOURCES]> {
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    if guard
        .as_ref()
        .is_none_or(|s| s.path != weights || s.async_readback != async_readback)
    {
        *guard = Some(Session::load(weights, async_readback)?);
    }
    let session = guard.as_ref().expect("session just loaded");

    let mut stft = Stft::new(N_FFT, HOP_LENGTH);
    overlap_add(pcm, on_progress, |chunk| {
        session.separate_chunk(&mut stft, chunk)
    })
}

impl<B: Backend> Session<B> {
    fn load(path: &Path, async_readback: bool) -> Result<Self> {
        let started = Instant::now();
        info!(
            path = %path.display(),
            async_readback,
            "stem model: loading safetensors"
        );
        let bytes =
            std::fs::read(path).with_context(|| format!("read stem weights {}", path.display()))?;
        let device: B::Device = Default::default();
        let model = load_htdemucs_from_safetensors::<B>(&bytes, &device)
            .with_context(|| format!("load stem weights {}", path.display()))?;
        let session = Self {
            path: path.to_path_buf(),
            device,
            model,
            async_readback,
        };
        if async_readback {
            session.warmup()?;
        }
        info!(
            path = %path.display(),
            bytes = bytes.len(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "stem model: ready"
        );
        Ok(session)
    }

    /// One dummy forward so wgpu compiles shaders / autotunes before real audio.
    fn warmup(&self) -> Result<()> {
        if matches!(std::env::var("MIXAR_STEMS_SKIP_WARMUP").as_deref(), Ok("1")) {
            info!("stem model: skipping wgpu warmup (MIXAR_STEMS_SKIP_WARMUP=1)");
            return Ok(());
        }
        let started = Instant::now();
        info!("stem model: wgpu warmup start");
        let freq = Tensor::<B, 4>::zeros(
            [1, 2 * AUDIO_CHANNELS, MODEL_BINS, MODEL_FRAMES],
            &self.device,
        );
        let time = Tensor::<B, 3>::zeros([1, AUDIO_CHANNELS, TRAINING_LENGTH], &self.device);
        let (out_freq, out_time) = self.model.forward(freq, time);
        let _ = read_tensor(out_freq, self.async_readback)?;
        let _ = read_tensor(out_time, self.async_readback)?;
        info!(
            elapsed_ms = started.elapsed().as_millis() as u64,
            "stem model: wgpu warmup done"
        );
        Ok(())
    }

    /// One padded [`TRAINING_LENGTH`] chunk of interleaved stereo → four stems.
    fn separate_chunk(&self, stft: &mut Stft, chunk: &[f32]) -> Result<[Vec<f32>; SOURCES]> {
        let mut left = vec![0.0f32; TRAINING_LENGTH];
        let mut right = vec![0.0f32; TRAINING_LENGTH];
        fill_stereo_window(chunk, 2, 0, &mut left, &mut right);

        let spec_left = trim_spec(&stft.forward(&pad_for_spec(&left))?);
        let spec_right = trim_spec(&stft.forward(&pad_for_spec(&right))?);

        let freq = Tensor::<B, 4>::from_data(
            TensorData::new(
                stft_to_cac_planar(&spec_left, &spec_right, MODEL_BINS),
                [1, 2 * AUDIO_CHANNELS, MODEL_BINS, MODEL_FRAMES],
            ),
            &self.device,
        );
        let mut planar_time = left;
        planar_time.extend_from_slice(&right);
        let time = Tensor::<B, 3>::from_data(
            TensorData::new(planar_time, [1, AUDIO_CHANNELS, TRAINING_LENGTH]),
            &self.device,
        );

        let (out_freq, out_time) = self.model.forward(freq, time);
        // With channel-as-complex the "mask" head *is* the predicted spectrogram,
        // so this is inverted directly rather than multiplied by the mixture.
        let out_freq = read_tensor(out_freq, self.async_readback)?;
        let out_time = read_tensor(out_time, self.async_readback)?;

        let plane = MODEL_BINS * MODEL_FRAMES;
        let mut spec = vec![Complex32::default(); (N_FFT / 2 + 1) * SPEC_FRAMES];
        let mut stems = std::array::from_fn(|_| Vec::new());
        for (stem, &source) in SOURCE_ORDER.iter().enumerate() {
            let mut waves: [Vec<f32>; AUDIO_CHANNELS] = std::array::from_fn(|_| Vec::new());
            for (channel, wave) in waves.iter_mut().enumerate() {
                // Output channels are `[source][audio channel][real, imag]`.
                let base = (source * 2 * AUDIO_CHANNELS + channel * 2) * plane;
                let predicted = cac_planar_to_complex(&out_freq[base..], MODEL_BINS, MODEL_FRAMES);
                restore_spec(&predicted, &mut spec);
                let freq_wave = stft.inverse(&spec, SPEC_LEN)?;

                let time_base = (source * AUDIO_CHANNELS + channel) * TRAINING_LENGTH;
                *wave = freq_wave[SPEC_PAD..SPEC_PAD + TRAINING_LENGTH]
                    .iter()
                    .zip(&out_time[time_base..time_base + TRAINING_LENGTH])
                    .map(|(f, t)| f + t)
                    .collect();
            }
            stems[stem] = waves[0]
                .iter()
                .zip(&waves[1])
                .flat_map(|(l, r)| [*l, *r])
                .collect();
        }
        Ok(stems)
    }
}

fn read_tensor<B: Backend, const D: usize>(
    tensor: Tensor<B, D>,
    async_readback: bool,
) -> Result<Vec<f32>> {
    let data = if async_readback {
        #[cfg(feature = "burn-wgpu")]
        {
            pollster::block_on(tensor.into_data_async())
                .map_err(|e| anyhow!("read HTDemucs output tensor: {e:?}"))?
        }
        #[cfg(not(feature = "burn-wgpu"))]
        {
            let _ = async_readback;
            tensor.into_data()
        }
    } else {
        tensor.into_data()
    };
    data.to_vec::<f32>()
        .map_err(|e| anyhow!("read HTDemucs output tensor: {e:?}"))
}

/// Demucs `_spec` padding, so the kept frames land where training put them.
fn pad_for_spec(wave: &[f32]) -> Vec<f32> {
    reflect_pad(wave, SPEC_PAD, SPEC_PAD_RIGHT)
}

/// Drop the Nyquist bin and the guard frames: `[bins, SPEC_FRAMES]` → `[MODEL_BINS, MODEL_FRAMES]`.
fn trim_spec(spec: &[Complex32]) -> Vec<Complex32> {
    let mut out = vec![Complex32::default(); MODEL_BINS * MODEL_FRAMES];
    for f in 0..MODEL_BINS {
        out[f * MODEL_FRAMES..][..MODEL_FRAMES]
            .copy_from_slice(&spec[f * SPEC_FRAMES + EDGE_FRAMES..][..MODEL_FRAMES]);
    }
    out
}

/// Inverse of [`trim_spec`]: zeros for the Nyquist bin and the guard frames.
///
/// The model predicts a free complex value for DC, but a real signal has none
/// there; `irfft` discards it, so drop it here instead of letting `realfft`
/// report it as a bad input.
fn restore_spec(model_spec: &[Complex32], spec: &mut [Complex32]) {
    spec.fill(Complex32::default());
    for f in 0..MODEL_BINS {
        spec[f * SPEC_FRAMES + EDGE_FRAMES..][..MODEL_FRAMES]
            .copy_from_slice(&model_spec[f * MODEL_FRAMES..][..MODEL_FRAMES]);
    }
    for dc in &mut spec[..SPEC_FRAMES] {
        dc.im = 0.0;
    }
}

/// Triangular fade, peaking mid-chunk and normalised to 1 (`transition_power = 1`).
fn chunk_weights() -> Vec<f32> {
    let half = TRAINING_LENGTH / 2;
    (0..TRAINING_LENGTH)
        .map(|i| {
            let raw = if i < half { i + 1 } else { TRAINING_LENGTH - i };
            raw as f32 / half as f32
        })
        .collect()
}

/// Run `separate_chunk` over 75 %-overlapping [`TRAINING_LENGTH`] chunks and
/// blend the results back into full-length stems.
///
/// Every chunk is zero-padded to the full length even at the tail:
/// `burn-ndarray`'s SIMD convolutions index out of bounds on short segments.
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
    let mut chunk = vec![0.0f32; TRAINING_LENGTH * 2];
    let total = frames.div_ceil(CHUNK_STRIDE);

    // Announce total before the first (slow) CPU forward so the UI knows work started.
    if let Some(report) = on_progress {
        report(0, total);
    }

    for index in 0..total {
        let offset = index * CHUNK_STRIDE;
        let valid = (frames - offset).min(TRAINING_LENGTH);
        chunk.fill(0.0);
        chunk[..valid * 2].copy_from_slice(&pcm[offset * 2..(offset + valid) * 2]);

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

    /// Interleaved stereo test tone, deep enough that HTDemucs should call it bass.
    fn tone(frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32 * std::f32::consts::TAU;
                [(t * 35.0).sin() * 0.3, (t * 52.5).sin() * 0.2]
            })
            .collect()
    }

    /// The fade weights must sum to the same thing everywhere the signal exists,
    /// otherwise overlap-add would dip at the chunk seams.
    #[test]
    fn overlap_add_reconstructs_a_passthrough_chunker() {
        let frames = TRAINING_LENGTH + CHUNK_STRIDE + 517;
        let pcm = tone(frames);
        let mut seen = Vec::new();
        let stems = overlap_add(
            &pcm,
            Some(&|done, total| {
                assert_eq!(total, 3);
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

        assert_eq!(seen, vec![TRAINING_LENGTH * 2; 3]);
        assert_eq!(stems[0].len(), pcm.len());
        let worst = pcm
            .iter()
            .zip(&stems[0])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-5, "max reconstruction error {worst}");
        assert!(stems[1].iter().all(|v| *v == 0.0));
    }

    #[test]
    fn overlap_add_handles_audio_shorter_than_one_chunk() {
        let pcm = tone(1_000);
        let stems = overlap_add(&pcm, None, |chunk| {
            Ok(std::array::from_fn(|_| chunk.to_vec()))
        })
        .expect("overlap add");
        assert_eq!(stems[3].len(), pcm.len());
        let worst = pcm
            .iter()
            .zip(&stems[3])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-5, "max reconstruction error {worst}");
    }

    #[test]
    fn overlap_add_on_empty_pcm_is_empty() {
        let stems = overlap_add(&[], None, |_| unreachable!("no chunks")).expect("overlap add");
        assert!(stems.iter().all(|s| s.is_empty()));
    }

    /// Pad → STFT → trim → restore → iSTFT → unpad is the contract the model
    /// sits inside; if it is not the identity, every stem comes out smeared.
    #[test]
    fn spec_roundtrip_is_lossless_away_from_the_chunk_edges() {
        let wave: Vec<f32> = (0..TRAINING_LENGTH)
            .map(|i| (i as f32 * 0.013).sin() * 0.5)
            .collect();
        let mut stft = Stft::new(N_FFT, HOP_LENGTH);

        let full = stft.forward(&pad_for_spec(&wave)).expect("stft");
        assert_eq!(full.len(), (N_FFT / 2 + 1) * SPEC_FRAMES);

        let trimmed = trim_spec(&full);
        assert_eq!(trimmed.len(), MODEL_BINS * MODEL_FRAMES);

        let mut spec = vec![Complex32::default(); (N_FFT / 2 + 1) * SPEC_FRAMES];
        restore_spec(&trimmed, &mut spec);
        let back = stft.inverse(&spec, SPEC_LEN).expect("istft");
        let chunk = &back[SPEC_PAD..SPEC_PAD + TRAINING_LENGTH];
        // The dropped guard frames cost the outer ~N_FFT samples of every chunk;
        // `overlap_add`'s triangular fade is what hides them.
        let worst = wave[N_FFT..TRAINING_LENGTH - N_FFT]
            .iter()
            .zip(&chunk[N_FFT..])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-4, "max roundtrip error {worst}");
    }

    /// Unlike a measured spectrogram, the model emits an imaginary DC term;
    /// `realfft` rejects one, so the inverse must survive it.
    #[test]
    fn istft_accepts_a_predicted_spectrogram_with_an_imaginary_dc_term() {
        let predicted: Vec<Complex32> = (0..MODEL_BINS * MODEL_FRAMES)
            .map(|i| Complex32::new((i % 7) as f32 * 0.01, (i % 5) as f32 * 0.01))
            .collect();
        let mut spec = vec![Complex32::default(); (N_FFT / 2 + 1) * SPEC_FRAMES];
        restore_spec(&predicted, &mut spec);

        let wave = Stft::new(N_FFT, HOP_LENGTH)
            .inverse(&spec, SPEC_LEN)
            .expect("istft of a predicted spectrogram");
        assert_eq!(wave.len(), SPEC_LEN);
        assert!(wave.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn chunk_weights_fade_in_and_out() {
        let w = chunk_weights();
        assert_eq!(w.len(), TRAINING_LENGTH);
        assert_eq!(w[TRAINING_LENGTH / 2 - 1], 1.0);
        assert!(w[0] < 1e-4 && w[TRAINING_LENGTH - 1] < 1e-4);
        assert!(w.iter().all(|v| *v > 0.0 && *v <= 1.0));
    }

    #[test]
    #[ignore = "downloads the ~84 MB HTDemucs checkpoint and runs a full CPU forward"]
    fn separates_a_sine_with_real_weights() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = std::env::var_os("MIXAR_MODELS_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| temp.path().to_path_buf());
        let handle =
            crate::resolve_model(&root, crate::DEFAULT_MODEL, None).expect("resolve weights");

        let pcm = tone(SAMPLE_RATE as usize); // 1 s
        let stems = separate_interleaved(
            &handle.local_path,
            &pcm,
            SAMPLE_RATE,
            ComputeBackend::NdArray,
            Some(&|done, total| {
                assert_eq!(total, 1);
                assert_eq!(done, 1);
            }),
        )
        .expect("separate");

        let rms: Vec<f32> = stems
            .iter()
            .map(|stem| {
                assert_eq!(stem.len(), pcm.len());
                assert!(stem.iter().all(|v| v.is_finite()));
                (stem.iter().map(|v| v * v).sum::<f32>() / stem.len() as f32).sqrt()
            })
            .collect();
        for (name, rms) in crate::STEM_NAMES.iter().zip(&rms) {
            eprintln!("{name}: rms {rms}");
        }

        // HTDemucs is near-conservative: the stems add back up to the mixture,
        // which only holds if the CaC decode, the iSTFT alignment and the time
        // branch are all wired correctly.
        let residual: f32 = pcm
            .iter()
            .enumerate()
            .map(|(i, m)| (m - stems.iter().map(|s| s[i]).sum::<f32>()).powi(2))
            .sum();
        let energy: f32 = pcm.iter().map(|m| m * m).sum();
        assert!(
            residual / energy < 0.01,
            "stems do not sum back to the mixture: {}",
            residual / energy
        );

        // A 35 Hz tone belongs to bass; anything else means `SOURCE_ORDER` is off.
        let loudest = rms
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| crate::STEM_NAMES[i]);
        assert_eq!(loudest, Some("bass"), "stem order looks rotated: {rms:?}");
    }
}
