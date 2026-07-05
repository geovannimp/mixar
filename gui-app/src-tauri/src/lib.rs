use engine_core::{Engine, EngineConfig};
use library_core::FileAudioSource;
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use tauri::State;

const NUM_DECKS: usize = 2;

#[derive(Debug, Clone, Default, Serialize)]
struct DeckInfo {
    track: Option<String>,
    playing: bool,
}

#[derive(Default)]
struct AppState {
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
        .manage(Mutex::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            start_engine,
            stop_engine,
            get_status,
            load_track,
            play_deck,
            pause_deck,
            sample_track_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
