use engine_core::{Engine, EngineConfig};
use library::{LibraryConfig, LibraryManager, NewCollection, WritableLibrary};
use library_core::{CollectionId, FileAudioSource, Library, LibrarySource, TrackId};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use tauri::{Manager, State};

const NUM_DECKS: usize = 2;

#[derive(Debug, Clone, Default, Serialize)]
struct DeckInfo {
    track: Option<String>,
    playing: bool,
}

struct AppState {
    library: LibraryManager,
    engine: Option<Engine>,
    decks: [DeckInfo; NUM_DECKS],
}

#[derive(Debug, Clone, Serialize)]
struct DeckStatus {
    id: usize,
    track: Option<String>,
    playing: bool,
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
    path: String,
}

#[derive(Debug, Clone, Serialize)]
struct AddFolderCollectionResult {
    collection: CollectionSummary,
    scan: library_core::ScanReport,
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
    Some(TrackSummary {
        id: source.id().as_str().to_string(),
        display_name: track_display_name(source),
        artist: source.metadata().artist.clone(),
        title: source.metadata().title.clone(),
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
fn start_engine(state: State<'_, Mutex<AppState>>) -> Result<EngineStatus, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    if state.engine.is_some() {
        return Ok(engine_status(&state));
    }

    let mut config = EngineConfig::default();
    config.backend = "cpal".to_string();

    let mut engine = Engine::new(config).map_err(|e| e.to_string())?;
    engine.start().map_err(|e| e.to_string())?;
    state.engine = Some(engine);

    Ok(engine_status(&state))
}

#[tauri::command]
fn stop_engine(state: State<'_, Mutex<AppState>>) -> Result<EngineStatus, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    if let Some(mut engine) = state.engine.take() {
        engine.stop().map_err(|e| e.to_string())?;
    }
    for deck in &mut state.decks {
        deck.playing = false;
    }
    Ok(engine_status(&state))
}

#[tauri::command]
fn get_status(state: State<'_, Mutex<AppState>>) -> Result<EngineStatus, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(engine_status(&state))
}

#[tauri::command]
fn list_collections(state: State<'_, Mutex<AppState>>) -> Result<Vec<CollectionSummary>, String> {
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
    state: State<'_, Mutex<AppState>>,
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
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<TrackSummary>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let tracks = state
        .library
        .get_collection_tracks(&CollectionId::new(collection_id))
        .map_err(|e| e.to_string())?;

    Ok(tracks.iter().filter_map(track_summary).collect())
}

#[tauri::command]
fn load_track(
    deck_id: usize,
    path: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    if !Path::new(&path).exists() {
        return Err(format!("File not found: {path}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let source = FileAudioSource::from_path(&path);
    with_engine(&mut state, |engine| {
        engine.load_track(deck_id, &source).map_err(|e| e.to_string())
    })?;

    state.decks[deck_id].track = Some(path);
    state.decks[deck_id].playing = false;
    Ok(deck_statuses(&state)[deck_id].clone())
}

#[tauri::command]
fn load_library_track_to_deck(
    deck_id: usize,
    track_id: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
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

    with_engine(&mut state, |engine| {
        engine.load_track(deck_id, &source).map_err(|e| e.to_string())
    })?;

    state.decks[deck_id].track = Some(path);
    state.decks[deck_id].playing = false;
    Ok(deck_statuses(&state)[deck_id].clone())
}

#[tauri::command]
fn play_deck(deck_id: usize, state: State<'_, Mutex<AppState>>) -> Result<DeckStatus, String> {
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
fn pause_deck(deck_id: usize, state: State<'_, Mutex<AppState>>) -> Result<DeckStatus, String> {
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

            app.manage(Mutex::new(AppState {
                library,
                engine: None,
                decks: Default::default(),
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_engine,
            stop_engine,
            get_status,
            list_collections,
            add_folder_collection,
            list_collection_tracks,
            load_track,
            load_library_track_to_deck,
            play_deck,
            pause_deck,
            sample_track_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
