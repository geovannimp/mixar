use engine_core::AudioSource;
use engine_core::LoadedAudio;
use library_core::FileAudioSource;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, Weak};
use tauri::State;

pub(crate) struct AudioCache {
    tracks: HashMap<String, Weak<LoadedAudio>>,
}

impl AudioCache {
    pub fn new() -> Self {
        Self {
            tracks: HashMap::new(),
        }
    }

    /// Drop cache entries whose audio is no longer held by any deck.
    pub fn prune(&mut self) {
        self.tracks.retain(|_, weak| weak.strong_count() > 0);
    }

    pub fn get(&mut self, key: &str) -> Option<Arc<LoadedAudio>> {
        self.prune();
        self.tracks.get(key).and_then(|weak| weak.upgrade())
    }

    pub fn remember(&mut self, key: String, audio: &Arc<LoadedAudio>) {
        self.tracks.insert(key, Arc::downgrade(audio));
        self.prune();
    }
}

impl Default for AudioCache {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn decode_file(path: &str) -> Result<LoadedAudio, String> {
    if !Path::new(path).exists() {
        return Err(format!("File not found: {path}"));
    }
    FileAudioSource::from_path(path)
        .load()
        .map_err(|e| e.to_string())
}

pub(crate) async fn get_or_decode(
    app_state: &State<'_, Mutex<crate::AppState>>,
    cache_key: String,
    path: String,
) -> Result<Arc<LoadedAudio>, String> {
    if let Some(cached) = app_state
        .lock()
        .map_err(|e| e.to_string())?
        .audio_cache
        .get(&cache_key)
    {
        return Ok(cached);
    }

    let decoded = tauri::async_runtime::spawn_blocking(move || decode_file(&path))
        .await
        .map_err(|e| e.to_string())??;

    let audio = Arc::new(decoded);
    let mut state = app_state.lock().map_err(|e| e.to_string())?;
    if let Some(cached) = state.audio_cache.get(&cache_key) {
        return Ok(cached);
    }
    state.audio_cache.remember(cache_key, &audio);
    Ok(audio)
}
