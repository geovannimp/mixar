use audio_core::{BusConfig, BusId, ChannelMapping, DeviceId};
use engine_core::{create_backend, AnalysisDurationMode, AudioConfig, Engine, EngineConfig};
use resampler::normalize_resampler_quality;
use library::{LibraryConfig, LibraryManager, NewCollection, WritableLibrary};
use library_core::{AnalyzeTrackOptions, CollectionId, Library, LibrarySource, TrackId};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

mod audio_cache;

use audio_cache::{get_or_decode, AudioCache};

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
struct DeckInfo {
    track: Option<String>,
    playing: bool,
    volume: f32,
    eq: DeckEq,
}

impl Default for DeckInfo {
    fn default() -> Self {
        Self {
            track: None,
            playing: false,
            volume: 1.0,
            eq: DeckEq::default(),
        }
    }
}

struct AppState {
    library: LibraryManager,
    engine: Option<Engine>,
    engine_config: EngineConfig,
    decks: [DeckInfo; NUM_DECKS],
    audio_cache: AudioCache,
}

const MASTER_BUS_ID: &str = "master";
const PREVIEW_BUS_ID: &str = "cue";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BusRouteSettings {
    device_id: String,
    left_channel: u16,
    right_channel: u16,
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
    }
}

fn default_preview_bus_route() -> BusRouteSettings {
    BusRouteSettings {
        device_id: "default".to_string(),
        left_channel: 3,
        right_channel: 4,
    }
}

fn bus_route_from_config(bus: &BusConfig) -> BusRouteSettings {
    BusRouteSettings {
        device_id: bus.device.as_str().to_string(),
        left_channel: bus.channels.left,
        right_channel: bus.channels.right,
    }
}

fn bus_config(id: &str, name: &str, route: &BusRouteSettings) -> BusConfig {
    BusConfig::new(
        BusId::new(id),
        name.to_string(),
        DeviceId::new(route.device_id.clone()),
        ChannelMapping::new(route.left_channel, route.right_channel),
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
struct DeckStatus {
    id: usize,
    track: Option<String>,
    playing: bool,
    volume: f32,
    eq: DeckEq,
}

#[derive(Debug, Clone, Serialize)]
struct EngineStatus {
    running: bool,
    backend: String,
    sample_rate: u32,
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

fn clamp_eq_db(value: f32) -> f32 {
    value.clamp(-24.0, 24.0)
}

fn deck_eq_gains(eq: &DeckEq) -> (f32, f32, f32) {
    (
        clamp_eq_db(eq.low),
        clamp_eq_db(eq.mid),
        clamp_eq_db(eq.high),
    )
}

fn deck_statuses(state: &AppState) -> Vec<DeckStatus> {
    state
        .decks
        .iter()
        .enumerate()
        .map(|(id, deck)| DeckStatus {
            id,
            track: deck.track.clone(),
            playing: deck.playing,
            volume: deck.volume,
            eq: deck.eq.clone(),
        })
        .collect()
}

fn engine_status(state: &AppState) -> EngineStatus {
    EngineStatus {
        running: state.engine.is_some(),
        backend: "cpal".to_string(),
        sample_rate: 48_000,
        decks: deck_statuses(state),
    }
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
fn start_engine(state: State<'_, SharedAppState>) -> Result<EngineStatus, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    if state.engine.is_some() {
        return Ok(engine_status(&state));
    }

    let config = state.engine_config.clone();
    let mut engine = Engine::new(config).map_err(|e| e.to_string())?;
    engine.start().map_err(|e| e.to_string())?;
    for (deck_id, deck) in state.decks.iter().enumerate() {
        engine
            .set_deck_volume(deck_id, deck.volume)
            .map_err(|e| e.to_string())?;
        let (low, mid, high) = deck_eq_gains(&deck.eq);
        engine
            .set_deck_eq_bands(deck_id, low, mid, high)
            .map_err(|e| e.to_string())?;
    }
    state.engine = Some(engine);

    Ok(engine_status(&state))
}

#[tauri::command]
fn stop_engine(state: State<'_, SharedAppState>) -> Result<EngineStatus, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    if let Some(mut engine) = state.engine.take() {
        engine.stop().map_err(|e| e.to_string())?;
    }
    for deck in &mut state.decks {
        deck.playing = false;
        deck.track = None;
    }
    state.audio_cache.prune();
    Ok(engine_status(&state))
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

#[tauri::command]
fn save_settings(
    settings: AppSettings,
    state: State<'_, SharedAppState>,
) -> Result<AppSettings, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    if state.engine.is_some() {
        return Err("Stop the engine before changing settings.".to_string());
    }
    apply_settings(&mut state, settings);
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
async fn load_track(
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
    with_engine(&mut state, |engine| {
        engine
            .load_track(deck_id, audio)
            .map_err(|e| e.to_string())
    })?;

    state.decks[deck_id].track = Some(path);
    state.decks[deck_id].playing = false;
    Ok(deck_statuses(&state)[deck_id].clone())
}

#[tauri::command]
async fn load_library_track_to_deck(
    deck_id: usize,
    track_id: String,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let (path, cache_key) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let source = state
            .library
            .get_track(&TrackId::new(track_id))
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Track not found in library.".to_string())?;

        let path = source
            .file()
            .ok_or_else(|| "Only file tracks can be loaded to a deck.".to_string())?
            .path()
            .to_string_lossy()
            .into_owned();

        (path.clone(), path)
    };

    let audio = get_or_decode(&state, cache_key, path.clone()).await?;

    let mut state = state.lock().map_err(|e| e.to_string())?;
    with_engine(&mut state, |engine| {
        engine
            .load_track(deck_id, audio)
            .map_err(|e| e.to_string())
    })?;

    state.decks[deck_id].track = Some(path);
    state.decks[deck_id].playing = false;
    Ok(deck_statuses(&state)[deck_id].clone())
}

#[tauri::command]
fn play_deck(deck_id: usize, state: State<'_, SharedAppState>) -> Result<DeckStatus, String> {
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
    Ok(deck_statuses(&state)[deck_id].clone())
}

#[tauri::command]
fn pause_deck(deck_id: usize, state: State<'_, SharedAppState>) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    with_engine(&mut state, |engine| {
        engine.pause(deck_id).map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].playing = false;
    Ok(deck_statuses(&state)[deck_id].clone())
}

#[tauri::command]
fn set_deck_volume(
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
    Ok(deck_statuses(&state)[deck_id].clone())
}

#[tauri::command]
fn set_deck_eq(
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
    Ok(deck_statuses(&state)[deck_id].clone())
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
                audio_cache: AudioCache::new(),
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
            analyze_library_track,
            load_track,
            load_library_track_to_deck,
            play_deck,
            pause_deck,
            set_deck_volume,
            set_deck_eq,
            sample_track_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
