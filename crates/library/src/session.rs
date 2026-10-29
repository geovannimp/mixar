//! Thin shim: host-owned bus + manager + worker (Tauri compatibility).
//!
//! Prefer attaching [`crate::LibraryBuses`] to [`crate::LibraryManager`] and
//! [`crate::spawn_library_worker`] directly. This type preserves the old
//! `LibrarySession` surface for existing hosts.

use crate::bus::{LibraryBus, LibraryBuses};
use crate::worker::{spawn_library_worker, LibraryWorker};
use crate::{LibraryConfig, LibraryManager, Result};
use bus_api::Event;
use event_bus::Subscriber;
use library_core::AnalysisDurationMode;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Owns the library manager, the shared typed bus handle, and the worker thread.
///
/// Shim over host-owned [`LibraryBuses`] + [`LibraryManager`] + [`LibraryWorker`].
pub struct LibrarySession {
    library: Arc<Mutex<LibraryManager>>,
    buses: LibraryBuses,
    /// Kept so Drop joins the worker; must not live inside the manager mutex.
    #[allow(dead_code)]
    worker: LibraryWorker,
}

impl LibrarySession {
    /// Open a library database and start the cmd worker.
    pub fn open(db_path: impl AsRef<Path>, config: LibraryConfig) -> Result<Self> {
        let manager = LibraryManager::open(db_path, config)?;
        Self::from_manager(manager)
    }

    /// In-memory library for tests.
    pub fn open_in_memory(config: LibraryConfig) -> Result<Self> {
        let manager = LibraryManager::open_in_memory(config)?;
        Self::from_manager(manager)
    }

    fn from_manager(mut manager: LibraryManager) -> Result<Self> {
        let buses = LibraryBuses::new();
        manager.set_buses(buses.clone());
        let library = Arc::new(Mutex::new(manager));
        let worker = spawn_library_worker(Arc::clone(&library))?;
        Ok(Self {
            library,
            buses,
            worker,
        })
    }

    /// Shared concrete library manager (engine / host invoke paths).
    pub fn library(&self) -> Arc<Mutex<LibraryManager>> {
        Arc::clone(&self.library)
    }

    /// Update default analysis duration used by AnalyzeTrack cmds.
    pub fn set_analysis_duration(&self, duration: AnalysisDurationMode) {
        self.buses.set_analysis_duration(duration);
    }

    /// Clone handle to the shared typed bus (engine / host publish path).
    pub fn bus(&self) -> LibraryBus {
        self.buses.bus()
    }

    /// Subscribe to all egress events (host bridge / test ack sync).
    pub fn subscribe_evt(&self) -> Result<Subscriber<Event>> {
        self.buses.subscribe_evt()
    }

    /// Fire-and-forget publish of a command on the shared typed bus.
    pub fn publish_cmd(&self, event: Event) -> Result<()> {
        self.buses.publish_cmd(event)
    }

    /// Fire-and-forget publish of an egress event on the shared typed bus.
    pub fn publish_evt(&self, event: Event) -> Result<()> {
        self.buses.publish_evt(event)
    }
}

// Drop joins via `LibraryWorker::drop` — do not double-join here.
