//! Engine session: owns `Engine`, omnibus buses, and the control thread.

use crate::bus::{new_buses, subscribe_evt_all, EngineBus, EvtReceiver};
use crate::config::EngineConfig;
use crate::control::control_thread_loop;
use crate::engine::Engine;
use anyhow::Result;
use engine_api::{encode_evt_body, EvtBody, Kind, Origin};
use library::LibraryManager;
use omnibus::Event;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

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
    /// Create a session with fresh buses and a control thread.
    pub fn new(config: EngineConfig) -> Result<Self> {
        Self::new_inner(config, None)
    }

    /// Create a session with a shared concrete library manager.
    pub fn new_with_library(
        config: EngineConfig,
        library: Arc<Mutex<LibraryManager>>,
    ) -> Result<Self> {
        Self::new_inner(config, Some(library))
    }

    fn new_inner(
        config: EngineConfig,
        library: Option<Arc<Mutex<LibraryManager>>>,
    ) -> Result<Self> {
        let (cmd_bus, evt_bus) = new_buses();
        let engine = Arc::new(Mutex::new(Some(match library {
            Some(library) => Engine::new_with_library(config, library)?,
            None => Engine::new(config)?,
        })));
        let revision = Arc::new(AtomicU64::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));

        let cmd = cmd_bus.clone();
        let evt = evt_bus.clone();
        let engine_handle = Arc::clone(&engine);
        let revision_handle = Arc::clone(&revision);
        let shutdown_flag = Arc::clone(&shutdown);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<()>>();
        let control_thread = thread::spawn(move || {
            control_thread_loop(
                cmd,
                evt,
                engine_handle,
                revision_handle,
                shutdown_flag,
                ready_tx,
            );
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

    /// Subscribe to all egress events (host bridge / MIDI). Hides omnibus filters.
    pub fn subscribe_evt_all(&self) -> Result<EvtReceiver> {
        Ok(subscribe_evt_all(&self.evt_bus)?)
    }

    /// Monotonic revision bumped when discrete engine state changes.
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    /// Fire-and-forget publish of a command (nested `CmdBody` bytes in payload).
    pub fn publish_cmd(&self, origin: Origin, kind: Kind, body: impl AsRef<[u8]>) -> Result<()> {
        self.cmd_bus
            .publish(Event::new(origin, kind, Arc::from(body.as_ref())))?;
        Ok(())
    }

    /// Host/engine egress: encode `EvtBody`, bump revision, publish on the evt bus.
    pub fn publish_evt(&self, origin: Origin, kind: Kind, body: EvtBody) -> Result<()> {
        let bytes = encode_evt_body(&body)?;
        self.revision.fetch_add(1, Ordering::Relaxed);
        self.evt_bus
            .publish(Event::new(origin, kind, Arc::from(bytes)))?;
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
