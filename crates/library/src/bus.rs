//! Omnibus cmd/evt buses for library control and egress.

use crate::{LibraryError, Result};
use library_api::{encode_evt_body, EvtBody, Kind, Origin};
use library_core::AnalysisDurationMode;
use omnibus::{Event, Filter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Shared omnibus type for cmd and evt buses (payload = nested MessagePack body bytes).
pub type LibraryBus = omnibus::Bus<Origin, Kind, Arc<[u8]>>;

/// Host-facing evt subscription (all origins/kinds).
pub type EvtReceiver = omnibus::BusReceiver<Origin, Kind, Arc<[u8]>>;

/// Shared evt handle type for host bridges.
pub type Evt = omnibus::Event<Origin, Kind, Arc<[u8]>>;

/// Host-owned cloneable cmd/evt bus bundle + shared revision / analysis duration.
///
/// Hosts create this once, inject clones into [`crate::LibraryManager`] via
/// `set_buses`, and keep a handle so controller/engine can `publish_evt`
/// without locking the DB mutex.
///
/// `revision` is bumped on every egress publish so hosts can stamp wire messages
/// (Tauri `library://bus`) with a monotonic library-state counter.
/// `analysis_duration` is the worker default for `AnalyzeTrack` when the cmd
/// does not override duration (shared with engine/settings at session start).
/// `stems_format` / `stems_root` / `models_root` configure the standalone
/// `GenerateStems` action and the stem cache read on deck load.
#[derive(Clone)]
pub struct LibraryBuses {
    cmd: LibraryBus,
    evt: LibraryBus,
    revision: Arc<AtomicU64>,
    analysis_duration: Arc<Mutex<AnalysisDurationMode>>,
    stems_format: Arc<Mutex<String>>,
    stems_root: Arc<Mutex<std::path::PathBuf>>,
    models_root: Arc<Mutex<std::path::PathBuf>>,
}

impl LibraryBuses {
    /// Create a matched cmd/evt pair with shared revision and analysis duration.
    pub fn new() -> Self {
        let (cmd, evt) = new_buses();
        Self {
            cmd,
            evt,
            revision: Arc::new(AtomicU64::new(0)),
            analysis_duration: Arc::new(Mutex::new(AnalysisDurationMode::default())),
            stems_format: Arc::new(Mutex::new(String::from("opus"))),
            stems_root: Arc::new(Mutex::new(std::path::PathBuf::from("stems"))),
            models_root: Arc::new(Mutex::new(std::path::PathBuf::from("models"))),
        }
    }

    /// Fire-and-forget publish of a command (nested `CmdBody` bytes in payload).
    pub fn publish_cmd(&self, origin: Origin, kind: Kind, body: impl AsRef<[u8]>) -> Result<()> {
        self.cmd
            .publish(Event::new(origin, kind, Arc::from(body.as_ref())))
            .map_err(omnibus_err)?;
        Ok(())
    }

    /// Encode `EvtBody`, bump revision, publish on the evt bus.
    pub fn publish_evt(&self, origin: Origin, kind: Kind, body: EvtBody) -> Result<()> {
        let bytes = encode_evt_body(&body)
            .map_err(|e| LibraryError::Io(std::io::Error::other(e.to_string())))?;
        self.revision.fetch_add(1, Ordering::Relaxed);
        self.evt
            .publish(Event::new(origin, kind, Arc::from(bytes)))
            .map_err(omnibus_err)?;
        Ok(())
    }

    /// Subscribe to every egress event (UI / bridge).
    pub fn subscribe_evt_all(&self) -> Result<EvtReceiver> {
        self.evt
            .subscribe(Filter::Any, Filter::Any)
            .map_err(omnibus_err)
    }

    /// Subscribe to egress events for one track (`Origin::Track`).
    pub fn subscribe_evt_track(&self, track_id: impl Into<String>) -> Result<EvtReceiver> {
        self.evt
            .subscribe(Filter::Is(Origin::Track(track_id.into())), Filter::Any)
            .map_err(omnibus_err)
    }

    /// Update default analysis duration used by AnalyzeTrack cmds.
    pub fn set_analysis_duration(&self, duration: AnalysisDurationMode) {
        *self
            .analysis_duration
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = duration;
    }

    /// Stem cache codec (`opus` | `flac`). AAC is normalized to opus until encode ships.
    pub fn set_stems_format(&self, format: impl Into<String>) {
        let format = format.into();
        let normalized = match format.as_str() {
            "flac" => "flac".to_string(),
            _ => "opus".to_string(),
        };
        *self.stems_format.lock().unwrap_or_else(|e| e.into_inner()) = normalized;
    }

    /// Root directory for stem caches (`{root}/{fnv64(track_id)}/`).
    pub fn set_stems_root(&self, root: std::path::PathBuf) {
        *self.stems_root.lock().unwrap_or_else(|e| e.into_inner()) = root;
    }

    /// Root directory for HTDemucs ONNX weights.
    pub fn set_models_root(&self, root: std::path::PathBuf) {
        *self.models_root.lock().unwrap_or_else(|e| e.into_inner()) = root;
    }

    /// Current stem cache codec (`opus` | `flac`).
    pub fn stems_format(&self) -> String {
        self.stems_format
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Clone of the configured stems root.
    pub fn stems_root(&self) -> std::path::PathBuf {
        self.stems_root
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Clone of the configured models root.
    pub fn models_root(&self) -> std::path::PathBuf {
        self.models_root
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Monotonic revision bumped when discrete library state changes.
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    /// Clone handle to the command ingress bus.
    pub fn cmd_bus(&self) -> LibraryBus {
        self.cmd.clone()
    }

    /// Clone handle to the event egress bus.
    pub fn evt_bus(&self) -> LibraryBus {
        self.evt.clone()
    }

    pub(crate) fn revision_arc(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.revision)
    }

    pub(crate) fn analysis_duration_arc(&self) -> Arc<Mutex<AnalysisDurationMode>> {
        Arc::clone(&self.analysis_duration)
    }
}

impl Default for LibraryBuses {
    fn default() -> Self {
        Self::new()
    }
}

fn omnibus_err(e: omnibus::OmnibusError) -> LibraryError {
    LibraryError::Io(std::io::Error::other(e.to_string()))
}

/// Create a matched cmd/evt bus pair.
///
/// Cmd capacity is large so bulk analyze (one cmd per track) can queue without
/// dropping while the worker runs analysis serially.
pub fn new_buses() -> (LibraryBus, LibraryBus) {
    (
        LibraryBus::with_capacity(4096),
        LibraryBus::with_capacity(2048),
    )
}
