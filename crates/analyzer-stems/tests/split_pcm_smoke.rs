//! Optional end-to-end smoke: downloads ~166MB HTDemucs ONNX and runs a short split.
//!
//! ```text
//! cargo test -p analyzer-stems --manifest-path crates/Cargo.toml --test split_pcm_smoke -- --ignored --nocapture
//! ```

use analyzer_stems::{
    split_interleaved_stereo, StemAudioFormat, StemSplitRequest, DEFAULT_MODEL, STEM_NAMES,
};
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
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).expect("out dir");

    let result = split_interleaved_stereo(StemSplitRequest {
        interleaved_stereo: &pcm,
        sample_rate,
        output_dir: &out,
        models_root: &models,
        model_name: DEFAULT_MODEL,
        format: StemAudioFormat::Opus,
        on_window_progress: None,
    })
    .expect("split");

    assert_eq!(result.sample_rate, 48_000); // Opus resample target
    assert!(
        result.backend.starts_with(&format!("{DEFAULT_MODEL}/")),
        "backend {}",
        result.backend
    );
    for (i, name) in STEM_NAMES.iter().enumerate() {
        let path = &result.paths[i];
        assert!(
            path.ends_with(format!("{name}.opus")),
            "path {} should be {name}.opus",
            path.display()
        );
        let meta = std::fs::metadata(path).unwrap_or_else(|e| {
            panic!("missing {}: {e}", path.display());
        });
        assert!(meta.len() > 0, "{} empty", path.display());
    }
}
