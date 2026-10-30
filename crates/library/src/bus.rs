//! The library's cmd/evt pair on the shared typed [`event_bus::EventBus`].

use crate::LibraryError;
use crate::Result;
use bus_api::Event;
use event_bus::EventBus;
use event_bus::Subscriber;
use library_core::AnalysisDurationMode;
use std::sync::Arc;
use std::sync::Mutex;

/// Shared bus type: one [`Event`] enum carries the engine's, the library's and
/// (later) the controller's events, so a host subscribes once.
pub type LibraryBus = EventBus<Event>;

/// Command depth. Deeper than the engine's 256 because bulk analysis and bulk
/// stem generation queue one command per track, and the worker drains them
/// serially; a collection-wide run must not overflow while it works through the
/// backlog.
const CMD_CAPACITY: usize = 4096;

/// Egress depth. Per subscriber, so it has to hold a UI drain gap without
/// dropping discrete events. Same sizing the engine evt side uses.
const EVT_CAPACITY: usize = 2048;

/// Host-owned cloneable bus handle + the config the worker reads.
///
/// Hosts create this once, inject clones into [`crate::LibraryManager`] via
/// `set_buses`, and keep a handle so the engine/controller can publish without
/// locking the DB mutex.
///
/// `analysis_duration` is the worker default for `AnalyzeTrack` when the cmd
/// does not override duration (shared with engine/settings at session start).
/// `stems_format` / `stems_model` / `stems_root` / `models_root` configure the
/// standalone `GenerateStems` action and the stem cache read on deck load.
/// Default offline separation model (HTDemucs ONNX).
pub const DEFAULT_STEMS_MODEL: &str = "htdemucs_mixxx_v1";
/// Optional SCNet int8 ONNX (spectrogram I/O, faster segments).
pub const SCNET_STEMS_MODEL: &str = "scnet_int8_v1";

#[derive(Clone)]
pub struct LibraryBuses {
    buses: EventBus<Event>,
    analysis_duration: Arc<Mutex<AnalysisDurationMode>>,
    stems_format: Arc<Mutex<String>>,
    stems_model: Arc<Mutex<String>>,
    stems_root: Arc<Mutex<std::path::PathBuf>>,
    models_root: Arc<Mutex<std::path::PathBuf>>,
    /// Serial stem-job queue. `GenerateStems` cmds enqueue a track id here and a
    /// single dedicated thread drains them one at a time — Demucs/ONNX passes
    /// are memory-heavy, so the old spawn-a-thread-per-cmd could OOM on bulk
    /// queues. Installed by [`crate::spawn_library_worker`]; `None` before that
    /// (cmd handlers fall back to a one-off thread).
    stem_queue: Arc<Mutex<Option<std::sync::mpsc::Sender<String>>>>,
}

impl LibraryBuses {
    /// Create a private cmd/evt pair (tests, offline hosts).
    pub fn new() -> Self {
        Self::with_bus(EventBus::new(CMD_CAPACITY, EVT_CAPACITY))
    }

    /// Attach to the process-wide bus the engine and controller already use.
    pub fn with_bus(bus: EventBus<Event>) -> Self {
        Self {
            buses: bus,
            analysis_duration: Arc::new(Mutex::new(AnalysisDurationMode::default())),
            stems_format: Arc::new(Mutex::new(String::from("opus"))),
            stems_model: Arc::new(Mutex::new(DEFAULT_STEMS_MODEL.to_string())),
            stems_root: Arc::new(Mutex::new(std::path::PathBuf::from("stems"))),
            models_root: Arc::new(Mutex::new(std::path::PathBuf::from("models"))),
            stem_queue: Arc::new(Mutex::new(None)),
        }
    }

    /// Fire-and-forget publish of a command.
    pub fn publish_cmd(&self, event: Event) -> Result<()> {
        self.buses.publish_cmd(event).map_err(omnibus_err)
    }

    /// Fire-and-forget publish of an egress event.
    pub fn publish_evt(&self, event: Event) -> Result<()> {
        self.buses.publish_evt(event).map_err(omnibus_err)
    }

    /// Subscribe to every command (the worker).
    pub fn subscribe_cmd(&self) -> Result<Subscriber<Event>> {
        self.buses.subscribe_cmd().map_err(omnibus_err)
    }

    /// Subscribe to every egress event (UI / bridge / engine ack sync).
    pub fn subscribe_evt(&self) -> Result<Subscriber<Event>> {
        self.buses.subscribe_evt().map_err(omnibus_err)
    }

    /// Clone handle to the shared bus (hosts that publish outside these setters).
    pub fn bus(&self) -> EventBus<Event> {
        self.buses.clone()
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

    /// Separation model id (`htdemucs_mixxx_v1` | `scnet_int8_v1`).
    pub fn set_stems_model(&self, model: impl Into<String>) {
        let model = model.into();
        let normalized = match model.as_str() {
            SCNET_STEMS_MODEL => SCNET_STEMS_MODEL.to_string(),
            _ => DEFAULT_STEMS_MODEL.to_string(),
        };
        *self.stems_model.lock().unwrap_or_else(|e| e.into_inner()) = normalized;
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

    /// Current separation model id.
    pub fn stems_model(&self) -> String {
        self.stems_model
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

    /// Install the serial stem-job queue sender (called by the worker spawn).
    pub(crate) fn set_stem_queue(&self, tx: std::sync::mpsc::Sender<String>) {
        *self.stem_queue.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
    }

    /// Enqueue a stem job. Returns false when no serial queue is installed.
    pub(crate) fn queue_stem(&self, track_id: String) -> bool {
        self.stem_queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|tx| tx.send(track_id).is_ok())
            .unwrap_or(false)
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

fn omnibus_err(e: event_bus::BusError) -> LibraryError {
    LibraryError::Io(std::io::Error::other(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_model_defaults_and_normalizes() {
        let buses = LibraryBuses::new();
        assert_eq!(buses.stems_model(), DEFAULT_STEMS_MODEL);
        buses.set_stems_model(SCNET_STEMS_MODEL);
        assert_eq!(buses.stems_model(), SCNET_STEMS_MODEL);
        buses.set_stems_model("unknown");
        assert_eq!(buses.stems_model(), DEFAULT_STEMS_MODEL);
    }

    #[test]
    fn subscription_open_before_publish_receives_commands() {
        let buses = LibraryBuses::new();
        let mut cmd = buses.subscribe_cmd().expect("cmd subscription");
        buses
            .publish_cmd(Event::RefreshTrack(bus_api::RefreshTrack {
                track_id: "t1".into(),
            }))
            .expect("publish");
        assert_eq!(
            cmd.recv_timeout(std::time::Duration::from_millis(200))
                .expect("recv"),
            Some(Event::RefreshTrack(bus_api::RefreshTrack {
                track_id: "t1".into()
            }))
        );
    }
}
