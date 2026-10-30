//! Shared helpers for `engine-core` integration tests.
//!
//! These used to be copy-pasted into every `bus_*.rs` file and had drifted into
//! five different bodies with four different deadlines. The worst divergence was
//! the receive loop: ten copies wrote
//!
//! ```ignore
//! sub.recv_timeout(...).expect("recv").expect("event")
//! ```
//!
//! but `Subscriber::recv_timeout` returns `Ok(None)` on timeout, so the
//! `.expect("event")` panicked after the first 50 ms poll. That made the
//! surrounding "deadline" loop dead code and the effective timeout 50 ms — a
//! flake under CI load (the control thread ticks every 33 ms).
//!
//! Every wait now goes through [`recv_evt`], which polls in short slices so a
//! timeout stays a timeout, and through the typed wrappers below, so a test
//! asserts on a payload field instead of decoding an envelope.
//!
//! Each integration-test binary compiles this module in full but uses only a
//! subset of it, hence the module-level `dead_code` allow.

#![allow(dead_code)]
#![allow(clippy::unwrap_used)] // test helper module

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bus_api::DeckLevels;
use bus_api::DeckPosition;
use bus_api::DeckUpdated;
use bus_api::EngineSnapshot;
use bus_api::Error;
use bus_api::Event;
use engine_core::EngineSession;
use event_bus::Subscriber;
use library_core::{AudioSource, FileAudioSource, TrackId, TrackMetadata};

/// Receiver type returned by `EngineSession::subscribe_evt_all` and
/// `Engine::subscribe_evt_all`.
pub type TestReceiver = Subscriber<Event>;

/// Default deadline for [`recv_evt`].
pub const RECV_TIMEOUT: Duration = Duration::from_secs(5);

/// Receive the next event satisfying `pred`, skipping every other event, until
/// [`RECV_TIMEOUT`] elapses.
///
/// A disconnected bus fails immediately with the real error rather than spinning
/// until the deadline and reporting a misleading timeout.
pub fn recv_evt(sub: &mut TestReceiver, mut pred: impl FnMut(&Event) -> bool) -> Event {
    let deadline = Instant::now() + RECV_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            panic!("timeout waiting for a matching evt after {RECV_TIMEOUT:?}");
        }
        match sub.recv_timeout(remaining.min(Duration::from_millis(50))) {
            // Timeout: nothing published yet, keep waiting out the deadline.
            Ok(None) => {}
            Ok(Some(event)) => {
                if pred(&event) {
                    return event;
                }
            }
            Err(error) => panic!("evt bus disconnected while waiting for an evt: {error}"),
        }
    }
}

/// Next `DeckUpdated`, optionally restricted by `pred` on the payload.
pub fn recv_updated_where(
    sub: &mut TestReceiver,
    mut pred: impl FnMut(&DeckUpdated) -> bool,
) -> DeckUpdated {
    let event = recv_evt(sub, |event| match event {
        Event::DeckUpdated(update) => pred(update),
        _ => false,
    });
    expect_updated(event)
}

/// Next `DeckUpdated` for any deck.
pub fn recv_updated(sub: &mut TestReceiver) -> DeckUpdated {
    recv_updated_where(sub, |_| true)
}

/// Next `DeckPosition`.
pub fn recv_position(sub: &mut TestReceiver) -> DeckPosition {
    expect_position(recv_evt(sub, |event| {
        matches!(event, Event::DeckPosition(_))
    }))
}

/// Next `DeckLevels`.
pub fn recv_levels(sub: &mut TestReceiver) -> DeckLevels {
    expect_levels(recv_evt(sub, |event| matches!(event, Event::DeckLevels(_))))
}

/// Next whole-engine snapshot.
pub fn recv_snapshot(sub: &mut TestReceiver) -> EngineSnapshot {
    expect_snapshot(recv_evt(sub, |event| {
        matches!(event, Event::EngineSnapshot(_))
    }))
}

/// Next error egress.
pub fn recv_error(sub: &mut TestReceiver) -> Error {
    expect_error(recv_evt(sub, |event| matches!(event, Event::Error(_))))
}

/// Unwrap a received event as `DeckUpdated`.
pub fn expect_updated(event: Event) -> DeckUpdated {
    match event {
        Event::DeckUpdated(update) => update,
        other => panic!("expected DeckUpdated, got {other:?}"),
    }
}

/// Unwrap a received event as `DeckPosition`.
pub fn expect_position(event: Event) -> DeckPosition {
    match event {
        Event::DeckPosition(position) => position,
        other => panic!("expected DeckPosition, got {other:?}"),
    }
}

/// Unwrap a received event as `DeckLevels`.
pub fn expect_levels(event: Event) -> DeckLevels {
    match event {
        Event::DeckLevels(levels) => levels,
        other => panic!("expected DeckLevels, got {other:?}"),
    }
}

/// Unwrap a received event as `EngineSnapshot`.
pub fn expect_snapshot(event: Event) -> EngineSnapshot {
    match event {
        Event::EngineSnapshot(snapshot) => snapshot,
        other => panic!("expected EngineSnapshot, got {other:?}"),
    }
}

/// Unwrap a received event as `Error`.
pub fn expect_error(event: Event) -> Error {
    match event {
        Event::Error(error) => error,
        other => panic!("expected Error, got {other:?}"),
    }
}

/// The checked-in, CI-friendly audio fixture.
pub fn short_tone_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-tone.wav")
}

/// Length of [`short_tone_fixture`] in milliseconds.
///
/// Keep in sync with `samples/fixtures/short-tone.wav` (documented as 0.25 s in
/// `samples/README.md`). Tests that depend on the fixture duration assert
/// against this so the number lives in one place.
pub const SHORT_TONE_LEN_MS: i32 = 250;

/// A file source backed by [`short_tone_fixture`] carrying `bpm` metadata.
pub fn source_with_bpm(id: &str, bpm: f64) -> AudioSource {
    AudioSource::File(FileAudioSource::new(
        TrackId::new(id),
        short_tone_fixture(),
        TrackMetadata {
            bpm: Some(bpm),
            ..Default::default()
        },
    ))
}

/// Write a 2-second silent mono 16-bit PCM WAV (48 kHz) for library import tests.
pub fn write_two_second_wav(path: &Path) {
    let sample_rate = 48_000u32;
    let sample_count = (sample_rate * 2) as usize;
    let pcm = vec![0u8; sample_count * 2];
    let data_size = pcm.len() as u32;
    let file_size = 36 + data_size;
    let byte_rate = sample_rate * 2;
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&file_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    file.write_all(&pcm).unwrap();
}

/// Wait until `deck_id` engine memory reports `count` hot cues (library ack).
pub fn wait_engine_hot_cue_count(session: &EngineSession, deck_id: usize, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let len = session
            .with_engine(|engine| {
                Ok(engine
                    .deck_snapshot(deck_id)
                    .expect("deck snapshot")
                    .hot_cues
                    .len())
            })
            .expect("engine available");
        if len == count {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timeout waiting for engine deck {deck_id} hot_cues.len() == {count}");
}
