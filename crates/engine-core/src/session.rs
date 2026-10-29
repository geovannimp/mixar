//! Thin shim: one bus + engine + cmd worker (host compatibility).
//!
//! Prefer attaching the bus to [`crate::Engine`] and calling
//! [`crate::spawn_engine_worker`] directly. This type preserves the
//! `EngineSession` surface used by tests and offline tools.

use crate::bus::new_event_bus;
use crate::config::EngineConfig;
use crate::control::{spawn_engine_worker, EngineWorker};
use crate::engine::Engine;
use anyhow::Result;
use bus_api::Event;
use event_bus::EventBus;
use event_bus::Subscriber;
use library::{LibraryBus, LibraryManager};
use std::sync::{Arc, Mutex};

/// Owns the engine, the shared cmd/evt bus and the cmd worker.
///
/// Shim over [`Engine`] + [`EngineWorker`].
pub struct EngineSession {
    /// Declared first so Drop joins the worker before `engine` is destroyed.
    #[allow(dead_code)]
    worker: EngineWorker,
    engine: Arc<Mutex<Option<Engine>>>,
    buses: EventBus<Event>,
}

impl EngineSession {
    /// Create a session with a fresh bus and a cmd worker.
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
        let buses = new_event_bus();
        engine.set_buses(buses.clone());
        let engine = Arc::new(Mutex::new(Some(engine)));
        let worker = spawn_engine_worker(Arc::clone(&engine))?;
        Ok(Self {
            worker,
            engine,
            buses,
        })
    }

    /// Clone of the cmd/evt bus.
    pub fn bus(&self) -> EventBus<Event> {
        self.buses.clone()
    }

    /// Subscribe to every egress event. Hides the subscription plumbing.
    pub fn subscribe_evt_all(&self) -> Result<Subscriber<Event>> {
        Ok(self.buses.subscribe_evt()?)
    }

    /// Fire-and-forget publish of a command.
    pub fn publish_cmd(&self, event: Event) -> Result<()> {
        self.buses.publish_cmd(event)?;
        Ok(())
    }

    /// Host/engine egress: publish on the evt bus.
    pub fn publish_evt(&self, event: Event) -> Result<()> {
        self.buses.publish_evt(event)?;
        Ok(())
    }

    /// Run a closure against the owned engine (start/stop, load track, etc.).
    pub fn with_engine<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Engine) -> Result<R>,
    {
        let mut guard = self.engine.lock().expect("engine handle mutex poisoned");
        let engine = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("engine not available"))?;
        f(engine)
    }
}
