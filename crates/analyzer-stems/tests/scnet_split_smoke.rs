//! Optional end-to-end smoke: downloads the 12.9 MB SCNet int8 ONNX and splits
//! a short synthetic clip.
//!
//! ```text
//! cargo nextest run -p analyzer-stems --manifest-path crates/Cargo.toml \
//!   --test scnet_split_smoke --run-ignored=only --no-capture
//! ```

use std::f32::consts::TAU;
use std::path::PathBuf;

use analyzer_stems::{resolve_model, separate_with_model, SCNET_MODEL, STEM_NAMES};

#[test]
#[ignore = "downloads SCNet int8 ONNX (~12.9MB) and runs ort inference"]
fn split_short_synthetic_pcm_with_scnet() {
    let sample_rate = 44_100u32;
    let frames = sample_rate as usize * 3;
    let mut pcm = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let beat = ((t * 2.0) % 1.0 < 0.08) as u8 as f32;
        let s = (t * 110.0 * TAU).sin() * 0.2 + (t * 330.0 * TAU).sin() * 0.05 + beat * 0.1;
        pcm.push(s);
        pcm.push(s * 0.8);
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let models = std::env::var_os("MIXAR_STEMS_MODEL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.path().join("models"));
    std::fs::create_dir_all(&models).expect("models dir");
    let handle = resolve_model(&models, SCNET_MODEL, None).expect("model");

    let (stems, ep) = separate_with_model(SCNET_MODEL, &handle.local_path, &pcm, sample_rate, None)
        .expect("split");

    assert!(ep.starts_with("webgpu") || ep.starts_with("cpu"), "ep {ep}");
    for (i, name) in STEM_NAMES.iter().enumerate() {
        let stem = &stems[i];
        assert_eq!(stem.len(), pcm.len(), "{name} length");
        assert!(stem.iter().all(|s| s.is_finite()), "{name} finite");
        assert!(
            stem.iter().any(|s| s.abs() > 1e-4),
            "{name} should contain audio"
        );
    }
}
