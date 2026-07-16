use audio_core::{BusConfig, BusId, ChannelMapping, ChannelMode, DeviceId};
use engine_core::{create_backend, AnalysisDurationMode, AudioConfig, Engine, EngineConfig};
use resampler::normalize_resampler_quality;
use library::{LibraryConfig, LibraryManager, NewCollection, WritableLibrary};
use library_core::{
    AnalyzeTrackOptions, CollectionId, Library, LibrarySource, SUPPORTED_AUDIO_EXTENSIONS,
    TrackId,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use deck_performance::{
    begin_deck_cue_hold, delete_hot_cue, delete_loop, end_deck_cue_hold, exit_deck_loop,
    apply_deck_performance, fetch_deck_performance, recall_saved_loop, save_hot_cue, save_loop,
    seek_deck, set_deck_auto_loop,
    set_deck_cue_point, set_deck_loop_in, set_deck_loop_out, set_deck_quantize, trigger_hot_cue,
    unload_deck, HotCueStatus, LoopRegionStatus, SavedLoopStatus,
};
use deck_sync::{
    beat_jump_deck, begin_loop_roll, cycle_deck_pad_mode, end_loop_roll, set_deck_filter,
    set_deck_gain_trim, set_deck_pad_mode, set_master_deck, toggle_deck_sync, PadMode, SyncMode,
};
use tauri::{AppHandle, Manager, State};

mod audio_cache;
mod deck_performance;
mod deck_sync;
mod engine_controller;
mod engine_events;
mod engine_notifier;
mod fs_browser;
mod waveform_render;

use audio_cache::{
    get_or_compute_detail, get_or_compute_overview, get_or_decode, AudioCache,
};
use fs_browser::{browse_directory, list_volumes, DirectoryListing, VolumeInfo};
use waveform_render::{render_scrolling_lane, WaveformDisplayGains};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

use engine_controller::{engine_status, publish_deck, publish_status};
use engine_notifier::EngineNotifier;

pub(crate) use engine_controller::{bump_revision, deck_status};

const NUM_DECKS: usize = 2;

type SharedAppState = Arc<Mutex<AppState>>;

#[derive(Debug, Clone, Serialize)]
struct DeckEq {
    low: f32,
    mid: f32,
    high: f32,
}

impl Default for DeckEq {
    fn default() -> Self {
        Self {
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeckInfo {
    track: Option<String>,
    track_id: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    bpm: Option<f64>,
    key: Option<String>,
    playing: bool,
    volume: f32,
    speed: f32,
    eq: DeckEq,
    cue_point_secs: Option<f64>,
    quantize: bool,
    hot_cues: Vec<HotCueStatus>,
    saved_loops: Vec<SavedLoopStatus>,
    active_loop: Option<LoopRegionStatus>,
    filter_db: f32,
    gain_trim_db: f32,
    loudness_lufs: Option<f64>,
    auto_gain_db: f32,
    sync_mode: SyncMode,
    pad_mode: PadMode,
    loop_roll_restore: Option<LoopRegionStatus>,
    headphone_cue: bool,
}

impl Default for DeckInfo {
    fn default() -> Self {
        Self {
            track: None,
            track_id: None,
            title: None,
            artist: None,
            bpm: None,
            key: None,
            playing: false,
            volume: 1.0,
            speed: 1.0,
            eq: DeckEq::default(),
            cue_point_secs: None,
            quantize: true,
            hot_cues: Vec::new(),
            saved_loops: Vec::new(),
            active_loop: None,
            filter_db: 0.0,
            gain_trim_db: 0.0,
            loudness_lufs: None,
            auto_gain_db: 0.0,
            sync_mode: SyncMode::Off,
            pad_mode: PadMode::HotCue,
            loop_roll_restore: None,
            headphone_cue: false,
        }
    }
}

pub(crate) struct AppState {
    pub library: LibraryManager,
    pub engine: Option<Engine>,
    pub engine_config: EngineConfig,
    pub decks: [DeckInfo; NUM_DECKS],
    pub crossfader: f32,
    pub cue_mix: f32,
    pub master_cue: bool,
    pub master_deck: usize,
    pub revision: u64,
    pub audio_cache: AudioCache,
    pub library_table_columns: Vec<String>,
    pub volume_normalizer_enabled: bool,
    pub target_lufs: f32,
    pub notifier: Option<EngineNotifier>,
}

const MASTER_BUS_ID: &str = "master";
const PREVIEW_BUS_ID: &str = "cue";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BusChannelMode {
    Stereo,
    Mono,
}

impl Default for BusChannelMode {
    fn default() -> Self {
        Self::Stereo
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BusRouteSettings {
    device_id: String,
    left_channel: u16,
    right_channel: u16,
    #[serde(default)]
    mode: BusChannelMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppSettings {
    backend: String,
    sample_rate: u32,
    buffer_size: u32,
    low_latency: bool,
    resampler_quality: String,
    master_bus: BusRouteSettings,
    preview_enabled: bool,
    preview_bus: BusRouteSettings,
    analysis_duration: AnalysisDurationMode,
    scan_folder_tree: bool,
    #[serde(default = "default_library_table_columns")]
    library_table_columns: Vec<String>,
    #[serde(default = "default_volume_normalizer_enabled")]
    volume_normalizer_enabled: bool,
    #[serde(default = "default_target_lufs")]
    target_lufs: f32,
}

fn default_volume_normalizer_enabled() -> bool {
    true
}

fn default_target_lufs() -> f32 {
    -18.0
}

fn default_library_table_columns() -> Vec<String> {
    vec![
        "title".to_string(),
        "artist".to_string(),
        "bpm".to_string(),
        "key".to_string(),
        "duration".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize)]
struct DeviceSummary {
    id: String,
    name: String,
    is_default: bool,
}

fn default_master_bus_route() -> BusRouteSettings {
    BusRouteSettings {
        device_id: "default".to_string(),
        left_channel: 1,
        right_channel: 2,
        mode: BusChannelMode::Stereo,
    }
}

fn default_preview_bus_route() -> BusRouteSettings {
    BusRouteSettings {
        device_id: "default".to_string(),
        left_channel: 3,
        right_channel: 4,
        mode: BusChannelMode::Stereo,
    }
}

fn bus_route_from_config(bus: &BusConfig) -> BusRouteSettings {
    BusRouteSettings {
        device_id: bus.device.as_str().to_string(),
        left_channel: bus.channels.left,
        right_channel: bus.channels.right,
        mode: match bus.channels.mode {
            ChannelMode::Mono => BusChannelMode::Mono,
            ChannelMode::Stereo => BusChannelMode::Stereo,
        },
    }
}

fn channel_mapping_from_route(route: &BusRouteSettings) -> ChannelMapping {
    match route.mode {
        BusChannelMode::Mono => ChannelMapping::mono(route.left_channel),
        BusChannelMode::Stereo => {
            ChannelMapping::stereo(route.left_channel, route.right_channel)
        }
    }
}

fn bus_config(id: &str, name: &str, route: &BusRouteSettings) -> BusConfig {
    BusConfig::new(
        BusId::new(id),
        name.to_string(),
        DeviceId::new(route.device_id.clone()),
        channel_mapping_from_route(route),
    )
}

fn buses_from_settings(settings: &AppSettings) -> Vec<BusConfig> {
    let mut buses = vec![bus_config(MASTER_BUS_ID, "Master", &settings.master_bus)];
    if settings.preview_enabled {
        buses.push(bus_config(
            PREVIEW_BUS_ID,
            "Preview",
            &settings.preview_bus,
        ));
    }
    buses
}

fn settings_from_state(state: &AppState) -> AppSettings {
    let config = &state.engine_config;
    let audio = config.audio.as_ref();
    let master_bus = config
        .buses
        .iter()
        .find(|bus| bus.id.as_str() == MASTER_BUS_ID)
        .map(bus_route_from_config)
        .unwrap_or_else(default_master_bus_route);
    let preview_bus_config = config
        .buses
        .iter()
        .find(|bus| bus.id.as_str() == PREVIEW_BUS_ID);
    AppSettings {
        backend: config.backend.clone(),
        sample_rate: config.sample_rate,
        buffer_size: config.buffer_size,
        low_latency: config.low_latency,
        resampler_quality: normalize_resampler_quality(
            audio.and_then(|a| a.resampler_quality.as_deref()),
        )
        .to_string(),
        master_bus,
        preview_enabled: preview_bus_config.is_some(),
        preview_bus: preview_bus_config
            .map(bus_route_from_config)
            .unwrap_or_else(default_preview_bus_route),
        analysis_duration: config.analysis_duration,
        scan_folder_tree: state.library.config().scan_folder_tree,
        library_table_columns: state.library_table_columns.clone(),
        volume_normalizer_enabled: state.volume_normalizer_enabled,
        target_lufs: state.target_lufs,
    }
}

fn apply_settings(state: &mut AppState, settings: AppSettings) {
    let config = &mut state.engine_config;
    config.buses = buses_from_settings(&settings);
    config.backend = settings.backend;
    config.sample_rate = settings.sample_rate;
    config.buffer_size = settings.buffer_size;
    config.low_latency = settings.low_latency;
    config.analysis_duration = settings.analysis_duration;

    config.audio = Some(AudioConfig {
        resampler_quality: Some(settings.resampler_quality.clone()),
    });

    state.library.set_config(LibraryConfig {
        scan_folder_tree: settings.scan_folder_tree,
    });
    state.library_table_columns = if settings.library_table_columns.is_empty() {
        default_library_table_columns()
    } else {
        settings.library_table_columns
    };
    state.volume_normalizer_enabled = settings.volume_normalizer_enabled;
    state.target_lufs = settings.target_lufs;
}

fn deck_auto_gain_db(enabled: bool, target_lufs: f32, loudness_lufs: Option<f64>) -> f32 {
    match (enabled, loudness_lufs) {
        (true, Some(loudness_lufs)) => analyzer_core::auto_gain_db(target_lufs, loudness_lufs),
        _ => 0.0,
    }
}

fn default_engine_config() -> EngineConfig {
    let mut config = EngineConfig::default();
    config.backend = "cpal".to_string();
    config.audio = Some(AudioConfig {
        resampler_quality: Some("medium".to_string()),
    });
    config.buses = vec![bus_config(
        MASTER_BUS_ID,
        "Master",
        &default_master_bus_route(),
    )];
    config
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeckStatus {
    id: usize,
    track: Option<String>,
    track_id: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    bpm: Option<f64>,
    key: Option<String>,
    playing: bool,
    volume: f32,
    speed: f32,
    eq: DeckEq,
    position_secs: Option<f64>,
    duration_secs: Option<f64>,
    cue_point_secs: Option<f64>,
    quantize: bool,
    hot_cues: Vec<HotCueStatus>,
    saved_loops: Vec<SavedLoopStatus>,
    active_loop: Option<LoopRegionStatus>,
    filter_db: f32,
    gain_trim_db: f32,
    loudness_lufs: Option<f64>,
    auto_gain_db: f32,
    sync_mode: SyncMode,
    is_master: bool,
    pad_mode: PadMode,
    headphone_cue: bool,
}

#[derive(Debug, Clone, Serialize)]
struct WaveformFrame {
    /// Full strip pixel width (may be wider than the viewport).
    width: u32,
    height: u32,
    /// Base64-encoded RGBA bytes (width * height * 4).
    rgba_base64: String,
    /// Playhead time the strip was centered on when rendered.
    center_secs: f64,
    /// Absolute timeline start covered by the strip.
    cover_start_secs: f64,
    /// Absolute timeline end covered by the strip.
    cover_end_secs: f64,
    /// Seconds shown in the viewport (center playhead window).
    visible_secs: f64,
}

#[derive(Debug, Clone, Serialize)]
struct EngineStatus {
    running: bool,
    backend: String,
    sample_rate: u32,
    crossfader: f32,
    cue_mix: f32,
    master_cue: bool,
    decks: Vec<DeckStatus>,
}

#[derive(Debug, Clone, Serialize)]
struct CollectionSummary {
    id: String,
    name: String,
    kind: String,
    path: Option<String>,
    track_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct TrackSummary {
    id: String,
    display_name: String,
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
    genre: Option<String>,
    bpm: Option<f64>,
    key: Option<String>,
    duration_secs: Option<f64>,
    path: String,
}

#[derive(Debug, Clone, Serialize)]
struct AddFolderCollectionResult {
    collection: CollectionSummary,
    scan: library_core::ScanReport,
}

pub(crate) fn clamp_eq_db(value: f32) -> f32 {
    value.clamp(-24.0, 24.0)
}

fn deck_playback_secs(state: &AppState, deck_id: usize) -> (Option<f64>, Option<f64>) {
    let Some(engine) = state.engine.as_ref() else {
        return (None, None);
    };
    match engine.deck_playback_secs(deck_id) {
        Some((position, duration)) => (Some(position), Some(duration)),
        None => (None, None),
    }
}

fn apply_path_metadata(deck: &mut DeckInfo, path: &str) {
    deck.title = Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    deck.artist = None;
    deck.bpm = None;
    deck.key = None;
}

fn clear_deck_info(deck: &mut DeckInfo) {
    *deck = DeckInfo {
        volume: deck.volume,
        eq: deck.eq.clone(),
        filter_db: deck.filter_db,
        gain_trim_db: deck.gain_trim_db,
        pad_mode: deck.pad_mode,
        headphone_cue: deck.headphone_cue,
        ..DeckInfo::default()
    };
}

fn track_display_name(source: &LibrarySource) -> String {
    let metadata = source.metadata();
    match (&metadata.artist, &metadata.title) {
        (Some(artist), Some(title)) => format!("{artist} — {title}"),
        (_, Some(title)) => title.clone(),
        _ => source
            .file()
            .and_then(|file| file.path().file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| source.id().as_str().to_string()),
    }
}

fn track_summary(source: &LibrarySource) -> Option<TrackSummary> {
    let file = source.file()?;
    let metadata = source.metadata();
    Some(TrackSummary {
        id: source.id().as_str().to_string(),
        display_name: track_display_name(source),
        artist: metadata.artist.clone(),
        title: metadata.title.clone(),
        album: metadata.album.clone(),
        genre: metadata.genre.clone(),
        bpm: metadata.bpm,
        key: metadata.key.clone(),
        duration_secs: metadata.duration_secs,
        path: file.path().to_string_lossy().into_owned(),
    })
}

fn collection_summary(
    library: &LibraryManager,
    collection: library_core::Collection,
) -> Result<CollectionSummary, String> {
    let track_count = library
        .get_collection_tracks(&collection.id)
        .map_err(|e| e.to_string())?
        .len();

    Ok(CollectionSummary {
        id: collection.id.as_str().to_string(),
        name: collection.name.clone(),
        kind: collection.collection_type().as_str().to_string(),
        path: collection
            .fs_path()
            .map(|path| path.to_string_lossy().into_owned()),
        track_count,
    })
}

fn with_engine<F, T>(state: &mut AppState, f: F) -> Result<T, String>
where
    F: FnOnce(&mut Engine) -> Result<T, String>,
{
    let engine = state
        .engine
        .as_mut()
        .ok_or_else(|| "Engine is not running. Start it first.".to_string())?;
    f(engine)
}

#[tauri::command]
fn start_engine(
    app: AppHandle,
    shared: State<'_, SharedAppState>,
) -> Result<EngineStatus, String> {
    let shared_state = shared.inner().clone();
    let mut state = shared.lock().map_err(|e| e.to_string())?;
    if state.engine.is_some() {
        return Ok(engine_status(&state));
    }

    let config = state.engine_config.clone();
    let mut engine = Engine::new(config).map_err(|e| e.to_string())?;
    engine.start().map_err(|e| e.to_string())?;
    state.engine = Some(engine);
    state.notifier = Some(EngineNotifier::start(app.clone(), shared_state));

    Ok(publish_status(&app, &mut state))
}

#[tauri::command]
fn stop_engine(
    app: AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<EngineStatus, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.notifier = None;
    if let Some(mut engine) = state.engine.take() {
        engine.stop().map_err(|e| e.to_string())?;
    }
    for deck in &mut state.decks {
        clear_deck_info(deck);
    }
    state.audio_cache.prune();
    Ok(publish_status(&app, &mut state))
}

#[tauri::command]
fn get_status(state: State<'_, SharedAppState>) -> Result<EngineStatus, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(engine_status(&state))
}

#[tauri::command]
fn get_settings(state: State<'_, SharedAppState>) -> Result<AppSettings, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(settings_from_state(&state))
}

/// Applies new settings. If the engine is running, it is stopped, the new
/// config is applied, and the engine is restarted with the same decks/tracks
/// reloaded — the caller never has to stop the engine first.
#[tauri::command]
async fn save_settings(
    app: AppHandle,
    settings: AppSettings,
    shared: State<'_, SharedAppState>,
) -> Result<AppSettings, String> {
    let shared_state = shared.inner().clone();

    let (was_running, deck_tracks) = {
        let mut state = shared.lock().map_err(|e| e.to_string())?;
        let was_running = state.engine.is_some();
        let deck_tracks: Vec<(usize, String)> = state
            .decks
            .iter()
            .enumerate()
            .filter_map(|(deck_id, deck)| deck.track.clone().map(|path| (deck_id, path)))
            .collect();

        if was_running {
            state.notifier = None;
            if let Some(mut engine) = state.engine.take() {
                engine.stop().map_err(|e| e.to_string())?;
            }
            // Do NOT clear_deck_info here — keep deck/track/hot-cue UI state intact
            // across the restart; only the engine (and its loaded audio) is torn down.
        }

        apply_settings(&mut state, settings);
        (was_running, deck_tracks)
    };

    if !was_running {
        let state = shared.lock().map_err(|e| e.to_string())?;
        return Ok(settings_from_state(&state));
    }

    let mut reloaded_tracks = Vec::with_capacity(deck_tracks.len());
    for (deck_id, path) in deck_tracks {
        let audio = get_or_decode(&shared, path.clone(), path).await?;
        reloaded_tracks.push((deck_id, audio));
    }

    let mut state = shared.lock().map_err(|e| e.to_string())?;
    let config = state.engine_config.clone();
    let mut engine = Engine::new(config).map_err(|e| e.to_string())?;
    engine.start().map_err(|e| e.to_string())?;
    state.engine = Some(engine);
    for (deck_id, audio) in reloaded_tracks {
        // Preserve auto gain from the original load; settings changes apply on next track load.
        let auto_gain_db = state.decks[deck_id].auto_gain_db;
        with_engine(&mut state, |engine| {
            engine
                .load_track(deck_id, audio, auto_gain_db)
                .map_err(|e| e.to_string())
        })?;
        // A fresh engine load starts paused; reflect that in deck state.
        state.decks[deck_id].playing = false;
    }
    state.notifier = Some(EngineNotifier::start(app.clone(), shared_state));
    let _ = publish_status(&app, &mut state);

    Ok(settings_from_state(&state))
}

#[tauri::command]
fn list_output_devices(backend: String) -> Result<Vec<DeviceSummary>, String> {
    let devices = create_backend(&backend)
        .map_err(|e| e.to_string())?
        .list_output_devices()
        .map_err(|e| e.to_string())?;

    let mut summaries: Vec<DeviceSummary> = vec![DeviceSummary {
        id: "default".to_string(),
        name: "System default".to_string(),
        is_default: true,
    }];

    for device in devices {
        if device.id.as_str() == "default" {
            continue;
        }
        summaries.push(DeviceSummary {
            id: device.id.as_str().to_string(),
            name: device.name,
            is_default: device.is_default,
        });
    }

    Ok(summaries)
}

#[tauri::command]
fn list_collections(state: State<'_, SharedAppState>) -> Result<Vec<CollectionSummary>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let collections = state.library.list_collections().map_err(|e| e.to_string())?;
    collections
        .into_iter()
        .map(|collection| collection_summary(&state.library, collection))
        .collect()
}

#[tauri::command]
fn add_folder_collection(
    folder_path: String,
    state: State<'_, SharedAppState>,
) -> Result<AddFolderCollectionResult, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let collection = state
        .library
        .add_collection(&NewCollection::folder(folder_path))
        .map_err(|e| e.to_string())?;
    let scan = state
        .library
        .sync_collection(Some(&collection.id))
        .map_err(|e| e.to_string())?;
    let summary = collection_summary(&state.library, collection)?;

    Ok(AddFolderCollectionResult {
        collection: summary,
        scan,
    })
}

#[tauri::command]
fn list_collection_tracks(
    collection_id: String,
    state: State<'_, SharedAppState>,
) -> Result<Vec<TrackSummary>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let tracks = state
        .library
        .get_collection_tracks(&CollectionId::new(collection_id))
        .map_err(|e| e.to_string())?;

    Ok(tracks.iter().filter_map(track_summary).collect())
}

#[derive(Debug, Clone, Serialize)]
struct ResolvedLibraryTrack {
    request_path: String,
    track: TrackSummary,
}

#[tauri::command]
async fn resolve_library_tracks_for_paths(
    paths: Vec<String>,
    state: State<'_, SharedAppState>,
) -> Result<Vec<ResolvedLibraryTrack>, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let guard = state.lock().map_err(|e| e.to_string())?;
        let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let resolved = guard
            .library
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
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn analyze_library_track(
    track_id: String,
    state: State<'_, SharedAppState>,
) -> Result<TrackSummary, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = state.lock().map_err(|e| e.to_string())?;
        let options = AnalyzeTrackOptions {
            force: false,
            analysis_duration: guard.engine_config.analysis_duration,
        };
        let source = guard
            .library
            .analyze_track(&TrackId::new(track_id), options)
            .map_err(|e| e.to_string())?;
        track_summary(&source).ok_or_else(|| "Only file tracks can be analyzed.".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn list_fs_volumes() -> Result<Vec<VolumeInfo>, String> {
    list_volumes()
}

#[tauri::command]
fn browse_fs_directory(path: String) -> Result<DirectoryListing, String> {
    browse_directory(&path)
}

#[tauri::command]
async fn load_path_to_deck(
    app: AppHandle,
    deck_id: usize,
    path: String,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let (track_id, cache_key, title, artist, bpm, key, loudness_lufs) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let source = state
            .library
            .import_file_path(Path::new(&path))
            .map_err(|e| e.to_string())?;
        let track_id = source.id().as_str().to_string();
        let loudness_lufs = state
            .library
            .track_loudness_lufs(&TrackId::new(track_id.clone()))
            .map_err(|e| e.to_string())?;

        let file_path = source
            .file()
            .ok_or_else(|| "Only file tracks can be loaded to a deck.".to_string())?
            .path()
            .to_string_lossy()
            .into_owned();

        let metadata = source.metadata();
        (
            track_id,
            file_path,
            metadata.title.clone(),
            metadata.artist.clone(),
            metadata.bpm,
            metadata.key.clone(),
            loudness_lufs,
        )
    };

    {
        let state = state.lock().map_err(|e| e.to_string())?;
        state
            .library
            .ensure_track_waveform(&TrackId::new(track_id.clone()))
            .map_err(|e| e.to_string())?;
    }

    let audio = get_or_decode(&state, cache_key.clone(), cache_key).await?;

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let auto_gain_db = deck_auto_gain_db(
        state.volume_normalizer_enabled,
        state.target_lufs,
        loudness_lufs,
    );
    with_engine(&mut state, |engine| {
        engine
            .load_track(deck_id, audio, auto_gain_db)
            .map_err(|e| e.to_string())
    })?;

    {
        let deck = &mut state.decks[deck_id];
        deck.track = Some(path);
        deck.track_id = Some(track_id.clone());
        deck.playing = false;
        deck.speed = 1.0;
        deck.title = title;
        deck.artist = artist;
        deck.bpm = bpm;
        deck.key = key;
        deck.loudness_lufs = loudness_lufs;
        deck.auto_gain_db = auto_gain_db;
    }
    let track_id_for_perf = state.decks[deck_id].track_id.clone();
    let (hot_cues, saved_loops) =
        fetch_deck_performance(&state.library, track_id_for_perf.as_deref());
    apply_deck_performance(&mut state.decks[deck_id], hot_cues, saved_loops, true);
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
async fn load_track(
    app: AppHandle,
    deck_id: usize,
    path: String,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let cache_key = path.clone();
    let audio = get_or_decode(&state, cache_key, path.clone()).await?;

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let auto_gain_db = deck_auto_gain_db(
        state.volume_normalizer_enabled,
        state.target_lufs,
        None,
    );
    with_engine(&mut state, |engine| {
        engine
            .load_track(deck_id, audio, auto_gain_db)
            .map_err(|e| e.to_string())
    })?;

    {
        let deck = &mut state.decks[deck_id];
        deck.track = Some(path.clone());
        deck.track_id = None;
        deck.playing = false;
        deck.speed = 1.0;
        deck.loudness_lufs = None;
        deck.auto_gain_db = auto_gain_db;
        apply_path_metadata(deck, &path);
    }
    let (hot_cues, saved_loops) = fetch_deck_performance(&state.library, None);
    apply_deck_performance(&mut state.decks[deck_id], hot_cues, saved_loops, true);
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
async fn load_library_track_to_deck(
    app: AppHandle,
    deck_id: usize,
    track_id: String,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let (path, cache_key, title, artist, bpm, key, loudness_lufs) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let track_id = TrackId::new(track_id.clone());
        let source = state
            .library
            .get_track(&track_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Track not found in library.".to_string())?;
        let loudness_lufs = state
            .library
            .track_loudness_lufs(&track_id)
            .map_err(|e| e.to_string())?;

        let path = source
            .file()
            .ok_or_else(|| "Only file tracks can be loaded to a deck.".to_string())?
            .path()
            .to_string_lossy()
            .into_owned();

        let metadata = source.metadata();
        (
            path.clone(),
            path,
            metadata.title.clone(),
            metadata.artist.clone(),
            metadata.bpm,
            metadata.key.clone(),
            loudness_lufs,
        )
    };

    {
        let state = state.lock().map_err(|e| e.to_string())?;
        state
            .library
            .ensure_track_waveform(&TrackId::new(track_id.clone()))
            .map_err(|e| e.to_string())?;
    }

    let audio = get_or_decode(&state, cache_key, path.clone()).await?;

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let auto_gain_db = deck_auto_gain_db(
        state.volume_normalizer_enabled,
        state.target_lufs,
        loudness_lufs,
    );
    with_engine(&mut state, |engine| {
        engine
            .load_track(deck_id, audio, auto_gain_db)
            .map_err(|e| e.to_string())
    })?;

    {
        let deck = &mut state.decks[deck_id];
        deck.track = Some(path);
        deck.track_id = Some(track_id.clone());
        deck.playing = false;
        deck.speed = 1.0;
        deck.title = title;
        deck.artist = artist;
        deck.bpm = bpm;
        deck.key = key;
        deck.loudness_lufs = loudness_lufs;
        deck.auto_gain_db = auto_gain_db;
    }
    let track_id_for_perf = state.decks[deck_id].track_id.clone();
    let (hot_cues, saved_loops) =
        fetch_deck_performance(&state.library, track_id_for_perf.as_deref());
    apply_deck_performance(&mut state.decks[deck_id], hot_cues, saved_loops, true);
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
fn play_deck(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    if state.decks[deck_id].track.is_none() {
        return Err("Load a track before playing.".to_string());
    }

    with_engine(&mut state, |engine| {
        engine.play(deck_id).map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].playing = true;
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
fn pause_deck(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    with_engine(&mut state, |engine| {
        engine.pause(deck_id).map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].playing = false;
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
fn set_deck_volume(
    app: AppHandle,
    deck_id: usize,
    volume: f32,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    if !(0.0..=1.0).contains(&volume) {
        return Err("Volume must be between 0 and 1.".to_string());
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.decks[deck_id].volume = volume;
    if let Some(engine) = state.engine.as_mut() {
        engine
            .set_deck_volume(deck_id, volume)
            .map_err(|e| e.to_string())?;
    }
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
fn set_deck_eq(
    app: AppHandle,
    deck_id: usize,
    low: f32,
    mid: f32,
    high: f32,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let eq = DeckEq {
        low: clamp_eq_db(low),
        mid: clamp_eq_db(mid),
        high: clamp_eq_db(high),
    };
    if let Some(engine) = state.engine.as_mut() {
        engine
            .set_deck_eq_bands(deck_id, eq.low, eq.mid, eq.high)
            .map_err(|e| e.to_string())?;
    }
    state.decks[deck_id].eq = eq;
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
fn set_deck_headphone_cue(
    app: AppHandle,
    deck_id: usize,
    enabled: bool,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.decks[deck_id].headphone_cue = enabled;
    if let Some(engine) = state.engine.as_mut() {
        engine
            .set_deck_headphone_cue(deck_id, enabled)
            .map_err(|e| e.to_string())?;
    }
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
fn set_crossfader(
    app: AppHandle,
    crossfader: f32,
    state: State<'_, SharedAppState>,
) -> Result<EngineStatus, String> {
    if !(0.0..=1.0).contains(&crossfader) {
        return Err("Crossfader must be between 0 and 1.".to_string());
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.crossfader = crossfader;
    if let Some(engine) = state.engine.as_mut() {
        engine
            .set_crossfader(crossfader)
            .map_err(|e| e.to_string())?;
    }
    Ok(publish_status(&app, &mut state))
}

#[tauri::command]
fn set_cue_mix(
    app: AppHandle,
    cue_mix: f32,
    state: State<'_, SharedAppState>,
) -> Result<EngineStatus, String> {
    if !(0.0..=1.0).contains(&cue_mix) {
        return Err("Cue mix must be between 0.0 and 1.0".to_string());
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.cue_mix = cue_mix;
    if let Some(engine) = state.engine.as_mut() {
        engine.set_cue_mix(cue_mix).map_err(|e| e.to_string())?;
    }
    Ok(publish_status(&app, &mut state))
}

#[tauri::command]
fn set_master_cue(
    app: AppHandle,
    enabled: bool,
    state: State<'_, SharedAppState>,
) -> Result<EngineStatus, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.master_cue = enabled;
    if let Some(engine) = state.engine.as_mut() {
        engine.set_master_cue(enabled).map_err(|e| e.to_string())?;
    }
    Ok(publish_status(&app, &mut state))
}

#[tauri::command]
fn set_deck_speed(
    app: AppHandle,
    deck_id: usize,
    speed: f32,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    if speed <= 0.0 || speed > 2.0 {
        return Err("Speed must be between 0 and 2.".to_string());
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.decks[deck_id].speed = speed;
    if let Some(engine) = state.engine.as_mut() {
        engine
            .set_deck_speed(deck_id, speed)
            .map_err(|e| e.to_string())?;
    }

    let mut synced_slaves = Vec::new();
    if deck_id == state.master_deck {
        for slave_id in 0..NUM_DECKS {
            if slave_id == deck_id {
                continue;
            }
            if state.decks[slave_id].sync_mode == SyncMode::Off {
                continue;
            }
            // Follow master tempo only — do not re-seek phase on every pitch nudge.
            deck_sync::apply_tempo_sync_for_state(&mut state, slave_id, deck_id)?;
            synced_slaves.push(slave_id);
        }
    }

    let master_status = publish_deck(&app, &mut state, deck_id);
    for slave_id in synced_slaves {
        let _ = publish_deck(&app, &mut state, slave_id);
    }
    Ok(master_status)
}

#[tauri::command]
async fn render_waveform_lane(
    track_id: Option<String>,
    path: Option<String>,
    width: u32,
    height: u32,
    position_secs: f64,
    visible_secs: f64,
    buffer_ratio: f64,
    include_detail: bool,
    include_beat_grid: bool,
    eq_low_db: f32,
    eq_mid_db: f32,
    eq_high_db: f32,
    state: State<'_, SharedAppState>,
) -> Result<WaveformFrame, String> {
    let viewport_width = width.max(1) as usize;
    let height = height.max(1) as usize;
    let visible_secs = visible_secs.max(0.1);
    let buffer_ratio = buffer_ratio.clamp(0.0, 4.0);
    // Wider strip so the client can pan smoothly between IPC refreshes.
    let cover_secs = visible_secs * (1.0 + 2.0 * buffer_ratio);
    let strip_width = ((viewport_width as f64) * (cover_secs / visible_secs))
        .round()
        .max(viewport_width as f64) as usize;

    let file_path = if let Some(path) = path {
        path
    } else if let Some(ref id) = track_id {
        let state = state.lock().map_err(|e| e.to_string())?;
        let source = state
            .library
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

    let cache_key = file_path.clone();
    let (overview, track_duration_secs, beat_grid) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let beat_grid = track_id.as_ref().and_then(|id| {
            state
                .library
                .get_track_beat_grid(&TrackId::new(id.clone()))
                .ok()
                .flatten()
        });

        if let Some(ref id) = track_id {
            if let Some(overview_row) = state
                .library
                .get_track_waveform_overview(&TrackId::new(id.clone()))
                .map_err(|e| e.to_string())?
            {
                let duration = state
                    .library
                    .get_track(&TrackId::new(id.clone()))
                    .map_err(|e| e.to_string())?
                    .and_then(|source| source.metadata().duration_secs)
                    .unwrap_or(0.0);
                if !overview_row.peaks.is_empty() && duration > 0.0 {
                    (Some(overview_row.peaks), duration, beat_grid)
                } else {
                    (None, 0.0, beat_grid)
                }
            } else {
                (None, 0.0, beat_grid)
            }
        } else {
            (None, 0.0, beat_grid)
        }
    };

    let (overview, track_duration_secs) = if let Some(peaks) = overview {
        (peaks, track_duration_secs)
    } else {
        get_or_compute_overview(&state, cache_key.clone(), file_path.clone()).await?
    };

    let detail = if include_detail {
        get_or_compute_detail(
            &state,
            cache_key,
            file_path,
            position_secs,
            cover_secs,
            0.0,
            strip_width,
        )
        .await?
    } else {
        None
    };

    let gains = WaveformDisplayGains::from_eq_db(eq_low_db, eq_mid_db, eq_high_db);
    let beat_grid_for_render = if include_beat_grid {
        beat_grid.as_ref()
    } else {
        None
    };
    let rgba = render_scrolling_lane(
        strip_width,
        height,
        &overview,
        detail.as_ref(),
        track_duration_secs,
        position_secs,
        cover_secs,
        gains,
        beat_grid_for_render,
        include_beat_grid,
    );

    let half_cover = cover_secs / 2.0;
    Ok(WaveformFrame {
        width: strip_width as u32,
        height: height as u32,
        rgba_base64: BASE64.encode(rgba),
        center_secs: position_secs,
        cover_start_secs: position_secs - half_cover,
        cover_end_secs: position_secs + half_cover,
        visible_secs,
    })
}

#[tauri::command]
fn get_track_artwork(
    track_id: Option<String>,
    path: Option<String>,
    state: State<'_, SharedAppState>,
) -> Result<Option<String>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let bytes = if let Some(track_id) = track_id {
        state
            .library
            .get_track_artwork(&TrackId::new(track_id))
            .map_err(|e| e.to_string())?
    } else if let Some(path) = path {
        library::read_artwork(std::path::Path::new(&path)).map_err(|e| e.to_string())?
    } else {
        return Err("track_id or path is required.".to_string());
    };

    Ok(bytes.map(|data| BASE64.encode(data)))
}

#[tauri::command]
fn get_supported_audio_extensions() -> Vec<&'static str> {
    SUPPORTED_AUDIO_EXTENSIONS.to_vec()
}

#[tauri::command]
fn sample_track_path() -> Option<String> {
    let candidates = [
        "../../samples/Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav",
        "../samples/Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav",
        "samples/Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav",
    ];

    candidates
        .iter()
        .find(|path| Path::new(path).exists())
        .map(|path| path.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app
                .path()
                .app_data_dir()
                .map_err(|err| format!("app data dir unavailable: {err}"))?;
            std::fs::create_dir_all(&app_data).map_err(|err| err.to_string())?;

            let library = LibraryManager::open(app_data.join("library.db"), LibraryConfig::default())
                .map_err(|err| err.to_string())?;

            app.manage(Arc::new(Mutex::new(AppState {
                library,
                engine: None,
                engine_config: default_engine_config(),
                decks: Default::default(),
                crossfader: 0.5,
                cue_mix: 0.0,
                master_cue: false,
                master_deck: 0,
                revision: 0,
                audio_cache: AudioCache::new(),
                library_table_columns: default_library_table_columns(),
                volume_normalizer_enabled: default_volume_normalizer_enabled(),
                target_lufs: default_target_lufs(),
                notifier: None,
            })));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_engine,
            stop_engine,
            get_status,
            get_settings,
            save_settings,
            list_output_devices,
            list_collections,
            add_folder_collection,
            list_collection_tracks,
            resolve_library_tracks_for_paths,
            list_fs_volumes,
            browse_fs_directory,
            analyze_library_track,
            load_track,
            load_path_to_deck,
            load_library_track_to_deck,
            play_deck,
            pause_deck,
            set_deck_volume,
            set_deck_eq,
            set_deck_speed,
            set_deck_headphone_cue,
            set_crossfader,
            set_cue_mix,
            set_master_cue,
            seek_deck,
            unload_deck,
            set_deck_cue_point,
            begin_deck_cue_hold,
            end_deck_cue_hold,
            set_deck_quantize,
            set_deck_auto_loop,
            set_deck_loop_in,
            set_deck_loop_out,
            exit_deck_loop,
            trigger_hot_cue,
            save_hot_cue,
            delete_hot_cue,
            save_loop,
            recall_saved_loop,
            delete_loop,
            render_waveform_lane,
            get_supported_audio_extensions,
            sample_track_path,
            set_master_deck,
            toggle_deck_sync,
            beat_jump_deck,
            set_deck_pad_mode,
            cycle_deck_pad_mode,
            set_deck_filter,
            set_deck_gain_trim,
            begin_loop_roll,
            end_loop_roll,
            get_track_artwork,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{deck_auto_gain_db, AppSettings};

    #[test]
    fn legacy_settings_default_volume_normalizer_values() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "backend": "cpal",
            "sample_rate": 48_000,
            "buffer_size": 512,
            "low_latency": false,
            "resampler_quality": "medium",
            "master_bus": {
                "device_id": "default",
                "left_channel": 1,
                "right_channel": 2
            },
            "preview_enabled": false,
            "preview_bus": {
                "device_id": "default",
                "left_channel": 3,
                "right_channel": 4
            },
            "analysis_duration": "fast",
            "scan_folder_tree": true
        }))
        .expect("legacy settings should deserialize");

        assert!(settings.volume_normalizer_enabled);
        assert_eq!(settings.target_lufs, -18.0);
    }

    #[test]
    fn deck_auto_gain_uses_loudness_only_when_enabled() {
        assert_eq!(deck_auto_gain_db(true, -18.0, Some(-24.0)), 6.0);
        assert_eq!(deck_auto_gain_db(false, -18.0, Some(-24.0)), 0.0);
        assert_eq!(deck_auto_gain_db(true, -18.0, None), 0.0);
    }
}
