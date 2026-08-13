//! Thin shim: host-owned buses + manager + worker (Tauri compatibility).
//!
//! Prefer attaching [`crate::LibraryBuses`] to [`crate::LibraryManager`] and
//! [`crate::spawn_library_worker`] directly. This type preserves the old
//! `LibrarySession` surface for existing hosts.

use crate::bus::{EvtReceiver, LibraryBus, LibraryBuses};
use crate::worker::{spawn_library_worker, LibraryWorker};
use crate::{LibraryConfig, LibraryManager, Result};
use library_api::{EvtBody, Kind, Origin};
use library_core::AnalysisDurationMode;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Owns the library manager, cmd/evt omnibus buses, revision counter, and worker thread.
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

    /// Clone handle to the command ingress bus.
    pub fn cmd_bus(&self) -> LibraryBus {
        self.buses.cmd_bus()
    }

    /// Clone handle to the event egress bus.
    pub fn evt_bus(&self) -> LibraryBus {
        self.buses.evt_bus()
    }

    /// Subscribe to all egress events (host bridge).
    pub fn subscribe_evt_all(&self) -> Result<EvtReceiver> {
        self.buses.subscribe_evt_all()
    }

    /// Subscribe to egress events for one track (`Origin::Track`).
    pub fn subscribe_evt_track(&self, track_id: impl Into<String>) -> Result<EvtReceiver> {
        self.buses.subscribe_evt_track(track_id)
    }

    /// Monotonic revision bumped when discrete library state changes.
    pub fn revision(&self) -> u64 {
        self.buses.revision()
    }

    /// Fire-and-forget publish of a command (nested `CmdBody` bytes in payload).
    pub fn publish_cmd(&self, origin: Origin, kind: Kind, body: impl AsRef<[u8]>) -> Result<()> {
        self.buses.publish_cmd(origin, kind, body)
    }

    /// Host/library egress: encode `EvtBody`, bump revision, publish on the evt bus.
    pub fn publish_evt(&self, origin: Origin, kind: Kind, body: EvtBody) -> Result<()> {
        self.buses.publish_evt(origin, kind, body)
    }
}

// Drop joins via `LibraryWorker::drop` — do not double-join here.
