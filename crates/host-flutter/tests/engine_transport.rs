//! EngineTransport: start/stop, play cmd, load path (null backend).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use engine_api::{decode_evt_body, EvtBody, Kind, Origin};
use host_flutter::api::engine::{EngineStartConfig, EngineTransport};
use host_flutter::api::library::LibraryTransport;

fn short_tone_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-tone.wav")
}

fn null_start_config() -> EngineStartConfig {
    EngineStartConfig {
        backend: "null".into(),
        sample_rate: None,
        buffer_size: None,
    }
}

fn recv_kind(rx: &engine_core::EvtReceiver, kind: Kind, timeout: Duration) -> engine_core::Evt {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = match rx.recv_timeout(remaining.min(Duration::from_millis(50))) {
            Ok(Some(event)) => event,
            Ok(None) => continue,
            Err(e) => panic!("evt bus disconnected waiting for {kind:?}: {e}"),
        };
        if *event.kind() == kind {
            return (*event).clone();
        }
    }
    panic!("timeout waiting for evt kind {kind:?}");
}

#[test]
fn start_stop_null_backend() {
    let library = LibraryTransport::open_in_memory().unwrap();
    let transport = EngineTransport::start(&library, null_start_config()).unwrap();
    assert!(transport.is_running());
    transport.stop().unwrap();
    assert!(!transport.is_running());
}

#[test]
fn play_on_empty_deck_publishes_error() {
    let library = LibraryTransport::open_in_memory().unwrap();
    let transport = EngineTransport::start(&library, null_start_config()).unwrap();
    let rx = transport.subscribe_evt_all().unwrap();
    transport.play(0).unwrap();
    let event = recv_kind(&rx, Kind::Error, Duration::from_secs(2));
    assert_eq!(*event.origin(), Origin::Deck(0));
}

#[test]
fn load_path_publishes_updated() {
    let library = LibraryTransport::open_in_memory().unwrap();
    let transport = EngineTransport::start(&library, null_start_config()).unwrap();
    let rx = transport.subscribe_evt_all().unwrap();
    transport
        .load_path(0, short_tone_fixture().to_string_lossy().into_owned())
        .unwrap();
    let event = recv_kind(&rx, Kind::Updated, Duration::from_secs(5));
    assert_eq!(*event.origin(), Origin::Deck(0));
}

#[test]
fn set_volume_publishes_updated() {
    let library = LibraryTransport::open_in_memory().unwrap();
    let transport = EngineTransport::start(&library, null_start_config()).unwrap();
    let rx = transport.subscribe_evt_all().unwrap();
    transport.set_volume(0, 0.25).unwrap();
    let event = recv_kind(&rx, Kind::Updated, Duration::from_secs(2));
    assert_eq!(*event.origin(), Origin::Deck(0));
    let EvtBody::DeckUpdated { volume, .. } = decode_evt_body(event.payload()).unwrap() else {
        panic!("expected DeckUpdated");
    };
    assert!((volume - 0.25).abs() < 1e-4);
}

#[test]
fn set_crossfader_publishes_status() {
    let library = LibraryTransport::open_in_memory().unwrap();
    let transport = EngineTransport::start(&library, null_start_config()).unwrap();
    let rx = transport.subscribe_evt_all().unwrap();
    transport.set_crossfader(0.2).unwrap();
    let event = recv_kind(&rx, Kind::Status, Duration::from_secs(2));
    assert_eq!(*event.origin(), Origin::Mixer);
    let EvtBody::EngineStatus { status } = decode_evt_body(event.payload()).unwrap() else {
        panic!("expected EngineStatus");
    };
    assert!((status.crossfader - 0.2).abs() < 1e-4);
}

#[test]
fn load_path_exposes_waveform_peaks() {
    let library = LibraryTransport::open_in_memory().unwrap();
    let transport = EngineTransport::start(&library, null_start_config()).unwrap();
    let path = short_tone_fixture().to_string_lossy().into_owned();
    transport.load_path(0, path.clone()).unwrap();

    let resolved = library.resolve_tracks_for_paths(vec![path]).unwrap();
    let track_id = resolved[0].track.id.clone();
    let overview = library
        .get_waveform_overview(track_id.clone())
        .unwrap()
        .expect("overview after load");
    assert!(overview.count > 0);
    assert_eq!(overview.rgb.len(), overview.count as usize * 3);

    let window = library.get_waveform_window(track_id, 0, 200, 32).unwrap();
    assert_eq!(window.count, 32);
    assert_eq!(window.rgb.len(), 96);
}

#[test]
fn seek_after_load_publishes_updated() {
    let library = LibraryTransport::open_in_memory().unwrap();
    let transport = EngineTransport::start(&library, null_start_config()).unwrap();
    let rx = transport.subscribe_evt_all().unwrap();
    transport
        .load_path(0, short_tone_fixture().to_string_lossy().into_owned())
        .unwrap();
    recv_kind(&rx, Kind::Updated, Duration::from_secs(5));
    transport.seek(0, 20).unwrap();
    recv_kind(&rx, Kind::Updated, Duration::from_secs(2));
}
