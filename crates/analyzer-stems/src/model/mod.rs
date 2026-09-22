//! Burn implementation of HTDemucs v4 (standard 4-stem).
//!
//! Shapes and layer wiring are fixed by the published checkpoint, so the
//! constants below are load-bearing: changing one without new weights breaks
//! the forward graph.

mod conv;
mod htdemucs;
mod transformer;

pub use htdemucs::HTDemucs;

pub(crate) use conv::{FreqDecLayer, FreqEncLayer, TimeDecLayer, TimeEncLayer};
pub(crate) use transformer::CrossTransformer;

use burn::prelude::{Backend, Tensor};
use burn::tensor::activation::sigmoid;

/// Base hidden channel count of the first encoder layer.
pub const CHANNELS: usize = 48;
/// Channel multiplier per encoder level.
pub const GROWTH: usize = 2;
/// Encoder / decoder levels per branch.
pub const DEPTH: usize = 4;
/// Strided conv kernel along frequency (freq branch) / time (time branch).
pub const KERNEL_SIZE: usize = 8;
/// Strided conv stride, matching `KERNEL_SIZE / 4` padding.
pub const STRIDE: usize = 4;
/// Cross-domain transformer layers per branch.
pub const T_LAYERS: usize = 5;
/// Attention heads in the cross-domain transformer.
pub const T_HEADS: usize = 8;
/// Model sample rate.
pub const SAMPLE_RATE: u32 = 44_100;
/// Segment length the released weights were trained on (~7.8 s).
pub const TRAINING_LENGTH: usize = 343_980;

/// Separated stems (drums, bass, other, vocals).
pub const SOURCES: usize = 4;
/// Audio channels in and out.
pub const AUDIO_CHANNELS: usize = 2;
/// Transformer width after `channel_upsampler` (`bottom_channels`).
pub const BOTTOM_CHANNELS: usize = 512;
/// Transformer feed-forward width (`t_hidden_scale = 4.0`).
pub const T_HIDDEN: usize = BOTTOM_CHANNELS * 4;
/// `max_period` for both sinusoidal position embeddings.
pub const T_MAX_PERIOD: f32 = 10_000.0;
/// `freq_emb`: weight applied to the frequency embedding added after encoder 0.
pub const FREQ_EMB_SCALE: f32 = 0.2;
/// `emb_scale`: `ScaledEmbedding` bakes a 1/10 division into the stored weight.
pub const FREQ_EMB_WEIGHT_SCALE: f32 = 10.0;
/// `DConv` residual sub-layers, dilations `1, 2` (`dconv_depth`).
pub const DCONV_DEPTH: usize = 2;
/// Trim applied to the frequency axis after each transposed conv (`kernel_size / 4`).
pub const CONV_PAD: usize = KERNEL_SIZE / 4;

/// Channels entering the transformer, before `channel_upsampler`.
pub const fn transformer_channels() -> usize {
    CHANNELS * GROWTH.pow(DEPTH as u32 - 1)
}

/// `F.glu(x, dim)`: split in half along `dim` and gate.
pub(crate) fn glu<B: Backend, const D: usize>(x: Tensor<B, D>, dim: usize) -> Tensor<B, D> {
    let half = x.dims()[dim] / 2;
    let gate = x.clone().narrow(dim, half, half);
    x.narrow(dim, 0, half) * sigmoid(gate)
}

#[cfg(all(test, feature = "burn-ndarray"))]
mod tests {
    use super::*;
    use crate::NdArrayBackend as TestBackend;
    use burn::prelude::TensorData;

    /// `permute(...).reshape(...)` must read the permuted layout, not the original
    /// buffer: the transformer token order and the `DConv` fold both rely on it,
    /// and getting it wrong would still produce plausible finite numbers.
    #[test]
    fn reshape_follows_permuted_layout() {
        let device = Default::default();
        // [1, 2, 3, 1] holding 1..=6 along (channel, bin).
        let x = Tensor::<TestBackend, 4>::from_data(
            TensorData::new(vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0], [1, 2, 3, 1]),
            &device,
        );
        // Swap channel and bin, as `frequency_embedding` and tokenisation do.
        let swapped = x.permute([0, 2, 1, 3]).reshape([6]);
        assert_eq!(
            swapped.into_data().to_vec::<f32>().expect("data"),
            vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]
        );

        let table = Tensor::<TestBackend, 2>::from_data(
            TensorData::new(vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0], [3, 2]),
            &device,
        );
        let planes = table.transpose().reshape([1, 2, 3, 1]);
        assert_eq!(
            planes.into_data().to_vec::<f32>().expect("data"),
            vec![1.0, 3.0, 5.0, 2.0, 4.0, 6.0]
        );
    }

    #[test]
    fn glu_gates_the_second_half() {
        let device = Default::default();
        let x = Tensor::<TestBackend, 3>::from_data(
            TensorData::new(vec![1.0f32, 2.0, 0.0, 0.0], [1, 4, 1]),
            &device,
        );
        let out = glu(x, 1).into_data().to_vec::<f32>().expect("data");
        // Gates are zero, so sigmoid(0) = 0.5 halves the first half.
        assert_eq!(out, vec![0.5, 1.0]);
    }
}
