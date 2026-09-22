//! Weight-backed PyTorch-equivalent layers built straight from a [`TensorStore`].
//!
//! HTDemucs is loaded once and only ever run forward, so these hold plain tensors
//! instead of Burn `Module`s: no random init of 84 MB of parameters we immediately
//! overwrite, and no record/field-name adapter between PyTorch and Burn naming.

use anyhow::{Context, Result};
use burn::prelude::{Backend, Tensor};
use burn::tensor::module::{conv1d, conv2d, conv_transpose1d, conv_transpose2d, linear};
use burn::tensor::ops::{ConvOptions, ConvTransposeOptions};

use super::TensorStore;
use crate::model::HTDemucs;

/// Build HTDemucs from a published safetensors checkpoint.
pub fn load_htdemucs_from_safetensors<B: Backend>(
    bytes: &[u8],
    device: &B::Device,
) -> Result<HTDemucs<B>> {
    let store = TensorStore::new(bytes)?;
    HTDemucs::from_store(&store, device)
}

/// `nn.Conv1d`: weight `[out, in, k]`.
pub struct Conv1dW<B: Backend> {
    weight: Tensor<B, 3>,
    bias: Tensor<B, 1>,
    stride: usize,
    padding: usize,
    dilation: usize,
}

impl<B: Backend> Conv1dW<B> {
    pub fn load(
        store: &TensorStore<'_>,
        path: &str,
        stride: usize,
        padding: usize,
        dilation: usize,
        device: &B::Device,
    ) -> Result<Self> {
        Ok(Self {
            weight: store.take(&format!("{path}.weight"), device)?,
            bias: store.take(&format!("{path}.bias"), device)?,
            stride,
            padding,
            dilation,
        })
    }

    pub fn out_channels(&self) -> usize {
        self.weight.dims()[0]
    }

    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        conv1d(
            x,
            self.weight.clone(),
            Some(self.bias.clone()),
            ConvOptions::new([self.stride], [self.padding], [self.dilation], 1),
        )
    }
}

/// `nn.Conv2d`: weight `[out, in, kh, kw]`.
pub struct Conv2dW<B: Backend> {
    weight: Tensor<B, 4>,
    bias: Tensor<B, 1>,
    stride: [usize; 2],
    padding: [usize; 2],
}

impl<B: Backend> Conv2dW<B> {
    pub fn load(
        store: &TensorStore<'_>,
        path: &str,
        stride: [usize; 2],
        padding: [usize; 2],
        device: &B::Device,
    ) -> Result<Self> {
        Ok(Self {
            weight: store.take(&format!("{path}.weight"), device)?,
            bias: store.take(&format!("{path}.bias"), device)?,
            stride,
            padding,
        })
    }

    pub fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        conv2d(
            x,
            self.weight.clone(),
            Some(self.bias.clone()),
            ConvOptions::new(self.stride, self.padding, [1, 1], 1),
        )
    }
}

/// `nn.ConvTranspose1d`: weight `[in, out, k]`, no padding (callers trim instead).
pub struct ConvTr1dW<B: Backend> {
    weight: Tensor<B, 3>,
    bias: Tensor<B, 1>,
    stride: usize,
}

impl<B: Backend> ConvTr1dW<B> {
    pub fn load(
        store: &TensorStore<'_>,
        path: &str,
        stride: usize,
        device: &B::Device,
    ) -> Result<Self> {
        Ok(Self {
            weight: store.take(&format!("{path}.weight"), device)?,
            bias: store.take(&format!("{path}.bias"), device)?,
            stride,
        })
    }

    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        conv_transpose1d(
            x,
            self.weight.clone(),
            Some(self.bias.clone()),
            ConvTransposeOptions::new([self.stride], [0], [0], [1], 1),
        )
    }
}

/// `nn.ConvTranspose2d`: weight `[in, out, kh, kw]`, no padding (callers trim instead).
pub struct ConvTr2dW<B: Backend> {
    weight: Tensor<B, 4>,
    bias: Tensor<B, 1>,
    stride: [usize; 2],
}

impl<B: Backend> ConvTr2dW<B> {
    pub fn load(
        store: &TensorStore<'_>,
        path: &str,
        stride: [usize; 2],
        device: &B::Device,
    ) -> Result<Self> {
        Ok(Self {
            weight: store.take(&format!("{path}.weight"), device)?,
            bias: store.take(&format!("{path}.bias"), device)?,
            stride,
        })
    }

    pub fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        conv_transpose2d(
            x,
            self.weight.clone(),
            Some(self.bias.clone()),
            ConvTransposeOptions::new(self.stride, [0, 0], [0, 0], [1, 1], 1),
        )
    }
}

/// `nn.Linear`. PyTorch stores `[out, in]`; Burn's `linear` wants `[in, out]`.
pub struct LinearW<B: Backend> {
    weight: Tensor<B, 2>,
    bias: Tensor<B, 1>,
}

impl<B: Backend> LinearW<B> {
    pub fn load(store: &TensorStore<'_>, path: &str, device: &B::Device) -> Result<Self> {
        let weight: Tensor<B, 2> = store.take(&format!("{path}.weight"), device)?;
        Ok(Self {
            weight: weight.transpose(),
            bias: store.take(&format!("{path}.bias"), device)?,
        })
    }

    /// Build from an already-sliced PyTorch-layout `[out, in]` weight and `[out]` bias.
    pub fn from_parts(weight: Tensor<B, 2>, bias: Tensor<B, 1>) -> Self {
        Self {
            weight: weight.transpose(),
            bias,
        }
    }

    pub fn forward<const D: usize>(&self, x: Tensor<B, D>) -> Tensor<B, D> {
        linear(x, self.weight.clone(), Some(self.bias.clone()))
    }
}

/// `nn.LayerNorm` over the last dimension.
pub struct LayerNormW<B: Backend> {
    weight: Tensor<B, 1>,
    bias: Tensor<B, 1>,
}

impl<B: Backend> LayerNormW<B> {
    pub fn load(store: &TensorStore<'_>, path: &str, device: &B::Device) -> Result<Self> {
        Ok(Self {
            weight: store.take(&format!("{path}.weight"), device)?,
            bias: store.take(&format!("{path}.bias"), device)?,
        })
    }

    pub fn forward<const D: usize>(&self, x: Tensor<B, D>) -> Tensor<B, D> {
        let dim = D - 1;
        let mean = x.clone().mean_dim(dim);
        let centered = x - mean;
        // LayerNorm uses the biased variance, so mean of squares (not `var`).
        let var = (centered.clone() * centered.clone()).mean_dim(dim);
        let normed = centered / (var + NORM_EPS).sqrt();
        normed * self.weight.clone().unsqueeze() + self.bias.clone().unsqueeze()
    }
}

/// `nn.GroupNorm(1, channels)` over `[B, C, L]`: one mean/var per batch item.
pub struct GroupNorm1W<B: Backend> {
    weight: Tensor<B, 1>,
    bias: Tensor<B, 1>,
}

impl<B: Backend> GroupNorm1W<B> {
    pub fn load(store: &TensorStore<'_>, path: &str, device: &B::Device) -> Result<Self> {
        Ok(Self {
            weight: store.take(&format!("{path}.weight"), device)?,
            bias: store.take(&format!("{path}.bias"), device)?,
        })
    }

    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let [b, c, l] = x.dims();
        let flat = x.clone().reshape([b, c * l]);
        let mean = flat.clone().mean_dim(1);
        let centered = flat - mean.clone();
        let var = (centered.clone() * centered).mean_dim(1);
        let normed = (x - mean.reshape([b, 1, 1])) / (var.reshape([b, 1, 1]) + NORM_EPS).sqrt();
        normed * self.weight.clone().reshape([1, c, 1]) + self.bias.clone().reshape([1, c, 1])
    }

    /// `MyGroupNorm`: same normalisation on `[B, T, C]` tensors (channels last).
    pub fn forward_channels_last(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        self.forward(x.swap_dims(1, 2)).swap_dims(1, 2)
    }
}

/// `LayerScale`: a learned per-channel gain on a residual branch.
pub struct LayerScaleW<B: Backend> {
    scale: Tensor<B, 1>,
}

impl<B: Backend> LayerScaleW<B> {
    pub fn load(store: &TensorStore<'_>, path: &str, device: &B::Device) -> Result<Self> {
        Ok(Self {
            scale: store
                .take(&format!("{path}.scale"), device)
                .with_context(|| format!("layer scale {path}"))?,
        })
    }

    /// `[B, C, L]` inputs (`channel_last = False`).
    pub fn forward_channels_first(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let c = self.scale.dims()[0];
        x * self.scale.clone().reshape([1, c, 1])
    }

    /// `[B, T, C]` inputs (`channel_last = True`).
    pub fn forward_channels_last(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        x * self.scale.clone().unsqueeze()
    }
}

/// PyTorch default eps for `LayerNorm` / `GroupNorm`.
const NORM_EPS: f32 = 1e-5;
