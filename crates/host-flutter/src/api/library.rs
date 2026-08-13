//! FRB `LibraryTransport`: browse collections + tracks (shared Tauri `library.db`).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use audio_core::{ms_to_secs, secs_to_ms, SpectralPeak};
use library::{
    read_artwork, spawn_library_worker, BeatGridSnapshot, Evt, LibraryBuses, LibraryConfig,
    LibraryManager, LibraryWorker, NewCollection, TrackId, WritableLibrary,
};
use library_api::{
    decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin,
    TrackSummary as ApiTrackSummary,
};
use library_core::{AudioSource, Collection, CollectionId, Library};

use crate::frb_generated::StreamSink;
use crate::waveform_render::{pack_waveform_frame, render_scrolling_lane, WaveformDisplayGains};

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

/// Request for a Tauri-compatible packed scrolling waveform lane (`WFR1`).
#[derive(Clone, Debug)]
pub struct RenderWaveformLaneRequest {
    pub track_id: Option<String>,
    pub path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub position_ms: i32,
    pub visible_ms: i32,
    pub buffer_ratio: f64,
    pub include_detail: bool,
    pub include_beat_grid: bool,
    pub eq_low_db: f32,
    pub eq_mid_db: f32,
    pub eq_high_db: f32,
}

/// Discriminator for thin library egress (unit enum — no freezed on Dart).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryEvtKind {
    TrackAnalyzed,
    TrackUpdated,
    Error,
    Notice,
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
    /// Stops `subscribe_events` forwarder threads on transport drop.
    evt_forwarder_shutdown: Arc<AtomicBool>,
}

impl Drop for LibraryTransport {
    fn drop(&mut self) {
        self.evt_forwarder_shutdown.store(true, Ordering::Relaxed);
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
            evt_forwarder_shutdown: Arc::new(AtomicBool::new(false)),
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

    /// List tracks in a collection.
    pub fn list_collection_tracks(
        &self,
        collection_id: String,
    ) -> Result<Vec<LibraryTrackSummary>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        let tracks = lib
            .get_collection_tracks(&CollectionId::new(collection_id))
            .map_err(|e| e.to_string())?;
        Ok(tracks.iter().filter_map(track_summary).collect())
    }

    /// Add a folder as a collection and sync its tracks.
    pub fn add_folder_collection(
        &self,
        folder_path: String,
    ) -> Result<AddFolderCollectionResult, String> {
        let mut lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        let collection = lib
            .add_collection(&NewCollection::folder(folder_path))
            .map_err(|e| e.to_string())?;
        let scan = lib
            .sync_collection(Some(&collection.id))
            .map_err(|e| e.to_string())?;
        let summary = collection_summary(&lib, collection)?;
        Ok(AddFolderCollectionResult {
            collection: summary,
            added: scan.added as u32,
            updated: scan.updated as u32,
            skipped: scan.skipped as u32,
            failed: scan.failed as u32,
        })
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
                track_summary(&source).map(|track| ResolvedLibraryTrack {
                    request_path,
                    track,
                })
            })
            .collect())
    }

    /// Embedded artwork bytes for a track id and/or file path.
    ///
    /// Returns `Ok(None)` when the file has no artwork (e.g. minimal WAV).
    pub fn get_track_artwork(
        &self,
        track_id: Option<String>,
        path: Option<String>,
    ) -> Result<Option<Vec<u8>>, String> {
        let file_path = if let Some(path) = path {
            path
        } else if let Some(track_id) = track_id {
            let lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            let source = lib
                .get_track(&TrackId::new(track_id))
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Track not found in library.".to_string())?;
            source
                .file()
                .ok_or_else(|| "Only file tracks have artwork.".to_string())?
                .path()
                .to_string_lossy()
                .into_owned()
        } else {
            return Err("track_id or path is required.".to_string());
        };

        read_artwork(Path::new(&file_path)).map_err(|e| e.to_string())
    }

    /// Queue analyze for a track (worker emits `TrackAnalyzed` / `Error`).
    pub fn analyze_track(&self, track_id: String, force: bool) -> Result<(), String> {
        let bytes = encode_cmd_body(&CmdBody::AnalyzeTrack { track_id, force })
            .map_err(|e| e.to_string())?;
        self.buses
            .publish_cmd(Origin::Library, Kind::AnalyzeTrack, bytes)
            .map_err(|e| e.to_string())
    }

    /// Queue metadata refresh for a track (worker emits `TrackUpdated` / `Error`).
    pub fn refresh_track(&self, track_id: String) -> Result<(), String> {
        let bytes =
            encode_cmd_body(&CmdBody::RefreshTrack { track_id }).map_err(|e| e.to_string())?;
        self.buses
            .publish_cmd(Origin::Library, Kind::RefreshTrack, bytes)
            .map_err(|e| e.to_string())
    }

    /// Forward thin typed library events to Dart via FRB `StreamSink`.
    pub fn subscribe_events(&self, sink: StreamSink<LibraryEvt>) -> Result<(), String> {
        let rx = self.buses.subscribe_evt_all().map_err(|e| e.to_string())?;
        let shutdown = Arc::clone(&self.evt_forwarder_shutdown);
        std::thread::Builder::new()
            .name("library-evt-forwarder".into())
            .spawn(move || {
                while !shutdown.load(Ordering::Relaxed) {
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
        Ok(())
    }

    /// Raw evt subscription for host/tests. Prefer [`Self::subscribe_events`] for Dart.
    #[flutter_rust_bridge::frb(ignore)]
    pub fn subscribe_evt_all(&self) -> Result<library::EvtReceiver, String> {
        self.buses.subscribe_evt_all().map_err(|e| e.to_string())
    }

    /// Render a scrolling waveform lane and return Tauri-compatible packed `WFR1` bytes.
    ///
    /// Overview peaks come from the library when present; otherwise an empty overview
    /// yields a valid silent/background frame. Detail windows are skipped this pass
    /// (no AppState audio cache).
    pub fn render_waveform_lane(
        &self,
        request: RenderWaveformLaneRequest,
    ) -> Result<Vec<u8>, String> {
        let viewport_width = request.width.max(1) as usize;
        let height = request.height.max(1) as usize;
        let visible_ms = request.visible_ms.max(100);
        let buffer_ratio = request.buffer_ratio.clamp(0.0, 4.0);
        let position_secs = ms_to_secs(request.position_ms);
        let visible_secs = ms_to_secs(visible_ms);
        let cover_secs = visible_secs * (1.0 + 2.0 * buffer_ratio);
        let strip_width = ((viewport_width as f64) * (cover_secs / visible_secs))
            .round()
            .max(viewport_width as f64) as usize;

        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;

        let file_path = if let Some(path) = request.path.clone() {
            path
        } else if let Some(ref id) = request.track_id {
            let source = lib
                .get_track(&TrackId::new(id.clone()))
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Track not found in library.".to_string())?;
            source
                .file()
                .ok_or_else(|| "Only file tracks have waveforms.".to_string())?
                .path()
                .to_string_lossy()
                .into_owned()
        } else {
            return Err("path or track_id is required for waveform rendering.".to_string());
        };
        let _ = file_path; // resolved for parity / future detail decode

        let beat_grid: Option<BeatGridSnapshot> = if request.include_beat_grid {
            request.track_id.as_ref().and_then(|id| {
                lib.get_track_beat_grid(&TrackId::new(id.clone()))
                    .ok()
                    .flatten()
            })
        } else {
            None
        };

        let (overview, track_duration_ms) = if let Some(ref id) = request.track_id {
            let track_id = TrackId::new(id.clone());
            let duration = lib
                .get_track(&track_id)
                .map_err(|e| e.to_string())?
                .and_then(|source| source.metadata().duration_ms)
                .unwrap_or(0);
            match lib
                .get_track_waveform_overview(&track_id)
                .map_err(|e| e.to_string())?
            {
                Some(overview_row) if !overview_row.peaks.is_empty() && duration > 0 => {
                    (overview_row.peaks, duration)
                }
                _ => (Vec::<SpectralPeak>::new(), duration),
            }
        } else {
            (Vec::<SpectralPeak>::new(), 0)
        };

        // ponytail: no AppState detail cache — overview-only frames this pass.
        let _ = request.include_detail;

        let gains = WaveformDisplayGains::from_eq_db(
            request.eq_low_db,
            request.eq_mid_db,
            request.eq_high_db,
        );
        let track_duration_secs = ms_to_secs(track_duration_ms);
        let rgba = render_scrolling_lane(
            strip_width,
            height,
            &overview,
            None,
            track_duration_secs,
            position_secs,
            cover_secs,
            gains,
            beat_grid.as_ref(),
            request.include_beat_grid,
        );

        let half_cover_ms = secs_to_ms(cover_secs / 2.0);
        Ok(pack_waveform_frame(
            strip_width as u32,
            height as u32,
            request.position_ms,
            request.position_ms - half_cover_ms,
            request.position_ms + half_cover_ms,
            visible_ms,
            rgba,
        ))
    }
}

/// Map omnibus library egress to the thin Dart-facing evt (ignores Navigate/Load/cues/…).
pub(crate) fn map_library_evt(ev: &Evt) -> Option<LibraryEvt> {
    let body = decode_evt_body(ev.payload()).ok()?;
    match body {
        EvtBody::TrackAnalyzed { track } => Some(LibraryEvt {
            kind: LibraryEvtKind::TrackAnalyzed,
            track: Some(api_track_summary(track)),
            message: None,
            track_id: None,
        }),
        EvtBody::TrackUpdated { track } => Some(LibraryEvt {
            kind: LibraryEvtKind::TrackUpdated,
            track: Some(api_track_summary(track)),
            message: None,
            track_id: None,
        }),
        EvtBody::Error { message, track_id } => Some(LibraryEvt {
            kind: LibraryEvtKind::Error,
            track: None,
            message: Some(message),
            track_id,
        }),
        EvtBody::Notice { message } => Some(LibraryEvt {
            kind: LibraryEvtKind::Notice,
            track: None,
            message: Some(message),
            track_id: None,
        }),
        EvtBody::Empty
        | EvtBody::HotCuesChanged { .. }
        | EvtBody::LoopsChanged { .. }
        | EvtBody::Navigate { .. }
        | EvtBody::Load { .. } => None,
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
    }
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

fn track_summary(source: &AudioSource) -> Option<LibraryTrackSummary> {
    let file = source.file()?;
    let metadata = source.metadata();
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
        path: file.path().to_string_lossy().into_owned(),
    })
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
        }
    }

    #[test]
    fn map_library_evt_maps_thin_kinds_and_ignores_navigate() {
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
                Origin::LibraryNavigation,
                Kind::Navigate,
                EvtBody::Navigate { delta: 1 },
            )
            .unwrap();
        let ev = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        assert!(map_library_evt(ev.as_ref()).is_none());
    }
}
