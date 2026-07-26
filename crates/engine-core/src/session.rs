//! Engine session: owns `Engine`, omnibus buses, and the control thread.

use crate::bus::{new_buses, EngineBus};
use crate::config::EngineConfig;
use crate::engine::Engine;
use anyhow::Result;
use engine_api::{encode_evt_body, EvtBody, Kind, Origin};
use omnibus::{Event, Filter};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Owns the engine, cmd/evt omnibus buses, revision counter, and control thread.
pub struct EngineSession {
    cmd_bus: EngineBus,
    evt_bus: EngineBus,
    engine: Arc<Mutex<Option<Engine>>>,
    revision: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
    control_thread: Option<JoinHandle<()>>,
}

impl EngineSession {
    /// Create a session with fresh buses and a control thread (handlers filled in Task 3).
    pub fn new(config: EngineConfig) -> Result<Self> {
        let (cmd_bus, evt_bus) = new_buses();
        let engine = Arc::new(Mutex::new(Some(Engine::new(config)?)));
        let revision = Arc::new(AtomicU64::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));

        let cmd = cmd_bus.clone();
        let evt = evt_bus.clone();
        let shutdown_flag = Arc::clone(&shutdown);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<()>>();
        let control_thread = thread::spawn(move || {
            control_thread_loop(cmd, evt, shutdown_flag, ready_tx);
        });
        ready_rx
            .recv()
            .map_err(|_| anyhow::anyhow!("control thread failed to start"))??;

        Ok(Self {
            cmd_bus,
            evt_bus,
            engine,
            revision,
            shutdown,
            control_thread: Some(control_thread),
        })
    }

    /// Clone handle to the command ingress bus.
    pub fn cmd_bus(&self) -> EngineBus {
        self.cmd_bus.clone()
    }

    /// Clone handle to the event egress bus.
    pub fn evt_bus(&self) -> EngineBus {
        self.evt_bus.clone()
    }

    /// Monotonic revision bumped when discrete engine state changes (Task 3).
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    /// Fire-and-forget publish of a command (nested `CmdBody` bytes in payload).
    pub fn publish_cmd(&self, origin: Origin, kind: Kind, body: impl AsRef<[u8]>) -> Result<()> {
        self.cmd_bus
            .publish(Event::new(origin, kind, Arc::from(body.as_ref())))?;
        Ok(())
    }

    /// Run a closure against the owned engine (start/stop, load track, etc.).
    pub fn with_engine<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Engine) -> Result<R>,
    {
        let mut guard = self.engine.lock().unwrap();
        let engine = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("engine not available"))?;
        f(engine)
    }
}

impl Drop for EngineSession {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.control_thread.take() {
            let _ = handle.join();
        }
    }
}

/// Task 2 stub: echo `Kind::Error` for every cmd until Task 3 adds real handlers.
fn control_thread_loop(
    cmd_bus: EngineBus,
    evt_bus: EngineBus,
    shutdown: Arc<AtomicBool>,
    ready: std::sync::mpsc::Sender<Result<()>>,
) {
    let rx = match cmd_bus.subscribe(Filter::Any, Filter::Any) {
        Ok(rx) => rx,
        Err(e) => {
            let _ = ready.send(Err(e.into()));
            return;
        }
    };
    let _ = ready.send(Ok(()));
    while !shutdown.load(Ordering::Relaxed) {
        let event = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Some(event)) => event,
            Ok(None) => continue,
            Err(_) => break,
        };
        let origin = event.origin().clone();
        let body = match encode_evt_body(&EvtBody::Error {
            message: "no handler".into(),
        }) {
            Ok(body) => body,
            Err(_) => continue,
        };
        let _ = evt_bus.publish(Event::new(origin, Kind::Error, Arc::from(body)));
    }
}
