//! Read-only safetensors view that yields Burn tensors.

use anyhow::{anyhow, bail, Result};
use burn::prelude::{Backend, Tensor, TensorData};
use half::{bf16, f16};
use safetensors::{Dtype, SafeTensors};

/// Published HTDemucs 4-stem checkpoints prefix every key with this signature.
pub const HTDEMUCS_SIGNATURE: &str = "955717e8";

/// Borrowed safetensors archive with the shared key prefix stripped.
pub struct TensorStore<'a> {
    tensors: SafeTensors<'a>,
    prefix: String,
}

impl<'a> TensorStore<'a> {
    /// Parse a safetensors blob. Fails on a malformed header or out-of-range offsets.
    pub fn new(bytes: &'a [u8]) -> Result<Self> {
        let tensors =
            SafeTensors::deserialize(bytes).map_err(|e| anyhow!("parse safetensors: {e}"))?;
        let prefix = detect_prefix(&tensors);
        Ok(Self { tensors, prefix })
    }

    /// Shared top-level key prefix (`955717e8` for published HTDemucs), or `""` when keys differ.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Number of tensors in the archive.
    pub fn len(&self) -> usize {
        self.tensors.len()
    }

    /// True when the archive holds no tensors.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fetch `name` (prefix applied) as an f32 tensor of rank `D`.
    pub fn take<B: Backend, const D: usize>(
        &self,
        name: &str,
        device: &B::Device,
    ) -> Result<Tensor<B, D>> {
        let (values, shape) = self.take_f32(name)?;
        if shape.len() != D {
            bail!(
                "tensor '{name}' has shape {shape:?} (rank {}), expected rank {D}",
                shape.len()
            );
        }
        Ok(Tensor::from_data(TensorData::new(values, shape), device))
    }

    /// Fetch `name` (prefix applied) as f32 values plus its shape.
    pub fn take_f32(&self, name: &str) -> Result<(Vec<f32>, Vec<usize>)> {
        let key = self.key(name);
        let view = self
            .tensors
            .tensor(&key)
            .map_err(|e| anyhow!("missing tensor '{key}': {e}"))?;
        let shape = view.shape().to_vec();
        let raw = view.data();
        let values = match view.dtype() {
            Dtype::F32 => decode(raw, 4, |b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            Dtype::F16 => decode(raw, 2, |b| f16::from_le_bytes([b[0], b[1]]).to_f32()),
            Dtype::BF16 => decode(raw, 2, |b| bf16::from_le_bytes([b[0], b[1]]).to_f32()),
            other => bail!("tensor '{key}' has unsupported dtype {other:?}"),
        };
        let expected: usize = shape.iter().product();
        if values.len() != expected {
            bail!(
                "tensor '{key}' holds {} values, shape {shape:?} needs {expected}",
                values.len()
            );
        }
        Ok((values, shape))
    }

    fn key(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}.{name}", self.prefix)
        }
    }
}

fn decode(raw: &[u8], width: usize, to_f32: impl Fn(&[u8]) -> f32) -> Vec<f32> {
    raw.chunks_exact(width).map(to_f32).collect()
}

/// Longest common first key segment, or `""` when the keys do not share one.
fn detect_prefix(tensors: &SafeTensors<'_>) -> String {
    let names = tensors.names();
    let mut prefix: Option<&str> = None;
    for name in &names {
        let Some((head, _)) = name.split_once('.') else {
            return String::new();
        };
        match prefix {
            None => prefix = Some(head),
            Some(seen) if seen == head => {}
            Some(_) => return String::new(),
        }
    }
    prefix.unwrap_or_default().to_string()
}

#[cfg(all(test, feature = "burn-ndarray"))]
mod tests {
    use super::*;
    use crate::NdArrayBackend as B;
    use safetensors::tensor::TensorView;

    fn blob(entries: Vec<(&str, Dtype, Vec<usize>, &[u8])>) -> Vec<u8> {
        let views: Vec<_> = entries
            .into_iter()
            .map(|(name, dtype, shape, data)| {
                (
                    name.to_string(),
                    TensorView::new(dtype, shape, data).expect("view"),
                )
            })
            .collect();
        safetensors::serialize(views, &None).expect("serialize")
    }

    #[test]
    fn take_round_trips_f32_shape_and_values() {
        let values: Vec<f32> = vec![1.0, 2.0, 3.0, -4.0, 5.5, 6.25];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let raw = blob(vec![("w", Dtype::F32, vec![2, 3], &bytes)]);

        let store = TensorStore::new(&raw).expect("store");
        assert_eq!(store.prefix(), "");
        assert_eq!(store.len(), 1);

        let t = store.take::<B, 2>("w", &Default::default()).expect("take");
        assert_eq!(t.dims(), [2, 3]);
        assert_eq!(t.into_data().to_vec::<f32>().expect("data"), values);

        // Rank mismatch must be an error, not a panic deep in Burn.
        assert!(store.take::<B, 3>("w", &Default::default()).is_err());
        assert!(store.take::<B, 2>("missing", &Default::default()).is_err());
    }

    #[test]
    fn take_widens_f16_and_strips_shared_prefix() {
        let halves: Vec<f16> = vec![f16::from_f32(0.5), f16::from_f32(-1.5)];
        let bytes: Vec<u8> = halves.iter().flat_map(|v| v.to_le_bytes()).collect();
        let raw = blob(vec![
            ("955717e8.a.weight", Dtype::F16, vec![2], &bytes),
            ("955717e8.b.weight", Dtype::F16, vec![2], &bytes),
        ]);

        let store = TensorStore::new(&raw).expect("store");
        assert_eq!(store.prefix(), HTDEMUCS_SIGNATURE);

        let t = store
            .take::<B, 1>("a.weight", &Default::default())
            .expect("take");
        assert_eq!(
            t.into_data().to_vec::<f32>().expect("data"),
            vec![0.5, -1.5]
        );
    }
}
