//! EngineTransport: start/stop, play cmd, load path (null backend).

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bus_api::DeckUpdated;
use bus_api::EngineSnapshot;
use bus_api::EngineStatus;
use bus_api::Event;
use bus_api::FxKind;
use event_bus::Subscriber;
use host_flutter::api::bus::BusTransport;
use host_flutter::api::engine::EngineTransport;
use host_flutter::api::library::LibraryTransport;
use host_flutter::testing;

fn short_tone_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-tone.wav")
}

/// 6 s of 120 BPM clicks. A steady tone has no detectable beat, so tests that
/// need a BPM (loop in/out, auto loop) load this instead.
fn short_beats_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-beats.wav")
}

fn null_engine_config() -> engine_core::EngineConfig {
    engine_core::EngineConfig {
        backend: "null".into(),
        ..engine_core::EngineConfig::default()
    }
}

type EvtRx = Subscriber<Event>;

/// Receive the next event `pick` accepts, in short slices against `timeout`.
///
/// `Ok(None)` from the bus is the slice timeout, not an event, so draining keeps
/// going; a disconnected bus is a bug in the test's bus setup, so it panics.
fn recv_matching<T>(
    rx: &mut EvtRx,
    what: &str,
    timeout: Duration,
    mut pick: impl FnMut(Event) -> Option<T>,
) -> T {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(remaining.min(Duration::from_millis(50))) {
            Ok(Some(event)) => {
                if let Some(picked) = pick(event) {
                    return picked;
                }
            }
            Ok(None) => continue,
            Err(e) => panic!("evt bus disconnected waiting for {what}: {e}"),
        }
    }
    panic!("timeout waiting for {what}");
}

/// Receive the next `DeckUpdated` for `deck_id`, skipping other events/decks.
fn recv_deck_updated(rx: &mut EvtRx, deck_id: u16, timeout: Duration) -> DeckUpdated {
    let what = format!("DeckUpdated on deck {deck_id}");
    recv_matching(rx, &what, timeout, |event| match event {
        Event::DeckUpdated(update) if update.deck == deck_id => Some(update),
        _ => None,
    })
}

/// Receive the next whole-engine snapshot, skipping other events.
fn recv_engine_status(rx: &mut EvtRx, timeout: Duration) -> EngineStatus {
    recv_matching(rx, "EngineSnapshot", timeout, |event| match event {
        Event::EngineSnapshot(EngineSnapshot { status }) => Some(status),
        _ => None,
    })
}

/// Receive the next engine error, skipping other events.
fn recv_error(rx: &mut EvtRx, timeout: Duration) -> bus_api::Error {
    recv_matching(rx, "Error", timeout, |event| match event {
        Event::Error(error) => Some(error),
        _ => None,
    })
}

fn load_fixture_path(transport: &EngineTransport, rx: &mut EvtRx, deck_id: u16, path: PathBuf) {
    transport
        .load_path(deck_id, path.to_string_lossy().into_owned())
        .unwrap();
    recv_deck_updated(rx, deck_id, Duration::from_secs(5));
}

fn load_fixture(transport: &EngineTransport, rx: &mut EvtRx, deck_id: u16) {
    load_fixture_path(transport, rx, deck_id, short_tone_fixture());
}

fn load_beat_fixture(transport: &EngineTransport, rx: &mut EvtRx, deck_id: u16) {
    load_fixture_path(transport, rx, deck_id, short_beats_fixture());
}

/// FX slots carried by the next `DeckUpdated` for `deck_id`.
fn expect_deck_fx(rx: &mut EvtRx, deck_id: u16, timeout: Duration) -> Vec<bus_api::FxSlotState> {
    recv_deck_updated(rx, deck_id, timeout).fx
}

/// `fx_routed` carried by the next `DeckUpdated` for `deck_id`.
fn expect_deck_fx_routed(rx: &mut EvtRx, deck_id: u16, timeout: Duration) -> bool {
    recv_deck_updated(rx, deck_id, timeout).fx_routed
}

#[test]
fn start_stop_null_backend() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    assert!(transport.is_running());
    transport.stop().unwrap();
    assert!(!transport.is_running());
}

#[test]
fn play_on_empty_deck_publishes_error() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    transport.play(0).unwrap();
    let error = recv_error(&mut rx, Duration::from_secs(2));
    assert_eq!(error.deck, Some(0));
}

#[test]
fn load_path_publishes_updated() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    transport
        .load_path(0, short_tone_fixture().to_string_lossy().into_owned())
        .unwrap();
    let update = recv_deck_updated(&mut rx, 0, Duration::from_secs(5));
    assert_eq!(update.deck, 0);
    assert!(
        update.track_id.as_ref().is_some_and(|id| !id.is_empty()),
        "load Updated must carry track_id for waveform fetch, got {:?}",
        update.track_id
    );
    assert!(update.duration_ms.is_some_and(|ms| ms > 0));
}

#[test]
fn set_volume_publishes_updated() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    transport.set_volume(0, 0.25).unwrap();
    let update = recv_deck_updated(&mut rx, 0, Duration::from_secs(2));
    assert_eq!(update.deck, 0);
    assert!((update.volume - 0.25).abs() < 1e-4);
}

#[test]
fn set_crossfader_publishes_status() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    transport.set_crossfader(0.2).unwrap();
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert!((status.crossfader - 0.2).abs() < 1e-4);
}

#[test]
fn set_cue_volume_publishes_status() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    transport.set_cue_volume(0.33).unwrap();
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert!((status.cue_volume - 0.33).abs() < 1e-4);
}

#[test]
fn set_master_volume_publishes_status() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    transport.set_master_volume(0.42).unwrap();
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert!((status.master_volume - 0.42).abs() < 1e-4);
}

#[test]
fn set_cue_mix_publishes_status() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    transport.set_cue_mix(0.35).unwrap();
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert!((status.cue_mix - 0.35).abs() < 1e-4);
}

#[test]
fn set_master_cue_publishes_status() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    transport.set_master_cue(true).unwrap();
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert!(status.master_cue);
}

#[test]
fn load_path_exposes_waveform_peaks() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
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
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    transport
        .load_path(0, short_tone_fixture().to_string_lossy().into_owned())
        .unwrap();
    recv_deck_updated(&mut rx, 0, Duration::from_secs(5));
    transport.seek(0, 20).unwrap();
    recv_deck_updated(&mut rx, 0, Duration::from_secs(2));
}

#[test]
fn set_speed_after_load_publishes_updated() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    load_fixture(&transport, &mut rx, 0);
    transport.set_speed(0, 0.25).unwrap();
    let update = recv_deck_updated(&mut rx, 0, Duration::from_secs(2));
    assert_eq!(update.deck, 0);
    assert!((update.speed - 0.25).abs() < 1e-4);
}

#[test]
fn set_cue_point_after_load_publishes_updated() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    load_fixture(&transport, &mut rx, 0);
    transport.set_cue_point(0).unwrap();
    let update = recv_deck_updated(&mut rx, 0, Duration::from_secs(2));
    assert_eq!(update.deck, 0);
    assert!(update.cue_point_ms.is_some());
}

#[test]
fn loop_in_after_load_publishes_pending_loop_in() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    load_beat_fixture(&transport, &mut rx, 0);
    transport.loop_in(0, 0).unwrap();
    // Load-time ensure analysis supplies BPM when tags omit it.
    let update = recv_deck_updated(&mut rx, 0, Duration::from_secs(2));
    assert_eq!(update.deck, 0);
    assert!(update.active_loop.is_none());
    assert!(update.pending_loop_in_ms.is_some());
}

#[test]
fn set_auto_loop_after_load_publishes_active_loop() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    load_beat_fixture(&transport, &mut rx, 0);
    transport.set_auto_loop(0, 4.0).unwrap();
    let update = recv_deck_updated(&mut rx, 0, Duration::from_secs(2));
    assert_eq!(update.deck, 0);
    let region = update.active_loop.expect("active_loop after set_auto_loop");
    assert!(region.out_ms > region.in_ms);
}

#[test]
fn set_master_deck_publishes_status() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    load_fixture(&transport, &mut rx, 0);
    load_fixture(&transport, &mut rx, 1);
    transport.set_master_deck(1).unwrap();
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert_eq!(status.master_deck, 1);
}

/// A straight two-channel custom group: A fades out, B fades in.
fn linear_group(id: &str, a_start: f32, a_end: f32) -> bus_api::CurveGroupSpec {
    let segment = bus_api::CurveSegmentSpec {
        kind: bus_api::CurveSegmentKind::Linear,
        rate: 0.0,
        c1x: 0.0,
        c1y: 0.0,
        c2x: 0.0,
        c2y: 0.0,
    };
    let curve = |y0: f32, y1: f32| bus_api::CurveSpecSpec {
        points: vec![
            bus_api::CurvePointSpec { x: 0.0, y: y0 },
            bus_api::CurvePointSpec { x: 1.0, y: y1 },
        ],
        segments: vec![segment],
    };
    bus_api::CurveGroupSpec {
        id: id.into(),
        display_name: id.into(),
        curves: vec![curve(a_start, a_end), curve(1.0 - a_start, 1.0 - a_end)],
    }
}

fn curve_set(id: &str, members: [&str; 2]) -> bus_api::CurveSetSpec {
    bus_api::CurveSetSpec {
        id: id.into(),
        display_name: id.into(),
        members: members
            .iter()
            .map(|member| bus_api::CurveRefSpec {
                id: (*member).into(),
                parameter: None,
            })
            .collect(),
    }
}

#[test]
fn crossfader_curve_commands_publish_status_and_notices() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();

    // A preset id selects without a restart and reports back.
    transport.set_crossfader_curve("linear".into()).unwrap();
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert_eq!(status.crossfader_curve.id, "linear");
    assert!(!status.crossfader_curve.hamster);

    // The custom library installs, then a knob sweep selects the set and morphs.
    transport
        .set_curve_library(bus_api::CurveLibrarySpec {
            groups: vec![linear_group("flat", 1.0, 1.0)],
            sets: vec![curve_set("sweep", ["flat", "linear"])],
        })
        .unwrap();
    let _ = recv_engine_status(&mut rx, Duration::from_secs(2));

    transport
        .set_crossfader_curve_amount("sweep".into(), 0.5, false)
        .unwrap();
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert_eq!(status.crossfader_curve.id, "sweep");
    assert!((status.crossfader_curve.morph - 0.5).abs() < 1e-6);

    transport.set_crossfader_behavior(true, false).unwrap();
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert!(status.crossfader_curve.hamster);
    assert!(!status.crossfader_curve.disabled);

    // An unknown id falls back to constant_power and explains itself in a notice.
    transport.set_crossfader_curve("nope".into()).unwrap();
    let notice = recv_matching(
        &mut rx,
        "Notice",
        Duration::from_secs(2),
        |event| match event {
            Event::Notice(notice) => Some(notice),
            _ => None,
        },
    );
    assert!(notice.message.contains("nope"), "{}", notice.message);
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert_eq!(
        status.crossfader_curve.id,
        bus_api::DEFAULT_CROSSFADER_CURVE_ID
    );

    // Duplicate ids reject the update, and the previous library stays usable.
    transport
        .set_curve_library(bus_api::CurveLibrarySpec {
            groups: vec![linear_group("dup", 1.0, 0.0), linear_group("dup", 0.0, 1.0)],
            sets: vec![],
        })
        .unwrap();
    let error = recv_matching(
        &mut rx,
        "Error",
        Duration::from_secs(2),
        |event| match event {
            Event::Error(error) => Some(error),
            _ => None,
        },
    );
    assert!(
        error.message.to_lowercase().contains("duplicate"),
        "{}",
        error.message
    );
    transport.set_crossfader_curve("flat".into()).unwrap();
    let status = recv_engine_status(&mut rx, Duration::from_secs(2));
    assert_eq!(status.crossfader_curve.id, "flat");
}

#[test]
fn set_sampler_bank_publishes_active_bank_id() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let bank_id = library.create_sampler_bank("pads".into(), None).unwrap().id;
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();
    transport.set_sampler_bank(0, bank_id.clone()).unwrap();
    let update = recv_deck_updated(&mut rx, 0, Duration::from_secs(2));
    assert_eq!(update.deck, 0);
    assert_eq!(
        update.active_sampler_bank_id.as_deref(),
        Some(bank_id.as_str())
    );
}

#[test]
fn fx_unit_and_deck_routing_publish_updated() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let transport = testing::start_engine(&bus, &library, null_engine_config()).unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();

    transport.set_fx_unit_slot(0, FxKind::Echo).unwrap();
    // Shared unit: every deck republishes, so drain both before the next command.
    let fx = expect_deck_fx(&mut rx, 0, Duration::from_secs(2));
    assert_eq!(fx.len(), bus_api::FX_SLOT_COUNT);
    assert_eq!(fx[0].kind, bus_api::FxKind::Echo);
    assert!(!fx[0].enabled, "selecting an effect leaves the slot off");
    let _ = recv_deck_updated(&mut rx, 1, Duration::from_secs(2));

    transport.set_fx_unit_slot_enabled(0, true).unwrap();
    let fx = expect_deck_fx(&mut rx, 0, Duration::from_secs(2));
    assert_eq!(fx.len(), bus_api::FX_SLOT_COUNT);
    assert!(fx[0].enabled, "the ON button must enable the slot");
    let _ = recv_deck_updated(&mut rx, 1, Duration::from_secs(2));

    transport.set_fx_unit_wet(0, 0.8).unwrap();
    let fx = expect_deck_fx(&mut rx, 0, Duration::from_secs(2));
    assert_eq!(fx.len(), bus_api::FX_SLOT_COUNT);
    assert!((fx[0].wet - 0.8).abs() < 1e-6, "wet {}", fx[0].wet);
    let _ = recv_deck_updated(&mut rx, 1, Duration::from_secs(2));

    transport.set_fx_unit_param(0, 0, 1.0).unwrap();
    let fx = expect_deck_fx(&mut rx, 0, Duration::from_secs(2));
    assert_eq!(fx.len(), bus_api::FX_SLOT_COUNT);
    // Echo TIME norm 1.0 = the longest table division (2/1 beats).
    assert!((fx[0].params[0] - 1.0).abs() < 1e-6, "{}", fx[0].params[0]);
    let _ = recv_deck_updated(&mut rx, 1, Duration::from_secs(2));

    transport.set_deck_fx_routed(1, true).unwrap();
    assert!(expect_deck_fx_routed(&mut rx, 1, Duration::from_secs(2)));
}
