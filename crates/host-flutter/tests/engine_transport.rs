//! EngineTransport: start/stop, play cmd, load path (null backend).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use engine_api::{Kind, Origin};
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
        let event = rx
            .recv_timeout(remaining.min(Duration::from_millis(50)))
            .expect("recv")
            .expect("event");
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
