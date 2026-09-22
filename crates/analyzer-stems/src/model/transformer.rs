//! Cross-domain transformer joining the frequency and time branches.
//!
//! Mirrors `CrossTransformerEncoder` from public HTDemucs v4 with the released
//! config: `t_norm_first`, `t_layer_scale`, `t_norm_out`, sinusoidal embeddings,
//! `t_cross_first = false` (so even layers self-attend, odd layers cross-attend).

use anyhow::Result;
use burn::prelude::{Backend, Tensor, TensorData};
use burn::tensor::activation::{gelu, softmax};

use super::{BOTTOM_CHANNELS, T_HEADS, T_LAYERS, T_MAX_PERIOD};
use crate::weights::{GroupNorm1W, LayerNormW, LayerScaleW, LinearW, TensorStore};

/// `nn.MultiheadAttention` with the fused `in_proj_weight` split into q/k/v.
struct AttnW<B: Backend> {
    query: LinearW<B>,
    key: LinearW<B>,
    value: LinearW<B>,
    out: LinearW<B>,
}

impl<B: Backend> AttnW<B> {
    fn load(store: &TensorStore<'_>, path: &str, device: &B::Device) -> Result<Self> {
        let weight: Tensor<B, 2> = store.take(&format!("{path}.in_proj_weight"), device)?;
        let bias: Tensor<B, 1> = store.take(&format!("{path}.in_proj_bias"), device)?;
        let dim = weight.dims()[1];
        let part = |i: usize| {
            LinearW::from_parts(
                weight.clone().narrow(0, i * dim, dim),
                bias.clone().narrow(0, i * dim, dim),
            )
        };
        Ok(Self {
            query: part(0),
            key: part(1),
            value: part(2),
            out: LinearW::load(store, &format!("{path}.out_proj"), device)?,
        })
    }

    fn forward(&self, q_src: Tensor<B, 3>, kv_src: Tensor<B, 3>) -> Tensor<B, 3> {
        let [batch, tokens, dim] = q_src.dims();
        let head_dim = dim / T_HEADS;
        let split = |x: Tensor<B, 3>| {
            let len = x.dims()[1];
            x.reshape([batch, len, T_HEADS, head_dim]).swap_dims(1, 2)
        };
        let q = split(self.query.forward(q_src));
        let k = split(self.key.forward(kv_src.clone()));
        let v = split(self.value.forward(kv_src));

        let scores = q
            .matmul(k.swap_dims(2, 3))
            .div_scalar((head_dim as f32).sqrt());
        let context = softmax(scores, 3)
            .matmul(v)
            .swap_dims(1, 2)
            .reshape([batch, tokens, dim]);
        self.out.forward(context)
    }
}

/// One transformer layer. Self-attention layers reuse `norm_q` for keys;
/// cross-attention layers normalise keys with their own `norm2`.
struct AttnLayer<B: Backend> {
    norm_q: LayerNormW<B>,
    norm_kv: Option<LayerNormW<B>>,
    norm_ff: LayerNormW<B>,
    attn: AttnW<B>,
    linear1: LinearW<B>,
    linear2: LinearW<B>,
    gamma_1: LayerScaleW<B>,
    gamma_2: LayerScaleW<B>,
    norm_out: GroupNorm1W<B>,
}

impl<B: Backend> AttnLayer<B> {
    fn load(store: &TensorStore<'_>, path: &str, cross: bool, device: &B::Device) -> Result<Self> {
        let (attn_name, norm_kv, ff_norm) = if cross {
            (
                "cross_attn",
                Some(LayerNormW::load(store, &format!("{path}.norm2"), device)?),
                "norm3",
            )
        } else {
            ("self_attn", None, "norm2")
        };
        Ok(Self {
            norm_q: LayerNormW::load(store, &format!("{path}.norm1"), device)?,
            norm_kv,
            norm_ff: LayerNormW::load(store, &format!("{path}.{ff_norm}"), device)?,
            attn: AttnW::load(store, &format!("{path}.{attn_name}"), device)?,
            linear1: LinearW::load(store, &format!("{path}.linear1"), device)?,
            linear2: LinearW::load(store, &format!("{path}.linear2"), device)?,
            gamma_1: LayerScaleW::load(store, &format!("{path}.gamma_1"), device)?,
            gamma_2: LayerScaleW::load(store, &format!("{path}.gamma_2"), device)?,
            norm_out: GroupNorm1W::load(store, &format!("{path}.norm_out"), device)?,
        })
    }

    fn forward(&self, q: Tensor<B, 3>, kv: Option<Tensor<B, 3>>) -> Tensor<B, 3> {
        let normed_q = self.norm_q.forward(q.clone());
        let normed_kv = match (&self.norm_kv, kv) {
            (Some(norm), Some(kv)) => norm.forward(kv),
            _ => normed_q.clone(),
        };
        let x = q + self
            .gamma_1
            .forward_channels_last(self.attn.forward(normed_q, normed_kv));

        let ff = self
            .linear2
            .forward(gelu(self.linear1.forward(self.norm_ff.forward(x.clone()))));
        let x = x + self.gamma_2.forward_channels_last(ff);
        self.norm_out.forward_channels_last(x)
    }
}

/// Interleaved self/cross attention over both branches.
pub struct CrossTransformer<B: Backend> {
    norm_in: LayerNormW<B>,
    norm_in_t: LayerNormW<B>,
    layers: Vec<AttnLayer<B>>,
    layers_t: Vec<AttnLayer<B>>,
}

impl<B: Backend> CrossTransformer<B> {
    pub fn from_store(store: &TensorStore<'_>, device: &B::Device) -> Result<Self> {
        let load_stack = |branch: &str| {
            (0..T_LAYERS)
                .map(|i| {
                    AttnLayer::load(
                        store,
                        &format!("crosstransformer.{branch}.{i}"),
                        is_cross(i),
                        device,
                    )
                })
                .collect::<Result<Vec<_>>>()
        };
        Ok(Self {
            norm_in: LayerNormW::load(store, "crosstransformer.norm_in", device)?,
            norm_in_t: LayerNormW::load(store, "crosstransformer.norm_in_t", device)?,
            layers: load_stack("layers")?,
            layers_t: load_stack("layers_t")?,
        })
    }

    /// `x`: `[B, C, Fr, T]` frequency branch, `xt`: `[B, C, T2]` time branch.
    pub fn forward(&self, x: Tensor<B, 4>, xt: Tensor<B, 3>) -> (Tensor<B, 4>, Tensor<B, 3>) {
        let [batch, channels, bins, frames] = x.dims();
        let device = x.device();

        // Token order is `(t * bins + f)`, matching the reference rearrange.
        let mut freq = x
            .permute([0, 3, 2, 1])
            .reshape([batch, frames * bins, channels]);
        freq = self.norm_in.forward(freq) + pos_embedding_2d::<B>(channels, bins, frames, &device);

        let time_len = xt.dims()[2];
        let mut time = self.norm_in_t.forward(xt.swap_dims(1, 2))
            + pos_embedding_1d::<B>(channels, time_len, &device);

        for i in 0..T_LAYERS {
            if is_cross(i) {
                let previous = freq.clone();
                freq = self.layers[i].forward(freq, Some(time.clone()));
                time = self.layers_t[i].forward(time, Some(previous));
            } else {
                freq = self.layers[i].forward(freq, None);
                time = self.layers_t[i].forward(time, None);
            }
        }

        let freq = freq
            .reshape([batch, frames, bins, channels])
            .permute([0, 3, 2, 1]);
        (freq, time.swap_dims(1, 2))
    }
}

/// `t_cross_first = false`: even indices self-attend, odd indices cross-attend.
fn is_cross(index: usize) -> bool {
    index % 2 == 1
}

/// `create_2d_sin_embedding` flattened to `[1, frames * bins, channels]`.
fn pos_embedding_2d<B: Backend>(
    channels: usize,
    bins: usize,
    frames: usize,
    device: &B::Device,
) -> Tensor<B, 3> {
    let half = channels / 2;
    let terms: Vec<f32> = (0..half)
        .step_by(2)
        .map(|k| (-(T_MAX_PERIOD.ln()) * k as f32 / half as f32).exp())
        .collect();

    let mut values = vec![0.0f32; frames * bins * channels];
    for frame in 0..frames {
        for bin in 0..bins {
            let base = (frame * bins + bin) * channels;
            for (k, term) in terms.iter().enumerate() {
                let (time_phase, bin_phase) = (frame as f32 * term, bin as f32 * term);
                values[base + 2 * k] = time_phase.sin();
                values[base + 2 * k + 1] = time_phase.cos();
                values[base + half + 2 * k] = bin_phase.sin();
                values[base + half + 2 * k + 1] = bin_phase.cos();
            }
        }
    }
    Tensor::from_data(
        TensorData::new(values, [1, frames * bins, channels]),
        device,
    )
}

/// `create_sin_embedding` (cos then sin halves) as `[1, length, channels]`.
fn pos_embedding_1d<B: Backend>(
    channels: usize,
    length: usize,
    device: &B::Device,
) -> Tensor<B, 3> {
    let half = channels / 2;
    let mut values = vec![0.0f32; length * channels];
    for pos in 0..length {
        let base = pos * channels;
        for k in 0..half {
            let phase = pos as f32 / T_MAX_PERIOD.powf(k as f32 / (half - 1) as f32);
            values[base + k] = phase.cos();
            values[base + half + k] = phase.sin();
        }
    }
    Tensor::from_data(TensorData::new(values, [1, length, channels]), device)
}

const _: () = assert!(
    BOTTOM_CHANNELS.is_multiple_of(2 * T_HEADS),
    "transformer width must split evenly across heads and the 2D embedding halves"
);
