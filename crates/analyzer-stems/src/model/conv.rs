//! Hybrid encoder / decoder layers shared by the frequency and time branches.
//!
//! Mirrors `HEncLayer` / `HDecLayer` / `DConv` from public HTDemucs v4. For the
//! released weights (`nfft = 4096`, `depth = 4`, `norm_starts = 4`) every layer
//! is un-normalised and non-empty, so the GroupNorms and the time→freq `inject`
//! path of the reference implementation carry no parameters and are omitted.

use anyhow::Result;
use burn::prelude::{Backend, Tensor};
use burn::tensor::activation::gelu;

use super::{glu, CONV_PAD, DCONV_DEPTH, STRIDE};
use crate::weights::{
    Conv1dW, Conv2dW, ConvTr1dW, ConvTr2dW, GroupNorm1W, LayerScaleW, TensorStore,
};

/// One `DConv` residual sub-layer: dilated conv → GroupNorm → GELU → 1x1 → GroupNorm → GLU → scale.
struct DConvLayer<B: Backend> {
    conv_in: Conv1dW<B>,
    norm_in: GroupNorm1W<B>,
    conv_out: Conv1dW<B>,
    norm_out: GroupNorm1W<B>,
    scale: LayerScaleW<B>,
}

impl<B: Backend> DConvLayer<B> {
    fn load(
        store: &TensorStore<'_>,
        path: &str,
        dilation: usize,
        device: &B::Device,
    ) -> Result<Self> {
        Ok(Self {
            // kernel 3 with `padding = dilation` keeps the length unchanged.
            conv_in: Conv1dW::load(store, &format!("{path}.0"), 1, dilation, dilation, device)?,
            norm_in: GroupNorm1W::load(store, &format!("{path}.1"), device)?,
            conv_out: Conv1dW::load(store, &format!("{path}.3"), 1, 0, 1, device)?,
            norm_out: GroupNorm1W::load(store, &format!("{path}.4"), device)?,
            scale: LayerScaleW::load(store, &format!("{path}.6"), device)?,
        })
    }

    fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let y = gelu(self.norm_in.forward(self.conv_in.forward(x)));
        let y = glu(self.norm_out.forward(self.conv_out.forward(y)), 1);
        self.scale.forward_channels_first(y)
    }
}

/// `DConv`: stacked dilated residual branches over `[B, C, T]`.
pub struct DConv<B: Backend> {
    layers: Vec<DConvLayer<B>>,
}

impl<B: Backend> DConv<B> {
    fn load(store: &TensorStore<'_>, path: &str, device: &B::Device) -> Result<Self> {
        let layers = (0..DCONV_DEPTH)
            .map(|d| DConvLayer::load(store, &format!("{path}.layers.{d}"), 1 << d, device))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { layers })
    }

    fn forward(&self, mut x: Tensor<B, 3>) -> Tensor<B, 3> {
        for layer in &self.layers {
            x = x.clone() + layer.forward(x);
        }
        x
    }

    /// Apply per frequency bin: `[B, C, Fr, T]` is folded into `[B * Fr, C, T]`.
    fn forward_freq(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let [b, c, fr, t] = x.dims();
        let folded = x.permute([0, 2, 1, 3]).reshape([b * fr, c, t]);
        self.forward(folded)
            .reshape([b, fr, c, t])
            .permute([0, 2, 1, 3])
    }
}

/// Frequency-branch encoder layer (`Conv2d` over the frequency axis).
pub struct FreqEncLayer<B: Backend> {
    conv: Conv2dW<B>,
    rewrite: Conv2dW<B>,
    dconv: DConv<B>,
}

impl<B: Backend> FreqEncLayer<B> {
    pub fn load(store: &TensorStore<'_>, path: &str, device: &B::Device) -> Result<Self> {
        Ok(Self {
            conv: Conv2dW::load(
                store,
                &format!("{path}.conv"),
                [STRIDE, 1],
                [CONV_PAD, 0],
                device,
            )?,
            // `context_enc = 0` makes the rewrite a 1x1 conv.
            rewrite: Conv2dW::load(store, &format!("{path}.rewrite"), [1, 1], [0, 0], device)?,
            dconv: DConv::load(store, &format!("{path}.dconv"), device)?,
        })
    }

    pub fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let y = gelu(self.conv.forward(x));
        let y = self.dconv.forward_freq(y);
        glu(self.rewrite.forward(y), 1)
    }
}

/// Time-branch encoder layer (`Conv1d` over samples).
pub struct TimeEncLayer<B: Backend> {
    conv: Conv1dW<B>,
    rewrite: Conv1dW<B>,
    dconv: DConv<B>,
}

impl<B: Backend> TimeEncLayer<B> {
    pub fn load(store: &TensorStore<'_>, path: &str, device: &B::Device) -> Result<Self> {
        Ok(Self {
            conv: Conv1dW::load(store, &format!("{path}.conv"), STRIDE, CONV_PAD, 1, device)?,
            rewrite: Conv1dW::load(store, &format!("{path}.rewrite"), 1, 0, 1, device)?,
            dconv: DConv::load(store, &format!("{path}.dconv"), device)?,
        })
    }

    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let x = pad_to_stride(x);
        let y = gelu(self.conv.forward(x));
        let y = self.dconv.forward(y);
        glu(self.rewrite.forward(y), 1)
    }
}

/// Right-pad with zeros so the strided conv output length is exactly `len / STRIDE`.
fn pad_to_stride<B: Backend>(x: Tensor<B, 3>) -> Tensor<B, 3> {
    let [b, c, len] = x.dims();
    let remainder = len % STRIDE;
    if remainder == 0 {
        return x;
    }
    let device = x.device();
    Tensor::cat(
        vec![x, Tensor::zeros([b, c, STRIDE - remainder], &device)],
        2,
    )
}

/// Frequency-branch decoder layer.
pub struct FreqDecLayer<B: Backend> {
    rewrite: Conv2dW<B>,
    dconv: DConv<B>,
    conv_tr: ConvTr2dW<B>,
    last: bool,
}

impl<B: Backend> FreqDecLayer<B> {
    pub fn load(
        store: &TensorStore<'_>,
        path: &str,
        last: bool,
        device: &B::Device,
    ) -> Result<Self> {
        Ok(Self {
            // `context = 1` makes the rewrite a 3x3 conv with symmetric padding.
            rewrite: Conv2dW::load(store, &format!("{path}.rewrite"), [1, 1], [1, 1], device)?,
            dconv: DConv::load(store, &format!("{path}.dconv"), device)?,
            conv_tr: ConvTr2dW::load(store, &format!("{path}.conv_tr"), [STRIDE, 1], device)?,
            last,
        })
    }

    pub fn forward(&self, x: Tensor<B, 4>, skip: Tensor<B, 4>) -> Tensor<B, 4> {
        let y = glu(self.rewrite.forward(x + skip), 1);
        let y = self.dconv.forward_freq(y);
        let z = self.conv_tr.forward(y);
        // Transposed conv over-produces by `2 * CONV_PAD` frequency bins.
        let fr = z.dims()[2];
        let z = z.narrow(2, CONV_PAD, fr - 2 * CONV_PAD);
        if self.last {
            z
        } else {
            gelu(z)
        }
    }
}

/// Time-branch decoder layer.
pub struct TimeDecLayer<B: Backend> {
    rewrite: Conv1dW<B>,
    dconv: DConv<B>,
    conv_tr: ConvTr1dW<B>,
    last: bool,
}

impl<B: Backend> TimeDecLayer<B> {
    pub fn load(
        store: &TensorStore<'_>,
        path: &str,
        last: bool,
        device: &B::Device,
    ) -> Result<Self> {
        Ok(Self {
            rewrite: Conv1dW::load(store, &format!("{path}.rewrite"), 1, 1, 1, device)?,
            dconv: DConv::load(store, &format!("{path}.dconv"), device)?,
            conv_tr: ConvTr1dW::load(store, &format!("{path}.conv_tr"), STRIDE, device)?,
            last,
        })
    }

    /// `length` is the pre-encode length of this level, restored by trimming the
    /// transposed conv output (which also undoes [`pad_to_stride`]).
    pub fn forward(&self, x: Tensor<B, 3>, skip: Tensor<B, 3>, length: usize) -> Tensor<B, 3> {
        let y = glu(self.rewrite.forward(x + skip), 1);
        let y = self.dconv.forward(y);
        let z = self.conv_tr.forward(y).narrow(2, CONV_PAD, length);
        if self.last {
            z
        } else {
            gelu(z)
        }
    }
}
