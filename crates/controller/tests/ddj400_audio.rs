//! Shipped DDJ-400 bundle: audio-output routing hints reach the host.
#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures

use std::path::Path;

#[test]
fn ddj400_declares_master_and_headphone_routes() {
    let bundle = controller::load_bundle(Path::new("../../mappings/ddj-400")).unwrap();
    let audio = &bundle.device.audio;
    assert_eq!(audio.output_name_contains, ["DDJ-400"]);
    assert_eq!(audio.master_channels, Some([1, 2]));
    assert_eq!(audio.headphone_channels, Some([3, 4]));
    assert!(audio.has_output_routes());
}

#[test]
fn list_mappings_exposes_audio_hints() {
    let root = tempfile::tempdir().unwrap();
    let shipped = Path::new("../../mappings");
    let engine = controller::ControllerEngine::open("test", root.path(), shipped).unwrap();
    let rows = engine.list_mappings().unwrap();
    let ddj = rows
        .iter()
        .find(|row| row.id == "ddj-400")
        .expect("shipped DDJ-400 mapping");
    let audio = ddj.audio.as_ref().expect("audio hints");
    assert_eq!(audio.master_channels, Some([1, 2]));
    assert_eq!(audio.headphone_channels, Some([3, 4]));
}
