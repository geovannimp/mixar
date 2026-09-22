//! safetensors → Burn parameter loading for HTDemucs.

mod load;
mod tensor_store;

pub use load::load_htdemucs_from_safetensors;
pub(crate) use load::{
    Conv1dW, Conv2dW, ConvTr1dW, ConvTr2dW, GroupNorm1W, LayerNormW, LayerScaleW, LinearW,
};
pub use tensor_store::{TensorStore, HTDEMUCS_SIGNATURE};
