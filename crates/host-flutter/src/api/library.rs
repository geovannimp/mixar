//! FRB `LibraryTransport`: browse collections + tracks (shared Tauri `library.db`).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use audio_core::{peaks_to_rgb_bytes, WaveformChannelMode};
use library::{
    read_artwork, spawn_library_worker, Evt, HistoryExportFormat, HistorySettings, LibraryBuses,
    LibraryConfig, LibraryManager, LibraryWorker, NewCollection, SamplerBankRecord, TrackId,
    WritableLibrary,
};
use library_api::{
    decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin,
    TrackSummary as ApiTrackSummary,
};
use library_core::{
    AnalysisDurationMode, AudioSource, Collection, CollectionConfig, CollectionId, Library,
};

use crate::frb_generated::StreamSink;

/// Offline analysis depth for library worker configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryAnalysisDurationSetting {
    Fast,
    Precise,
    Complete,
}

impl From<LibraryAnalysisDurationSetting> for AnalysisDurationMode {
    fn from(value: LibraryAnalysisDurationSetting) -> Self {
        match value {
            LibraryAnalysisDurationSetting::Fast => Self::Fast,
            LibraryAnalysisDurationSetting::Precise => Self::Precise,
            LibraryAnalysisDurationSetting::Complete => Self::Complete,
        }
    }
}

/// Sampler bank row for deck default-bank pickers.
#[derive(Clone, Debug)]
pub struct SamplerBankInfo {
    pub id: String,
    pub name: String,
    pub play_mode: Option<SamplerPlayMode>,
    pub sort_index: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplerPlayMode {
    Oneshot,
    Hold,
    Loop,
}

impl From<SamplerBankRecord> for SamplerBankInfo {
    fn from(bank: SamplerBankRecord) -> Self {
        Self {
            id: bank.id,
            name: bank.name,
            play_mode: bank.play_mode.map(|m| match m {
                library::SamplerPlayMode::Oneshot => SamplerPlayMode::Oneshot,
                library::SamplerPlayMode::Hold => SamplerPlayMode::Hold,
                library::SamplerPlayMode::Loop => SamplerPlayMode::Loop,
            }),
            sort_index: bank.sort_index,
        }
    }
}

/// Collection row for the Flutter collections pane (mirrors Tauri `CollectionSummary`).
#[derive(Clone, Debug)]
pub struct LibraryCollectionSummary {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub path: Option<String>,
    pub track_count: u32,
}

/// Track row for the Flutter track table (mirrors Tauri / `library_api::TrackSummary`).
#[derive(Clone, Debug)]
pub struct LibraryTrackSummary {
    pub id: String,
    pub display_name: String,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub duration_ms: Option<i32>,
    pub path: String,
    pub isrc: Option<String>,
    /// Embedded artwork bytes when loaded via [`LibraryTransport::get_track`].
    /// Lists leave this `None` until artwork is persisted in the library DB.
    pub artwork: Option<Vec<u8>>,
}

/// Result of adding a folder collection and syncing it.
#[derive(Clone, Debug)]
pub struct AddFolderCollectionResult {
    pub collection: LibraryCollectionSummary,
    pub added: u32,
    pub updated: u32,
    pub skipped: u32,
    pub failed: u32,
}

/// Path lookup hit: original request path + resolved track summary.
#[derive(Clone, Debug)]
pub struct ResolvedLibraryTrack {
    pub request_path: String,
    pub track: LibraryTrackSummary,
}

/// Packed mono RGB peaks (`count × 3` uint8 bytes).
#[derive(Clone, Debug)]
pub struct WaveformPeaks {
    pub count: u32,
    pub rgb: Vec<u8>,
    pub start_ms: i32,
    pub end_ms: i32,
}

/// Beat-grid overlay data (beat times in seconds).
#[derive(Clone, Debug)]
pub struct BeatGridData {
    pub beats: Vec<f32>,
    pub downbeats: Vec<f32>,
    pub bpm: Option<f64>,
}

/// Performance history session row for the Flutter History pane.
#[derive(Clone, Debug)]
pub struct HistorySessionSummary {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub last_activity_at: String,
    pub closed: bool,
    pub entry_count: u32,
}

/// One committed play in a history session.
#[derive(Clone, Debug)]
pub struct HistoryEntryInfo {
    pub id: String,
    pub deck: u16,
    pub track_id: Option<String>,
    pub location: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_sec: Option<i32>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub isrc: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub played_duration_ms: Option<i64>,
}

/// Prompt shown on app launch when the last session is still inside the idle window.
#[derive(Clone, Debug)]
pub struct HistoryRestorePromptInfo {
    pub session_id: String,
    pub title: String,
    pub last_activity_at: String,
}

/// Derived export format for history sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryExportFormatSetting {
    Csv,
    M3u8,
    Txt,
}

impl From<HistoryExportFormatSetting> for HistoryExportFormat {
    fn from(value: HistoryExportFormatSetting) -> Self {
        match value {
            HistoryExportFormatSetting::Csv => Self::Csv,
            HistoryExportFormatSetting::M3u8 => Self::M3u8,
            HistoryExportFormatSetting::Txt => Self::Txt,
        }
    }
}

/// Discriminator for thin library egress (unit enum — no freezed on Dart).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryEvtKind {
    TrackAnalyzed,
    TrackUpdated,
    Error,
    Notice,
    HotCuesChanged,
    Navigate,
    Load,
    LoopsChanged,
    BeatGridChanged,
}

/// Persisted hot cue row for Dart (`library_api::HotCue`).
#[derive(Clone, Debug)]
pub struct HotCueInfo {
    pub slot: u8,
    pub position_ms: i32,
    pub label: Option<String>,
}

/// Persisted saved-loop row for Dart (`library_api::SavedLoop`).
#[derive(Clone, Debug)]
pub struct SavedLoopInfo {
    pub slot: u8,
    pub in_ms: i32,
    pub out_ms: i32,
    pub label: Option<String>,
}

/// Thin typed library egress for Dart (no MessagePack on the Flutter side).
///
/// Struct + unit kind avoids FRB's freezed dependency for fielded enums.
#[derive(Clone, Debug)]
pub struct LibraryEvt {
    pub kind: LibraryEvtKind,
    pub track: Option<LibraryTrackSummary>,
    pub message: Option<String>,
    pub track_id: Option<String>,
    pub hot_cues: Option<Vec<HotCueInfo>>,
    pub delta: Option<i32>,
    pub deck: Option<u16>,
    pub loops: Option<Vec<SavedLoopInfo>>,
    pub beat_grid: Option<BeatGridData>,
}

struct EvtForwarder {
    shutdown: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

/// Cloneable library cmd/evt buses for hosts that only publish (controller).
#[derive(Clone)]
#[flutter_rust_bridge::frb(opaque)]
pub struct LibraryBusHandle {
    buses: LibraryBuses,
}

impl LibraryBusHandle {
    /// Wrap an existing bus pair (tests / `LibraryTransport::buses`).
    #[flutter_rust_bridge::frb(ignore)]
    pub fn from_buses(buses: LibraryBuses) -> Self {
        Self { buses }
    }

    pub(crate) fn buses(&self) -> LibraryBuses {
        self.buses.clone()
    }
}

/// Host-owned library handle exposed to Dart via FRB methods.
#[flutter_rust_bridge::frb(opaque)]
pub struct LibraryTransport {
    // ponytail: Mutex serializes browse calls per transport. Upgrade to a
    // read-capable manager / connection pool if concurrent queries matter.
    library: Arc<Mutex<LibraryManager>>,
    buses: LibraryBuses,
    /// Drop joins the worker; must not live inside the manager mutex.
    #[allow(dead_code)]
    worker: LibraryWorker,
    /// Drop stops any active Dart evt forwarder.
    evt_forwarder: Mutex<Option<EvtForwarder>>,
}

impl Drop for LibraryTransport {
    fn drop(&mut self) {
        let mut slot = self.evt_forwarder.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(fwd) = slot.take() {
            fwd.shutdown.store(true, Ordering::Relaxed);
            let _ = fwd.handle.join();
        }
    }
}

impl LibraryTransport {
    /// Open (or create) a SQLite library at `db_path`.
    pub fn open(db_path: String) -> Result<Self, String> {
        let manager =
            LibraryManager::open(db_path, LibraryConfig::default()).map_err(|e| e.to_string())?;
        Self::from_manager(manager)
    }

    /// In-memory library for tests.
    pub fn open_in_memory() -> Result<Self, String> {
        let manager =
            LibraryManager::open_in_memory(LibraryConfig::default()).map_err(|e| e.to_string())?;
        Self::from_manager(manager)
    }

    fn from_manager(mut manager: LibraryManager) -> Result<Self, String> {
        let buses = LibraryBuses::new();
        manager.set_buses(buses.clone());
        let library = Arc::new(Mutex::new(manager));
        let worker = spawn_library_worker(Arc::clone(&library)).map_err(|e| e.to_string())?;
        Ok(Self {
            library,
            buses,
            worker,
            evt_forwarder: Mutex::new(None),
        })
    }

    /// List all collections with track counts.
    pub fn list_collections(&self) -> Result<Vec<LibraryCollectionSummary>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        let collections = lib.list_collections().map_err(|e| e.to_string())?;
        collections
            .into_iter()
            .map(|c| collection_summary(&lib, c))
            .collect()
    }

    /// List tracks in a collection (artwork left unset — not stored in DB yet).
    pub fn list_collection_tracks(
        &self,
        collection_id: String,
    ) -> Result<Vec<LibraryTrackSummary>, String> {
        let sources = {
            let lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            lib.get_collection_tracks(&CollectionId::new(collection_id))
                .map_err(|e| e.to_string())?
        };
        Ok(sources
            .iter()
            .filter_map(|s| track_summary(s, false))
            .collect())
    }

    /// Load one track including embedded artwork when present.
    pub fn get_track(&self, track_id: String) -> Result<Option<LibraryTrackSummary>, String> {
        let source = {
            let lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            lib.get_track(&TrackId::new(track_id))
                .map_err(|e| e.to_string())?
        };
        Ok(source.as_ref().and_then(|s| track_summary(s, true)))
    }

    /// Add a folder as a collection and sync its tracks.
    pub fn add_folder_collection(
        &self,
        folder_path: String,
        scan_folder_tree: bool,
        name: Option<String>,
    ) -> Result<AddFolderCollectionResult, String> {
        let new_collection = match name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
            Some(display_name) => NewCollection {
                name: Some(display_name.to_string()),
                config: CollectionConfig::Folder {
                    fs_path: PathBuf::from(&folder_path),
                    scan_folder_tree,
                },
            },
            None => NewCollection::folder_scan(folder_path, scan_folder_tree),
        };
        let collection_id = {
            let mut lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            lib.add_collection(&new_collection)
                .map_err(|e| e.to_string())?
                .id
        };

        // ponytail: sync holds the manager mutex for the whole folder walk/import, so
        // analyze/refresh cmds and other RPCs wait. Upgrade: sync off-mutex with a
        // per-collection lock, or a background scan job that publishes progress evts.
        let (scan, summary) = {
            let mut lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            let scan = lib
                .sync_collection(Some(&collection_id))
                .map_err(|e| e.to_string())?;
            let collection = lib
                .list_collections()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|c| c.id == collection_id)
                .ok_or_else(|| "collection missing after sync".to_string())?;
            let summary = collection_summary(&lib, collection)?;
            (scan, summary)
        };

        Ok(AddFolderCollectionResult {
            collection: summary,
            added: scan.added as u32,
            updated: scan.updated as u32,
            skipped: scan.skipped as u32,
            failed: scan.failed as u32,
        })
    }

    /// Create an empty playlist collection.
    pub fn add_playlist_collection(
        &self,
        name: String,
        sortable: bool,
    ) -> Result<LibraryCollectionSummary, String> {
        let collection = {
            let mut lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            lib.add_collection(&NewCollection::playlist(name.trim(), sortable))
                .map_err(|e| e.to_string())?
        };
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        collection_summary(&lib, collection)
    }

    /// Resolve library tracks for the given filesystem paths.
    pub fn resolve_tracks_for_paths(
        &self,
        paths: Vec<String>,
    ) -> Result<Vec<ResolvedLibraryTrack>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let resolved = lib
            .lookup_file_tracks_at_paths(&path_bufs)
            .map_err(|e| e.to_string())?;
        Ok(resolved
            .into_iter()
            .filter_map(|(request_path, source)| {
                track_summary(&source, false).map(|track| ResolvedLibraryTrack {
                    request_path,
                    track,
                })
            })
            .collect())
    }

    /// Queue analyze for a track via the library cmd bus only (worker emits evt).
    pub fn analyze_track(&self, track_id: String, force: bool) -> Result<(), String> {
        let bytes = encode_cmd_body(&CmdBody::AnalyzeTrack { track_id, force })
            .map_err(|e| e.to_string())?;
        self.buses
            .publish_cmd(Origin::Library, Kind::AnalyzeTrack, bytes)
            .map_err(|e| e.to_string())
    }

    /// L0 overview peaks from the library DB, if present.
    pub fn get_waveform_overview(&self, track_id: String) -> Result<Option<WaveformPeaks>, String> {
        let id = TrackId::new(track_id);
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        let Some(row) = lib
            .get_track_waveform_overview(&id)
            .map_err(|e| e.to_string())?
        else {
            return Ok(None);
        };
        let duration_ms = lib
            .get_track(&id)
            .map_err(|e| e.to_string())?
            .and_then(|t| t.metadata().duration_ms)
            .unwrap_or(0);
        Ok(Some(pack_peaks(&row.peaks, 0, duration_ms)))
    }

    /// L1 JIT window peaks from the decode cache (decodes the file if needed).
    pub fn get_waveform_window(
        &self,
        track_id: String,
        start_ms: i32,
        end_ms: i32,
        buckets: u32,
    ) -> Result<WaveformPeaks, String> {
        let id = TrackId::new(track_id);
        let (peaks, start_ms, end_ms) = LibraryManager::compute_waveform_window(
            &self.library,
            &id,
            start_ms,
            end_ms,
            buckets as usize,
        )
        .map_err(|e| e.to_string())?;
        Ok(pack_peaks(&peaks, start_ms, end_ms))
    }

    /// Analyzed beat grid, if present.
    pub fn get_beat_grid(&self, track_id: String) -> Result<Option<BeatGridData>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        let Some(grid) = lib
            .get_track_beat_grid(&TrackId::new(track_id))
            .map_err(|e| e.to_string())?
        else {
            return Ok(None);
        };
        Ok(Some(BeatGridData {
            beats: grid.beats,
            downbeats: grid.downbeats,
            bpm: grid.bpm,
        }))
    }

    /// Queue metadata refresh for a track via the library cmd bus only.
    pub fn refresh_track(&self, track_id: String) -> Result<(), String> {
        let bytes =
            encode_cmd_body(&CmdBody::RefreshTrack { track_id }).map_err(|e| e.to_string())?;
        self.buses
            .publish_cmd(Origin::Library, Kind::RefreshTrack, bytes)
            .map_err(|e| e.to_string())
    }

    /// Persist an active loop region for a track (worker emits [`LibraryEvtKind::LoopsChanged`]).
    pub fn save_loop(
        &self,
        track_id: String,
        slot: u8,
        in_ms: i32,
        out_ms: i32,
    ) -> Result<(), String> {
        if slot > 15 {
            return Err("loop slot must be 0..=15".into());
        }
        if out_ms <= in_ms {
            return Err("loop out must be after loop in".into());
        }
        let bytes = encode_cmd_body(&CmdBody::SaveLoop {
            track_id,
            slot,
            in_ms,
            out_ms,
            label: None,
            color: None,
        })
        .map_err(|e| e.to_string())?;
        self.buses
            .publish_cmd(Origin::Library, Kind::SaveLoop, bytes)
            .map_err(|e| e.to_string())
    }

    /// Delete a saved loop slot (worker emits [`LibraryEvtKind::LoopsChanged`]).
    pub fn delete_loop(&self, track_id: String, slot: u8) -> Result<(), String> {
        if slot > 15 {
            return Err("loop slot must be 0..=15".into());
        }
        let bytes =
            encode_cmd_body(&CmdBody::DeleteLoop { track_id, slot }).map_err(|e| e.to_string())?;
        self.buses
            .publish_cmd(Origin::Library, Kind::DeleteLoop, bytes)
            .map_err(|e| e.to_string())
    }

    /// Persist a manually edited beat grid (worker emits [`LibraryEvtKind::BeatGridChanged`]).
    pub fn save_beat_grid(
        &self,
        track_id: String,
        bpm: f64,
        first_beat_secs: f32,
    ) -> Result<(), String> {
        if !(bpm > 20.0 && bpm < 400.0) {
            return Err("beat grid bpm must be between 20 and 400".into());
        }
        let bytes = encode_cmd_body(&CmdBody::SaveBeatGrid {
            track_id,
            bpm,
            first_beat_secs,
        })
        .map_err(|e| e.to_string())?;
        self.buses
            .publish_cmd(Origin::Library, Kind::SaveBeatGrid, bytes)
            .map_err(|e| e.to_string())
    }

    /// Forward thin typed library events to Dart via FRB `StreamSink`.
    ///
    /// Replaces any previous forwarder so repeated subscribe calls do not leak threads.
    pub fn subscribe_events(&self, sink: StreamSink<LibraryEvt>) -> Result<(), String> {
        let rx = self.buses.subscribe_evt_all().map_err(|e| e.to_string())?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);
        let handle = std::thread::Builder::new()
            .name("library-evt-forwarder".into())
            .spawn(move || {
                while !shutdown_flag.load(Ordering::Relaxed) {
                    match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(Some(ev)) => {
                            if let Some(mapped) = map_library_evt(ev.as_ref()) {
                                if sink.add(mapped).is_err() {
                                    break;
                                }
                            }
                        }
                        Ok(None) => continue,
                        Err(_) => break,
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        let mut slot = self.evt_forwarder.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(prev) = slot.take() {
            prev.shutdown.store(true, Ordering::Relaxed);
            let _ = prev.handle.join();
        }
        *slot = Some(EvtForwarder { shutdown, handle });
        Ok(())
    }

    /// Raw evt subscription for host/tests. Prefer [`Self::subscribe_events`] for Dart.
    #[flutter_rust_bridge::frb(ignore)]
    pub fn subscribe_evt_all(&self) -> Result<library::EvtReceiver, String> {
        self.buses.subscribe_evt_all().map_err(|e| e.to_string())
    }

    /// Shared manager for engine load-to-deck (prepare outside the engine lock).
    #[flutter_rust_bridge::frb(ignore)]
    pub fn library_arc(&self) -> Arc<Mutex<LibraryManager>> {
        Arc::clone(&self.library)
    }

    /// Library cmd bus clone for `Engine::new_with_library_bus`.
    #[flutter_rust_bridge::frb(ignore)]
    pub fn cmd_bus(&self) -> library::LibraryBus {
        self.buses.cmd_bus()
    }

    /// Clone of the library cmd/evt buses for [`crate::api::controller::ControllerTransport`].
    pub fn buses(&self) -> LibraryBusHandle {
        LibraryBusHandle::from_buses(self.buses.clone())
    }

    /// Apply library analysis duration from app settings.
    pub fn apply_library_settings(
        &self,
        analysis_duration: LibraryAnalysisDurationSetting,
    ) -> Result<(), String> {
        self.buses.set_analysis_duration(analysis_duration.into());
        Ok(())
    }

    /// Apply performance history settings from app settings.
    pub fn apply_history_settings(
        &self,
        enabled: bool,
        session_idle_minutes: u32,
        min_play_seconds: u32,
        min_deck_volume: f32,
    ) -> Result<(), String> {
        if session_idle_minutes == 0 {
            return Err("history_session_idle_minutes must be > 0".into());
        }
        if min_play_seconds == 0 {
            return Err("history_min_play_seconds must be > 0".into());
        }
        if !min_deck_volume.is_finite() || !(0.0..=1.0).contains(&min_deck_volume) {
            return Err("history_min_deck_volume must be between 0 and 1".into());
        }
        let mut lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.set_history_settings(HistorySettings {
            enabled,
            session_idle_minutes,
            min_play_seconds,
            min_deck_volume,
        });
        Ok(())
    }

    /// Restore prompt when the last session is still inside the idle window.
    pub fn history_restore_prompt(&self) -> Result<Option<HistoryRestorePromptInfo>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        Ok(lib
            .history_restore_prompt()
            .map(|p| HistoryRestorePromptInfo {
                session_id: p.session_id,
                title: p.title,
                last_activity_at: p.last_activity_at,
            }))
    }

    pub fn history_restore_session(&self, session_id: String) -> Result<(), String> {
        let mut lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.history_restore_session(&session_id)
            .map_err(|e| e.to_string())
    }

    pub fn history_decline_restore(&self) -> Result<(), String> {
        let mut lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.history_decline_restore().map_err(|e| e.to_string())
    }

    pub fn history_new_session(&self) -> Result<(), String> {
        let mut lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.history_new_session().map_err(|e| e.to_string())
    }

    pub fn history_resume_session(&self) -> Result<(), String> {
        let mut lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.history_resume_session().map_err(|e| e.to_string())
    }

    pub fn history_can_resume(&self) -> Result<bool, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        Ok(lib.history_can_resume())
    }

    pub fn list_history_sessions(&self) -> Result<Vec<HistorySessionSummary>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.list_history_sessions()
            .map(|rows| {
                rows.into_iter()
                    .map(|row| HistorySessionSummary {
                        id: row.id,
                        title: row.title,
                        started_at: row.started_at,
                        last_activity_at: row.last_activity_at,
                        closed: row.closed,
                        entry_count: row.entry_count,
                    })
                    .collect()
            })
            .map_err(|e| e.to_string())
    }

    pub fn history_session_entries(
        &self,
        session_id: String,
    ) -> Result<Vec<HistoryEntryInfo>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.history_session_entries(&session_id)
            .map(|entries| entries.into_iter().map(history_entry_info).collect())
            .map_err(|e| e.to_string())
    }

    pub fn rename_history_session(&self, session_id: String, title: String) -> Result<(), String> {
        let mut lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.rename_history_session(&session_id, title.trim())
            .map_err(|e| e.to_string())
    }

    pub fn delete_history_session(&self, session_id: String) -> Result<(), String> {
        let mut lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.delete_history_session(&session_id)
            .map_err(|e| e.to_string())
    }

    pub fn export_history_session(
        &self,
        session_id: String,
        format: HistoryExportFormatSetting,
        dest_path: String,
    ) -> Result<(), String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.export_history_session(&session_id, format.into(), Path::new(&dest_path))
            .map_err(|e| e.to_string())
    }

    pub fn save_history_as_playlist(
        &self,
        session_id: String,
        name: String,
        sortable: bool,
    ) -> Result<LibraryCollectionSummary, String> {
        let collection = {
            let mut lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            lib.save_history_as_playlist(&session_id, name.trim(), sortable)
                .map_err(|e| e.to_string())?
        };
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        collection_summary(&lib, collection)
    }

    pub fn history_dir(&self) -> Result<String, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        Ok(lib.history_dir().to_string_lossy().into_owned())
    }

    pub fn reveal_history_folder(&self) -> Result<(), String> {
        let dir = {
            let lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            lib.history_dir().to_path_buf()
        };
        reveal_path_in_file_manager(&dir)
    }

    pub fn update_track_isrc(&self, track_id: String, isrc: Option<String>) -> Result<(), String> {
        let normalized = isrc.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.update_track_isrc(&TrackId::new(track_id), normalized)
            .map_err(|e| e.to_string())
    }

    /// Sampler banks stored in the library DB.
    pub fn list_sampler_banks(&self) -> Result<Vec<SamplerBankInfo>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.list_sampler_banks()
            .map(|rows| rows.into_iter().map(SamplerBankInfo::from).collect())
            .map_err(|e| e.to_string())
    }
}

/// Map omnibus library egress to the thin Dart-facing evt.
pub(crate) fn map_library_evt(ev: &Evt) -> Option<LibraryEvt> {
    let body = decode_evt_body(ev.payload()).ok()?;
    match body {
        EvtBody::TrackAnalyzed { track } => Some(LibraryEvt {
            kind: LibraryEvtKind::TrackAnalyzed,
            track: Some(api_track_summary(track)),
            message: None,
            track_id: None,
            hot_cues: None,
            delta: None,
            deck: None,
            loops: None,
            beat_grid: None,
        }),
        EvtBody::TrackUpdated { track } => Some(LibraryEvt {
            kind: LibraryEvtKind::TrackUpdated,
            track: Some(api_track_summary(track)),
            message: None,
            track_id: None,
            hot_cues: None,
            delta: None,
            deck: None,
            loops: None,
            beat_grid: None,
        }),
        EvtBody::Error { message, track_id } => Some(LibraryEvt {
            kind: LibraryEvtKind::Error,
            track: None,
            message: Some(message),
            track_id,
            hot_cues: None,
            delta: None,
            deck: None,
            loops: None,
            beat_grid: None,
        }),
        EvtBody::Notice { message } => Some(LibraryEvt {
            kind: LibraryEvtKind::Notice,
            track: None,
            message: Some(message),
            track_id: None,
            hot_cues: None,
            delta: None,
            deck: None,
            loops: None,
            beat_grid: None,
        }),
        EvtBody::HotCuesChanged { track_id, hot_cues } => Some(LibraryEvt {
            kind: LibraryEvtKind::HotCuesChanged,
            track: None,
            message: None,
            track_id: Some(track_id),
            hot_cues: Some(
                hot_cues
                    .into_iter()
                    .map(|c| HotCueInfo {
                        slot: c.slot,
                        position_ms: c.position_ms,
                        label: c.label,
                    })
                    .collect(),
            ),
            delta: None,
            deck: None,
            loops: None,
            beat_grid: None,
        }),
        EvtBody::LoopsChanged { track_id, loops } => Some(LibraryEvt {
            kind: LibraryEvtKind::LoopsChanged,
            track: None,
            message: None,
            track_id: Some(track_id),
            hot_cues: None,
            delta: None,
            deck: None,
            loops: Some(
                loops
                    .into_iter()
                    .map(|l| SavedLoopInfo {
                        slot: l.slot,
                        in_ms: l.in_ms,
                        out_ms: l.out_ms,
                        label: l.label,
                    })
                    .collect(),
            ),
            beat_grid: None,
        }),
        EvtBody::BeatGridChanged {
            track_id,
            beat_grid,
        } => Some(LibraryEvt {
            kind: LibraryEvtKind::BeatGridChanged,
            track: None,
            message: None,
            track_id: Some(track_id),
            hot_cues: None,
            delta: None,
            deck: None,
            loops: None,
            beat_grid: Some(BeatGridData {
                beats: beat_grid.beats,
                downbeats: beat_grid.downbeats,
                bpm: Some(beat_grid.bpm),
            }),
        }),
        EvtBody::Navigate { delta } => Some(LibraryEvt {
            kind: LibraryEvtKind::Navigate,
            track: None,
            message: None,
            track_id: None,
            hot_cues: None,
            delta: Some(delta),
            deck: None,
            loops: None,
            beat_grid: None,
        }),
        EvtBody::Load { deck } => Some(LibraryEvt {
            kind: LibraryEvtKind::Load,
            track: None,
            message: None,
            track_id: None,
            hot_cues: None,
            delta: None,
            deck: Some(deck),
            loops: None,
            beat_grid: None,
        }),
        EvtBody::Empty => None,
    }
}

fn api_track_summary(track: ApiTrackSummary) -> LibraryTrackSummary {
    LibraryTrackSummary {
        id: track.id,
        display_name: track.display_name,
        artist: track.artist,
        title: track.title,
        album: track.album,
        genre: track.genre,
        bpm: track.bpm,
        key: track.key,
        duration_ms: track.duration_ms,
        path: track.path,
        isrc: track.isrc,
        artwork: None,
    }
}

fn history_entry_info(entry: library::HistoryEntry) -> HistoryEntryInfo {
    HistoryEntryInfo {
        id: entry.id,
        deck: entry.deck,
        track_id: entry.track_id,
        location: entry.location,
        title: entry.title,
        artist: entry.artist,
        album: entry.album,
        duration_sec: entry.duration_sec,
        bpm: entry.bpm,
        key: entry.key,
        isrc: entry.isrc,
        started_at: entry.started_at,
        ended_at: entry.ended_at,
        played_duration_ms: entry.played_duration_ms,
    }
}

fn reveal_path_in_file_manager(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(path);
        cmd
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut cmd = std::process::Command::new("open");
        cmd.arg(path);
        cmd
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = std::process::Command::new("explorer");
        cmd.arg(path);
        cmd
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Err("reveal in folder is unsupported on this platform".into());

    command.spawn().map_err(|e| e.to_string())?;
    Ok(())
}

fn collection_summary(
    library: &LibraryManager,
    collection: Collection,
) -> Result<LibraryCollectionSummary, String> {
    let track_count = library
        .get_collection_tracks(&collection.id)
        .map_err(|e| e.to_string())?
        .len() as u32;
    Ok(LibraryCollectionSummary {
        id: collection.id.as_str().to_string(),
        name: collection.name.clone(),
        kind: collection.collection_type().as_str().to_string(),
        path: collection
            .fs_path()
            .map(|path| path.to_string_lossy().into_owned()),
        track_count,
    })
}

fn track_summary(source: &AudioSource, include_artwork: bool) -> Option<LibraryTrackSummary> {
    let file = source.file()?;
    let metadata = source.metadata();
    let path = file.path().to_path_buf();
    let artwork = if include_artwork {
        read_artwork(Path::new(&path)).ok().flatten()
    } else {
        None
    };
    Some(LibraryTrackSummary {
        id: source.id().as_str().to_string(),
        display_name: track_display_name(source),
        artist: metadata.artist.clone(),
        title: metadata.title.clone(),
        album: metadata.album.clone(),
        genre: metadata.genre.clone(),
        bpm: metadata.bpm,
        key: metadata.key.clone(),
        duration_ms: metadata.duration_ms,
        path: path.to_string_lossy().into_owned(),
        isrc: metadata.isrc.clone(),
        artwork,
    })
}

fn pack_peaks(peaks: &[audio_core::SpectralPeak], start_ms: i32, end_ms: i32) -> WaveformPeaks {
    WaveformPeaks {
        count: peaks.len() as u32,
        rgb: peaks_to_rgb_bytes(peaks, WaveformChannelMode::Mono),
        start_ms,
        end_ms,
    }
}

fn track_display_name(source: &AudioSource) -> String {
    let metadata = source.metadata();
    if let Some(title) = metadata.title.as_ref().filter(|t| !t.is_empty()) {
        return title.clone();
    }
    source
        .file()
        .and_then(|f| {
            f.path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| source.id().as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use library_api::EvtBody;
    use std::time::Duration;

    fn sample_api_track() -> ApiTrackSummary {
        ApiTrackSummary {
            id: "t1".into(),
            display_name: "Track One".into(),
            artist: Some("Artist".into()),
            title: Some("Title".into()),
            album: None,
            genre: None,
            bpm: Some(128.0),
            key: Some("Am".into()),
            duration_ms: Some(60_000),
            path: "/music/one.wav".into(),
            isrc: None,
        }
    }

    #[test]
    fn map_library_evt_maps_thin_kinds_including_navigate_load_loops() {
        let buses = LibraryBuses::new();
        let rx = buses.subscribe_evt_all().unwrap();

        buses
            .publish_evt(
                Origin::Library,
                Kind::Error,
                EvtBody::Error {
                    message: "missing".into(),
                    track_id: Some("t1".into()),
                },
            )
            .unwrap();
        let ev = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        let mapped = map_library_evt(ev.as_ref()).expect("Error maps");
        assert_eq!(mapped.kind, LibraryEvtKind::Error);
        assert_eq!(mapped.message.as_deref(), Some("missing"));
        assert_eq!(mapped.track_id.as_deref(), Some("t1"));
        assert!(mapped.hot_cues.is_none());

        buses
            .publish_evt(
                Origin::Library,
                Kind::TrackUpdated,
                EvtBody::TrackUpdated {
                    track: sample_api_track(),
                },
            )
            .unwrap();
        let ev = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        let mapped = map_library_evt(ev.as_ref()).expect("TrackUpdated maps");
        assert_eq!(mapped.kind, LibraryEvtKind::TrackUpdated);
        let track = mapped.track.expect("track");
        assert_eq!(track.id, "t1");
        assert_eq!(track.display_name, "Track One");
        assert_eq!(track.bpm, Some(128.0));

        buses
            .publish_evt(
                Origin::Library,
                Kind::HotCuesChanged,
                EvtBody::HotCuesChanged {
                    track_id: "t1".into(),
                    hot_cues: vec![library_api::HotCue {
                        slot: 0,
                        position_ms: 12_500,
                        loop_length_beats: None,
                        color: None,
                        label: Some("intro".into()),
                    }],
                },
            )
            .unwrap();
        let ev = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        let mapped = map_library_evt(ev.as_ref()).expect("HotCuesChanged maps");
        assert_eq!(mapped.kind, LibraryEvtKind::HotCuesChanged);
        assert_eq!(mapped.track_id.as_deref(), Some("t1"));
        let cues = mapped.hot_cues.expect("cues");
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].slot, 0);
        assert_eq!(cues[0].position_ms, 12_500);
        assert_eq!(cues[0].label.as_deref(), Some("intro"));

        buses
            .publish_evt(
                Origin::LibraryNavigation,
                Kind::Navigate,
                EvtBody::Navigate { delta: 1 },
            )
            .unwrap();
        let ev = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        let mapped = map_library_evt(ev.as_ref()).expect("Navigate maps");
        assert_eq!(mapped.kind, LibraryEvtKind::Navigate);
        assert_eq!(mapped.delta, Some(1));

        buses
            .publish_evt(
                Origin::LibraryNavigation,
                Kind::Load,
                EvtBody::Load { deck: 0 },
            )
            .unwrap();
        let ev = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        let mapped = map_library_evt(ev.as_ref()).expect("Load maps");
        assert_eq!(mapped.kind, LibraryEvtKind::Load);
        assert_eq!(mapped.deck, Some(0));

        buses
            .publish_evt(
                Origin::Library,
                Kind::LoopsChanged,
                EvtBody::LoopsChanged {
                    track_id: "t1".into(),
                    loops: vec![library_api::SavedLoop {
                        slot: 2,
                        in_ms: 1_000,
                        out_ms: 5_000,
                        label: Some("break".into()),
                        color: None,
                    }],
                },
            )
            .unwrap();
        let ev = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        let mapped = map_library_evt(ev.as_ref()).expect("LoopsChanged maps");
        assert_eq!(mapped.kind, LibraryEvtKind::LoopsChanged);
        assert_eq!(mapped.track_id.as_deref(), Some("t1"));
        let loops = mapped.loops.expect("loops");
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].slot, 2);
        assert_eq!(loops[0].in_ms, 1_000);
        assert_eq!(loops[0].out_ms, 5_000);
        assert_eq!(loops[0].label.as_deref(), Some("break"));
    }
}
