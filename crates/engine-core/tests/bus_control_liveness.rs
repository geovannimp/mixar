//! The control thread keeps making progress under a sustained command stream and
//! stops promptly when the host drops it.
//!
//! The old loop ticked after every command and drained the library lane until it
//! was empty, so a producer that never let a queue go idle could amplify
//! telemetry and delay shutdown. The watchdog here only fails a hang: progress
//! is asserted on events, not on sleeps.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use bus_api::Command;
use bus_api::SetMasterVolume;
use engine_core::EngineConfig;
use engine_core::EngineSession;

/// Finite deadline: a hang must fail the test rather than block CI forever.
const WATCHDOG: Duration = Duration::from_secs(5);

#[test]
fn a_sustained_command_stream_still_publishes_and_stops_on_drop() {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    let mut rx = session.subscribe_evt_all("test").expect("evt");

    let stop = Arc::new(AtomicBool::new(false));
    let producer = {
        // The producer publishes outside the session so it never holds the engine lock.
        let bus = session.bus();
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                // More than one batch per wake-up, so the queue is never idle.
                for _ in 0..128 {
                    bus.publish_cmd(Command::SetMasterVolume(SetMasterVolume {
                        volume: 0.5,
                        soft_takeover: false,
                    }))
                    .expect("engine inbox");
                }
            }
        })
    };

    // Egress must keep flowing while the command lane is saturated.
    let deadline = Instant::now() + WATCHDOG;
    let mut progressed = false;
    while Instant::now() < deadline && !progressed {
        progressed = matches!(rx.recv_timeout(Duration::from_millis(50)), Ok(Some(_)));
    }

    stop.store(true, Ordering::Relaxed);
    producer.join().expect("producer");
    assert!(progressed, "no egress under a sustained command stream");

    // Stop is out of band and joins the worker, so it must not wait for the
    // backlog. The drop runs on its own thread: if the join hangs, the watchdog
    // fails here instead of blocking the test forever.
    let (elapsed_tx, elapsed_rx) = std::sync::mpsc::channel();
    let stopper = std::thread::spawn(move || {
        let started = Instant::now();
        drop(session);
        let _ = elapsed_tx.send(started.elapsed());
    });
    let elapsed = elapsed_rx
        .recv_timeout(WATCHDOG)
        .expect("worker stop/join did not finish inside the watchdog");
    stopper.join().expect("stopper");
    assert!(
        elapsed < WATCHDOG,
        "worker stop/join took {elapsed:?}, longer than the watchdog"
    );
}
