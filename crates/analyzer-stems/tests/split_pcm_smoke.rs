//! Optional end-to-end smoke: downloads ~166MB HTDemucs ONNX and runs a short split.
//!
//! ```text
//! cargo test -p analyzer-stems --manifest-path crates/Cargo.toml --test split_pcm_smoke -- --ignored --nocapture
//! ```

use analyzer_stems::{split_interleaved_stereo, StemSplitRequest, DEFAULT_MODEL, STEM_NAMES};
use std::f32::consts::TAU;

#[test]
#[ignore = "downloads HTDemucs ONNX (~166MB) and runs ort inference"]
fn split_short_synthetic_pcm() {
    let sample_rate = 44_100u32;
    let frames = sample_rate as usize; // 1 s
    let mut pcm = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let s = (t * 220.0 * TAU).sin() * 0.2;
        pcm.push(s);
        pcm.push(s * 0.8);
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let models = dir.path().join("models");
    std::fs::create_dir_all(&models).expect("models dir");

    let result = split_interleaved_stereo(StemSplitRequest {
        interleaved_stereo: &pcm,
        sample_rate,
        models_root: &models,
        model_name: DEFAULT_MODEL,
        on_window_progress: None,
    })
    .expect("split");

    assert_eq!(result.sample_rate, 44_100);
    assert!(
        result.backend.starts_with(&format!("{DEFAULT_MODEL}/")),
        "backend {}",
        result.backend
    );
    for (i, name) in STEM_NAMES.iter().enumerate() {
        let stem = &result.stems[i];
        assert!(!stem.is_empty(), "{name} stem should have PCM samples");
        assert!(
            stem.len().is_multiple_of(2),
            "{name} should be interleaved stereo"
        );
    }
}
