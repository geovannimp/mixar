//! HTDemucs v4 forward graph.

use anyhow::{Context, Result};
use burn::prelude::{Backend, Tensor};

use super::{
    CrossTransformer, FreqDecLayer, FreqEncLayer, TimeDecLayer, TimeEncLayer, DEPTH,
    FREQ_EMB_SCALE, FREQ_EMB_WEIGHT_SCALE,
};
use crate::weights::{Conv1dW, TensorStore};

/// `1e-5` floor added to the input standard deviations, as in the reference model.
const NORM_FLOOR: f32 = 1e-5;

/// Hybrid Transformer Demucs, standard 4-stem.
pub struct HTDemucs<B: Backend> {
    encoder: Vec<FreqEncLayer<B>>,
    tencoder: Vec<TimeEncLayer<B>>,
    decoder: Vec<FreqDecLayer<B>>,
    tdecoder: Vec<TimeDecLayer<B>>,
    /// `ScaledEmbedding` table, `[bins, channels]`.
    freq_emb: Tensor<B, 2>,
    channel_upsampler: Conv1dW<B>,
    channel_downsampler: Conv1dW<B>,
    channel_upsampler_t: Conv1dW<B>,
    channel_downsampler_t: Conv1dW<B>,
    /// Transformer width (`bottom_channels`), read from the upsampler weight.
    bottom_channels: usize,
    crosstransformer: CrossTransformer<B>,
}

impl<B: Backend> HTDemucs<B> {
    /// Build every layer from an already-parsed checkpoint.
    pub fn from_store(store: &TensorStore<'_>, device: &B::Device) -> Result<Self> {
        let encoder = (0..DEPTH)
            .map(|i| FreqEncLayer::load(store, &format!("encoder.{i}"), device))
            .collect::<Result<Vec<_>>>()
            .context("frequency encoder")?;
        let tencoder = (0..DEPTH)
            .map(|i| TimeEncLayer::load(store, &format!("tencoder.{i}"), device))
            .collect::<Result<Vec<_>>>()
            .context("time encoder")?;
        // The reference builds decoders innermost-first, so only the final entry is `last`.
        let decoder = (0..DEPTH)
            .map(|i| FreqDecLayer::load(store, &format!("decoder.{i}"), i + 1 == DEPTH, device))
            .collect::<Result<Vec<_>>>()
            .context("frequency decoder")?;
        let tdecoder = (0..DEPTH)
            .map(|i| TimeDecLayer::load(store, &format!("tdecoder.{i}"), i + 1 == DEPTH, device))
            .collect::<Result<Vec<_>>>()
            .context("time decoder")?;

        let sampler = |name: &str| Conv1dW::load(store, name, 1, 0, 1, device);
        let channel_upsampler = sampler("channel_upsampler")?;
        Ok(Self {
            encoder,
            tencoder,
            decoder,
            tdecoder,
            freq_emb: store.take("freq_emb.embedding.weight", device)?,
            bottom_channels: channel_upsampler.out_channels(),
            channel_upsampler,
            channel_downsampler: sampler("channel_downsampler")?,
            channel_upsampler_t: sampler("channel_upsampler_t")?,
            channel_downsampler_t: sampler("channel_downsampler_t")?,
            crosstransformer: CrossTransformer::from_store(store, device)?,
        })
    }

    /// Separate one padded segment.
    ///
    /// `freq` is the channel-as-complex spectrogram `[1, 4, bins, frames]`
    /// (see [`crate::dsp`]); `time` is the matching waveform `[1, 2, samples]`.
    /// Returns per-source masks `[1, 16, bins, frames]` and waveforms
    /// `[1, 8, samples]`, both with the source index folded into the channel
    /// axis and de-normalised, ready for iSTFT and overlap-add.
    pub fn forward(&self, freq: Tensor<B, 4>, time: Tensor<B, 3>) -> (Tensor<B, 4>, Tensor<B, 3>) {
        let (mut x, freq_mean, freq_std) = normalize_freq(freq);
        let (mut xt, time_mean, time_std) = normalize_time(time);

        let mut skips = Vec::with_capacity(DEPTH);
        let mut skips_t = Vec::with_capacity(DEPTH);
        let mut lengths_t = Vec::with_capacity(DEPTH);

        for level in 0..DEPTH {
            lengths_t.push(xt.dims()[2]);
            xt = self.tencoder[level].forward(xt);
            skips_t.push(xt.clone());

            x = self.encoder[level].forward(x);
            if level == 0 {
                let embedding = self.frequency_embedding(x.dims());
                x = x + embedding;
            }
            skips.push(x.clone());
        }

        let [batch, channels, bins, frames] = x.dims();
        let bottom = self.bottom_channels;
        let wide = self
            .channel_upsampler
            .forward(x.reshape([batch, channels, bins * frames]))
            .reshape([batch, bottom, bins, frames]);
        let (wide, wide_t) = self
            .crosstransformer
            .forward(wide, self.channel_upsampler_t.forward(xt));
        x = self
            .channel_downsampler
            .forward(wide.reshape([batch, bottom, bins * frames]))
            .reshape([batch, channels, bins, frames]);
        xt = self.channel_downsampler_t.forward(wide_t);

        for level in 0..DEPTH {
            x = self.decoder[level].forward(x, skips.pop().expect("freq skip per level"));
            xt = self.tdecoder[level].forward(
                xt,
                skips_t.pop().expect("time skip per level"),
                lengths_t.pop().expect("time length per level"),
            );
        }

        (x * freq_std + freq_mean, xt * time_std + time_mean)
    }

    /// `freq_emb`: per-bin embedding broadcast over batch and frames.
    fn frequency_embedding(&self, [_, channels, bins, _]: [usize; 4]) -> Tensor<B, 4> {
        let [table_bins, table_channels] = self.freq_emb.dims();
        assert!(
            bins <= table_bins && channels == table_channels,
            "freq_emb table is [{table_bins}, {table_channels}], encoder produced {bins} bins \
             of {channels} channels"
        );
        self.freq_emb
            .clone()
            .narrow(0, 0, bins)
            .transpose()
            .reshape([1, channels, bins, 1])
            .mul_scalar(FREQ_EMB_SCALE * FREQ_EMB_WEIGHT_SCALE)
    }
}

/// Normalise over channels, bins and frames; returns the broadcastable moments.
fn normalize_freq<B: Backend>(x: Tensor<B, 4>) -> (Tensor<B, 4>, Tensor<B, 4>, Tensor<B, 4>) {
    let [batch, channels, bins, frames] = x.dims();
    let flat = x.clone().reshape([batch, channels * bins * frames]);
    let mean = flat.clone().mean_dim(1).reshape([batch, 1, 1, 1]);
    let std = flat.var(1).sqrt().reshape([batch, 1, 1, 1]);
    let normed = (x - mean.clone()) / (std.clone() + NORM_FLOOR);
    (normed, mean, std)
}

/// Normalise over channels and samples; returns the broadcastable moments.
fn normalize_time<B: Backend>(x: Tensor<B, 3>) -> (Tensor<B, 3>, Tensor<B, 3>, Tensor<B, 3>) {
    let [batch, channels, samples] = x.dims();
    let flat = x.clone().reshape([batch, channels * samples]);
    let mean = flat.clone().mean_dim(1).reshape([batch, 1, 1]);
    let std = flat.var(1).sqrt().reshape([batch, 1, 1]);
    let normed = (x - mean.clone()) / (std.clone() + NORM_FLOOR);
    (normed, mean, std)
}

#[cfg(all(test, feature = "burn-ndarray"))]
mod tests {
    use super::*;
    use crate::model::{AUDIO_CHANNELS, SOURCES, TRAINING_LENGTH, T_LAYERS};
    use crate::weights::HTDEMUCS_SIGNATURE;
    use crate::NdArrayBackend as B;
    use burn::prelude::TensorData;

    /// Scaled-down stand-in for the published checkpoint: same layer graph and
    /// key names, 1/6th the channel width, so a real forward pass is cheap.
    mod tiny {
        /// `DConv` compresses by 8, so 16 keeps its hidden width above 1 —
        /// `burn-ndarray`'s SIMD conv1d mis-indexes single-channel inputs, which
        /// the real model (hidden width 6 and up) never hits.
        pub const CHANNELS: usize = 16;
        pub const BOTTOM: usize = 32;
        /// Bins and samples must stay long enough that the deepest encoder output
        /// exceeds the conv padding of 2: `burn-ndarray`'s SIMD conv walks
        /// `0..pad` rows unconditionally and indexes out of bounds otherwise.
        /// Real segments (2048 bins, 343980 samples) clear this by a wide margin.
        pub const BINS: usize = 1024;
        pub const FRAMES: usize = 8;
        pub const SAMPLES: usize = 2048;
        /// Frequency bins surviving encoder 0, which sizes the `freq_emb` table.
        pub const EMB_BINS: usize = BINS / crate::model::STRIDE;
    }

    /// Deterministic small weights: enough structure to expose wiring bugs,
    /// small enough that 13 layers of residuals stay in range.
    fn filler(seed: usize, count: usize) -> Vec<f32> {
        (0..count)
            .map(|i| {
                let h = (seed.wrapping_mul(0x9E37_79B9) ^ i.wrapping_mul(0x85EB_CA6B)) % 2_003;
                (h as f32 / 2_003.0 - 0.5) * 0.1
            })
            .collect()
    }

    /// Append `name` with `shape` filled by [`filler`].
    fn push(entries: &mut Vec<(String, Vec<usize>, Vec<f32>)>, name: String, shape: Vec<usize>) {
        let count = shape.iter().product();
        let values = filler(entries.len() + 1, count);
        entries.push((name, shape, values));
    }

    /// `DConv` parameters for a branch of `channels` width.
    fn push_dconv(entries: &mut Vec<(String, Vec<usize>, Vec<f32>)>, path: &str, channels: usize) {
        let hidden = channels / 8;
        for d in 0..2 {
            let layer = format!("{path}.dconv.layers.{d}");
            push(
                entries,
                format!("{layer}.0.weight"),
                vec![hidden, channels, 3],
            );
            push(entries, format!("{layer}.0.bias"), vec![hidden]);
            push(entries, format!("{layer}.1.weight"), vec![hidden]);
            push(entries, format!("{layer}.1.bias"), vec![hidden]);
            push(
                entries,
                format!("{layer}.3.weight"),
                vec![2 * channels, hidden, 1],
            );
            push(entries, format!("{layer}.3.bias"), vec![2 * channels]);
            push(entries, format!("{layer}.4.weight"), vec![2 * channels]);
            push(entries, format!("{layer}.4.bias"), vec![2 * channels]);
            push(entries, format!("{layer}.6.scale"), vec![channels]);
        }
    }

    fn push_attn_layer(entries: &mut Vec<(String, Vec<usize>, Vec<f32>)>, path: &str, cross: bool) {
        let dim = tiny::BOTTOM;
        let attn = if cross { "cross_attn" } else { "self_attn" };
        let mut norms = vec!["norm1", "norm2", "norm_out"];
        if cross {
            norms.push("norm3");
        }
        for norm in norms {
            push(entries, format!("{path}.{norm}.weight"), vec![dim]);
            push(entries, format!("{path}.{norm}.bias"), vec![dim]);
        }
        push(
            entries,
            format!("{path}.{attn}.in_proj_weight"),
            vec![3 * dim, dim],
        );
        push(
            entries,
            format!("{path}.{attn}.in_proj_bias"),
            vec![3 * dim],
        );
        push(
            entries,
            format!("{path}.{attn}.out_proj.weight"),
            vec![dim, dim],
        );
        push(entries, format!("{path}.{attn}.out_proj.bias"), vec![dim]);
        push(
            entries,
            format!("{path}.linear1.weight"),
            vec![4 * dim, dim],
        );
        push(entries, format!("{path}.linear1.bias"), vec![4 * dim]);
        push(
            entries,
            format!("{path}.linear2.weight"),
            vec![dim, 4 * dim],
        );
        push(entries, format!("{path}.linear2.bias"), vec![dim]);
        push(entries, format!("{path}.gamma_1.scale"), vec![dim]);
        push(entries, format!("{path}.gamma_2.scale"), vec![dim]);
    }

    fn tiny_checkpoint() -> Vec<u8> {
        let c = tiny::CHANNELS;
        let mut entries = Vec::new();

        // Encoders: frequency branch starts from 4 CaC planes, time branch from stereo.
        let mut freq_in = 2 * AUDIO_CHANNELS;
        let mut time_in = AUDIO_CHANNELS;
        for level in 0..DEPTH {
            let out = c << level;
            let enc = format!("encoder.{level}");
            push(
                &mut entries,
                format!("{enc}.conv.weight"),
                vec![out, freq_in, 8, 1],
            );
            push(&mut entries, format!("{enc}.conv.bias"), vec![out]);
            push(
                &mut entries,
                format!("{enc}.rewrite.weight"),
                vec![2 * out, out, 1, 1],
            );
            push(&mut entries, format!("{enc}.rewrite.bias"), vec![2 * out]);
            push_dconv(&mut entries, &enc, out);

            let tenc = format!("tencoder.{level}");
            push(
                &mut entries,
                format!("{tenc}.conv.weight"),
                vec![out, time_in, 8],
            );
            push(&mut entries, format!("{tenc}.conv.bias"), vec![out]);
            push(
                &mut entries,
                format!("{tenc}.rewrite.weight"),
                vec![2 * out, out, 1],
            );
            push(&mut entries, format!("{tenc}.rewrite.bias"), vec![2 * out]);
            push_dconv(&mut entries, &tenc, out);

            freq_in = out;
            time_in = out;
        }

        // Decoders run outermost-last, so index 3 emits the per-source channels.
        for index in 0..DEPTH {
            let level = DEPTH - 1 - index;
            let chin = c << level;
            let (freq_out, time_out) = if level == 0 {
                (SOURCES * 2 * AUDIO_CHANNELS, SOURCES * AUDIO_CHANNELS)
            } else {
                (c << (level - 1), c << (level - 1))
            };
            let dec = format!("decoder.{index}");
            push(
                &mut entries,
                format!("{dec}.conv_tr.weight"),
                vec![chin, freq_out, 8, 1],
            );
            push(&mut entries, format!("{dec}.conv_tr.bias"), vec![freq_out]);
            push(
                &mut entries,
                format!("{dec}.rewrite.weight"),
                vec![2 * chin, chin, 3, 3],
            );
            push(&mut entries, format!("{dec}.rewrite.bias"), vec![2 * chin]);
            push_dconv(&mut entries, &dec, chin);

            let tdec = format!("tdecoder.{index}");
            push(
                &mut entries,
                format!("{tdec}.conv_tr.weight"),
                vec![chin, time_out, 8],
            );
            push(&mut entries, format!("{tdec}.conv_tr.bias"), vec![time_out]);
            push(
                &mut entries,
                format!("{tdec}.rewrite.weight"),
                vec![2 * chin, chin, 3],
            );
            push(&mut entries, format!("{tdec}.rewrite.bias"), vec![2 * chin]);
            push_dconv(&mut entries, &tdec, chin);
        }

        push(
            &mut entries,
            "freq_emb.embedding.weight".into(),
            vec![tiny::EMB_BINS, c],
        );

        let deep = c << (DEPTH - 1);
        for suffix in ["", "_t"] {
            push(
                &mut entries,
                format!("channel_upsampler{suffix}.weight"),
                vec![tiny::BOTTOM, deep, 1],
            );
            push(
                &mut entries,
                format!("channel_upsampler{suffix}.bias"),
                vec![tiny::BOTTOM],
            );
            push(
                &mut entries,
                format!("channel_downsampler{suffix}.weight"),
                vec![deep, tiny::BOTTOM, 1],
            );
            push(
                &mut entries,
                format!("channel_downsampler{suffix}.bias"),
                vec![deep],
            );
        }

        for suffix in ["", "_t"] {
            push(
                &mut entries,
                format!("crosstransformer.norm_in{suffix}.weight"),
                vec![tiny::BOTTOM],
            );
            push(
                &mut entries,
                format!("crosstransformer.norm_in{suffix}.bias"),
                vec![tiny::BOTTOM],
            );
        }
        for branch in ["layers", "layers_t"] {
            for i in 0..T_LAYERS {
                push_attn_layer(
                    &mut entries,
                    &format!("crosstransformer.{branch}.{i}"),
                    i % 2 == 1,
                );
            }
        }

        serialize_f32(entries)
    }

    fn serialize_f32(entries: Vec<(String, Vec<usize>, Vec<f32>)>) -> Vec<u8> {
        let raw: Vec<(String, Vec<usize>, Vec<u8>)> = entries
            .into_iter()
            .map(|(name, shape, values)| {
                let bytes = values.iter().flat_map(|v| v.to_le_bytes()).collect();
                (format!("{HTDEMUCS_SIGNATURE}.{name}"), shape, bytes)
            })
            .collect();
        let views: Vec<_> = raw
            .iter()
            .map(|(name, shape, bytes)| {
                (
                    name.clone(),
                    safetensors::tensor::TensorView::new(
                        safetensors::Dtype::F32,
                        shape.clone(),
                        bytes,
                    )
                    .expect("view"),
                )
            })
            .collect();
        safetensors::serialize(views, &None).expect("serialize")
    }

    #[test]
    fn forward_runs_the_full_graph_and_stays_finite() {
        let blob = tiny_checkpoint();
        let device = Default::default();
        let model = crate::load_htdemucs_from_safetensors::<B>(&blob, &device).expect("load");

        let freq_len = 2 * AUDIO_CHANNELS * tiny::BINS * tiny::FRAMES;
        let freq = Tensor::<B, 4>::from_data(
            TensorData::new(
                (0..freq_len)
                    .map(|i| (i as f32 * 0.017).sin())
                    .collect::<Vec<_>>(),
                [1, 2 * AUDIO_CHANNELS, tiny::BINS, tiny::FRAMES],
            ),
            &device,
        );
        let time = Tensor::<B, 3>::from_data(
            TensorData::new(
                (0..AUDIO_CHANNELS * tiny::SAMPLES)
                    .map(|i| (i as f32 * 0.031).cos())
                    .collect::<Vec<_>>(),
                [1, AUDIO_CHANNELS, tiny::SAMPLES],
            ),
            &device,
        );

        let (out_freq, out_time) = model.forward(freq, time);
        assert_eq!(
            out_freq.dims(),
            [1, SOURCES * 2 * AUDIO_CHANNELS, tiny::BINS, tiny::FRAMES]
        );
        assert_eq!(
            out_time.dims(),
            [1, SOURCES * AUDIO_CHANNELS, tiny::SAMPLES]
        );

        for (label, values) in [
            (
                "freq",
                out_freq.into_data().to_vec::<f32>().expect("freq data"),
            ),
            (
                "time",
                out_time.into_data().to_vec::<f32>().expect("time data"),
            ),
        ] {
            assert!(
                values.iter().all(|v| v.is_finite()),
                "{label} output contains NaN or inf"
            );
            assert!(
                values.iter().any(|v| v.abs() > 1e-6),
                "{label} output is all zeros, the graph is not wired to the weights"
            );
        }
    }

    #[test]
    fn load_reports_the_missing_key() {
        let mut entries: Vec<(String, Vec<usize>, Vec<f32>)> = Vec::new();
        push(
            &mut entries,
            "encoder.0.conv.weight".into(),
            vec![8, 4, 8, 1],
        );
        let blob = serialize_f32(entries);
        let error = crate::load_htdemucs_from_safetensors::<B>(&blob, &Default::default())
            .err()
            .expect("truncated checkpoint must fail");
        assert!(
            format!("{error:#}").contains("encoder.0.conv.bias"),
            "unhelpful error: {error:#}"
        );
    }

    #[test]
    #[ignore = "downloads the ~84 MB HTDemucs checkpoint"]
    fn loads_htdemucs_safetensors_from_models_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = std::env::var_os("MIXAR_MODELS_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| temp.path().to_path_buf());
        let handle =
            crate::resolve_model(&root, crate::DEFAULT_MODEL, None).expect("resolve weights");
        let bytes = std::fs::read(&handle.local_path).expect("read weights");

        let store = crate::TensorStore::new(&bytes).expect("store");
        assert_eq!(store.prefix(), HTDEMUCS_SIGNATURE);
        assert_eq!(store.len(), 533);

        let device = Default::default();
        let model = HTDemucs::<B>::from_store(&store, &device).expect("load");

        // Short segment: enough to prove the real weight shapes compose.
        let bins = crate::N_FFT / 2;
        let frames = 4;
        let samples = TRAINING_LENGTH / 64;
        let ramp = |len: usize, step: f32| {
            (0..len)
                .map(|i| (i as f32 * step).sin())
                .collect::<Vec<_>>()
        };
        let (freq, time) = model.forward(
            Tensor::from_data(
                TensorData::new(
                    ramp(2 * AUDIO_CHANNELS * bins * frames, 0.013),
                    [1, 2 * AUDIO_CHANNELS, bins, frames],
                ),
                &device,
            ),
            Tensor::from_data(
                TensorData::new(
                    ramp(AUDIO_CHANNELS * samples, 0.021),
                    [1, AUDIO_CHANNELS, samples],
                ),
                &device,
            ),
        );
        assert_eq!(freq.dims(), [1, SOURCES * 2 * AUDIO_CHANNELS, bins, frames]);
        assert_eq!(time.dims(), [1, SOURCES * AUDIO_CHANNELS, samples]);
        for values in [
            freq.into_data().to_vec::<f32>().expect("freq data"),
            time.into_data().to_vec::<f32>().expect("time data"),
        ] {
            assert!(values.iter().all(|v| v.is_finite()), "non-finite output");
        }
    }
}
