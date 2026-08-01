//! Library session: owns `LibraryManager`, omnibus buses, and the worker thread.

use crate::bus::{new_buses, subscribe_evt_all, subscribe_evt_track, EvtReceiver, LibraryBus};
use crate::worker::worker_thread_loop;
use crate::{LibraryConfig, LibraryError, LibraryManager, Result};
use library_api::{encode_evt_body, EvtBody, Kind, Origin};
use library_core::AnalysisDurationMode;
use omnibus::Event;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Owns the library manager, cmd/evt omnibus buses, revision counter, and worker thread.
pub struct LibrarySession {
    cmd_bus: LibraryBus,
    evt_bus: LibraryBus,
    library: Arc<Mutex<LibraryManager>>,
    analysis_duration: Arc<Mutex<AnalysisDurationMode>>,
    revision: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl LibrarySession {
    /// Open a library database and start the cmd worker.
    pub fn open(db_path: impl AsRef<Path>, config: LibraryConfig) -> Result<Self> {
        let manager = LibraryManager::open(db_path, config)?;
        Self::from_manager(manager, AnalysisDurationMode::default())
    }

    /// In-memory library for tests.
    pub fn open_in_memory(config: LibraryConfig) -> Result<Self> {
        let manager = LibraryManager::open_in_memory(config)?;
        Self::from_manager(manager, AnalysisDurationMode::default())
    }

    fn from_manager(
        manager: LibraryManager,
        analysis_duration: AnalysisDurationMode,
    ) -> Result<Self> {
        let (cmd_bus, evt_bus) = new_buses();
        let library = Arc::new(Mutex::new(manager));
        let analysis_duration = Arc::new(Mutex::new(analysis_duration));
        let revision = Arc::new(AtomicU64::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));

        let cmd = cmd_bus.clone();
        let evt = evt_bus.clone();
        let library_handle = Arc::clone(&library);
        let duration_handle = Arc::clone(&analysis_duration);
        let revision_handle = Arc::clone(&revision);
        let shutdown_flag = Arc::clone(&shutdown);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<std::result::Result<(), String>>();
        let worker = thread::spawn(move || {
            worker_thread_loop(
                cmd,
                evt,
                library_handle,
                duration_handle,
                revision_handle,
                shutdown_flag,
                ready_tx,
            );
        });
        ready_rx
            .recv()
            .map_err(|_| LibraryError::Io(std::io::Error::other("library worker failed to start")))?
            .map_err(|e| LibraryError::Io(std::io::Error::other(e)))?;

        Ok(Self {
            cmd_bus,
            evt_bus,
            library,
            analysis_duration,
            revision,
            shutdown,
            worker: Some(worker),
        })
    }

    /// Shared concrete library manager (engine / host invoke paths).
    pub fn library(&self) -> Arc<Mutex<LibraryManager>> {
        Arc::clone(&self.library)
    }

    /// Update default analysis duration used by AnalyzeTrack cmds.
    pub fn set_analysis_duration(&self, duration: AnalysisDurationMode) {
        *self
            .analysis_duration
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = duration;
    }

    /// Clone handle to the command ingress bus.
    pub fn cmd_bus(&self) -> LibraryBus {
        self.cmd_bus.clone()
    }

    /// Clone handle to the event egress bus.
    pub fn evt_bus(&self) -> LibraryBus {
        self.evt_bus.clone()
    }

    /// Subscribe to all egress events (host bridge).
    pub fn subscribe_evt_all(&self) -> Result<EvtReceiver> {
        subscribe_evt_all(&self.evt_bus)
            .map_err(|e| LibraryError::Io(std::io::Error::other(e.to_string())))
    }

    /// Subscribe to egress events for one track (`Origin::Track`).
    pub fn subscribe_evt_track(&self, track_id: impl Into<String>) -> Result<EvtReceiver> {
        subscribe_evt_track(&self.evt_bus, track_id)
            .map_err(|e| LibraryError::Io(std::io::Error::other(e.to_string())))
    }

    /// Monotonic revision bumped when discrete library state changes.
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    /// Fire-and-forget publish of a command (nested `CmdBody` bytes in payload).
    pub fn publish_cmd(&self, origin: Origin, kind: Kind, body: impl AsRef<[u8]>) -> Result<()> {
        self.cmd_bus
            .publish(Event::new(origin, kind, Arc::from(body.as_ref())))
            .map_err(|e| LibraryError::Io(std::io::Error::other(e.to_string())))?;
        Ok(())
    }

    /// Host/library egress: encode `EvtBody`, bump revision, publish on the evt bus.
    pub fn publish_evt(&self, origin: Origin, kind: Kind, body: EvtBody) -> Result<()> {
        let bytes = encode_evt_body(&body)
            .map_err(|e| LibraryError::Io(std::io::Error::other(e.to_string())))?;
        self.revision.fetch_add(1, Ordering::Relaxed);
        self.evt_bus
            .publish(Event::new(origin, kind, Arc::from(bytes)))
            .map_err(|e| LibraryError::Io(std::io::Error::other(e.to_string())))?;
        Ok(())
    }
}

impl Drop for LibrarySession {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}
