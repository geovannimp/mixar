//! Shared helpers for `engine-core` integration tests.
//!
//! These used to be copy-pasted into every `bus_*.rs` file and had drifted into
//! five different bodies with four different deadlines. The worst divergence was
//! in `recv_evt_kind`: ten copies wrote
//!
//! ```ignore
//! sub.recv_timeout(...).expect("recv").expect("event")
//! ```
//!
//! but `omnibus::BusReceiver::recv_timeout` returns `Ok(None)` on timeout, so the
//! `.expect("event")` panicked after the first 50 ms poll. That made the
//! surrounding "deadline" loop dead code and the effective timeout 50 ms — a
//! flake under CI load (the control thread ticks every 33 ms).
//!
//! Each integration-test binary compiles this module in full but uses only a
//! subset of it, hence the module-level `dead_code` allow.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine_api::{Kind, Origin};
use library_core::{AudioSource, FileAudioSource, TrackId, TrackMetadata};

/// Event type carried by the engine's evt bus.
pub type TestEvent = omnibus::Event<Origin, Kind, Arc<[u8]>>;

/// Receiver type returned by `EngineSession::evt_bus().subscribe(..)`.
pub type TestReceiver = omnibus::BusReceiver<Origin, Kind, Arc<[u8]>>;

/// Default deadline for [`recv_evt_kind`].
pub const RECV_TIMEOUT: Duration = Duration::from_secs(2);

/// Receive the next event of `kind`, skipping every other kind, until
/// [`RECV_TIMEOUT`] elapses.
///
/// A disconnected bus fails immediately with the real error rather than spinning
/// until the deadline and reporting a misleading timeout.
pub fn recv_evt_kind(sub: &TestReceiver, kind: Kind) -> TestEvent {
    recv_evt_where(sub, kind, |_| true)
}

/// [`recv_evt_kind`] restricted to events whose decoded body also satisfies
/// `pred`. Use this instead of hand-rolling a receive loop when the assertion
/// depends on a body field (e.g. a specific deck id), so every wait in the suite
/// reports timeout and disconnect failures identically.
pub fn recv_evt_where(
    sub: &TestReceiver,
    kind: Kind,
    mut pred: impl FnMut(&TestEvent) -> bool,
) -> TestEvent {
    let deadline = Instant::now() + RECV_TIMEOUT;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match sub.recv_timeout(remaining.min(Duration::from_millis(50))) {
            Ok(Some(event)) => {
                if *event.kind() == kind && pred(&event) {
                    return (*event).clone();
                }
            }
            // Timeout: nothing published yet, keep waiting out the deadline.
            Ok(None) => {}
            Err(error) => panic!("evt bus disconnected while waiting for {kind:?}: {error}"),
        }
    }
    panic!("timeout waiting for evt kind {kind:?} after {RECV_TIMEOUT:?}");
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
