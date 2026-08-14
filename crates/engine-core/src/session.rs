//! Thin shim: host-owned buses + engine + control thread (Tauri compatibility).
//!
//! Prefer attaching [`crate::EngineBuses`] to [`crate::Engine`] and
//! [`crate::spawn_engine_control`] directly. This type preserves the old
//! `EngineSession` surface for existing hosts.

use crate::bus::{EngineBus, EngineBuses, EvtReceiver};
use crate::config::EngineConfig;
use crate::control::{spawn_engine_control, EngineControl};
use crate::engine::Engine;
use anyhow::Result;
use engine_api::{EvtBody, Kind, Origin};
use library::{LibraryBus, LibraryManager};
use std::sync::{Arc, Mutex};

/// Owns the engine, cmd/evt omnibus buses, revision counter, and control thread.
///
/// Shim over host-owned [`EngineBuses`] + [`Engine`] + [`EngineControl`].
pub struct EngineSession {
    engine: Arc<Mutex<Option<Engine>>>,
    buses: EngineBuses,
    /// Kept so Drop joins the control thread; must not live inside the engine mutex.
    #[allow(dead_code)]
    control: EngineControl,
}

impl EngineSession {
    /// Create a session with fresh buses and a control thread.
    pub fn new(config: EngineConfig) -> Result<Self> {
        Self::from_engine(Engine::new(config)?)
    }

    /// Create a session with a shared concrete library manager.
    pub fn new_with_library(
        config: EngineConfig,
        library: Arc<Mutex<LibraryManager>>,
    ) -> Result<Self> {
        Self::from_engine(Engine::new_with_library(config, library)?)
    }

    /// Create a session with library manager + library cmd bus for performance persistence.
    pub fn new_with_library_bus(
        config: EngineConfig,
        library: Arc<Mutex<LibraryManager>>,
        library_cmd: LibraryBus,
    ) -> Result<Self> {
        Self::from_engine(Engine::new_with_library_bus(config, library, library_cmd)?)
    }

    fn from_engine(mut engine: Engine) -> Result<Self> {
        let buses = EngineBuses::new();
        engine.set_buses(buses.clone());
        let engine = Arc::new(Mutex::new(Some(engine)));
        let control = spawn_engine_control(Arc::clone(&engine))?;
        Ok(Self {
            engine,
            buses,
            control,
        })
    }

    /// Clone handle to the command ingress bus.
    pub fn cmd_bus(&self) -> EngineBus {
        self.buses.cmd_bus()
    }

    /// Clone handle to the event egress bus.
    pub fn evt_bus(&self) -> EngineBus {
        self.buses.evt_bus()
    }

    /// Subscribe to all egress events (host bridge / MIDI). Hides omnibus filters.
    pub fn subscribe_evt_all(&self) -> Result<EvtReceiver> {
        self.buses.subscribe_evt_all()
    }

    /// Monotonic revision bumped when discrete engine state changes.
    pub fn revision(&self) -> u64 {
        self.buses.revision()
    }

    /// Fire-and-forget publish of a command (nested `CmdBody` bytes in payload).
    pub fn publish_cmd(&self, origin: Origin, kind: Kind, body: impl AsRef<[u8]>) -> Result<()> {
        self.buses.publish_cmd(origin, kind, body)
    }

    /// Host/engine egress: encode `EvtBody`, bump revision, publish on the evt bus.
    pub fn publish_evt(&self, origin: Origin, kind: Kind, body: EvtBody) -> Result<()> {
        self.buses.publish_evt(origin, kind, body)
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
