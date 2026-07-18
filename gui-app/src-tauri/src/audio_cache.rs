use audio_core::{
    compute_overview_envelope, compute_spectral_window, waveform_buckets_for_window, SpectralPeak,
    WaveformAnalysisConfig,
};
use engine_core::{AudioSource, LoadedAudio};
use library_core::FileAudioSource;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, Weak};
use tauri::State;

use crate::waveform_render::DetailWindow;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct DetailCacheKey {
    start_ms: u64,
    end_ms: u64,
    buckets: usize,
}

#[derive(Clone)]
pub(crate) struct DetailCacheEntry {
    pub peaks: Vec<SpectralPeak>,
    pub start_secs: f64,
    pub end_secs: f64,
}

#[derive(Clone)]
pub(crate) struct OverviewCacheEntry {
    pub peaks: Vec<SpectralPeak>,
    pub duration_secs: f64,
}

pub(crate) struct AudioCache {
    tracks: HashMap<String, Weak<LoadedAudio>>,
    overviews: HashMap<String, OverviewCacheEntry>,
    detail_windows: HashMap<String, HashMap<DetailCacheKey, DetailCacheEntry>>,
}

impl AudioCache {
    pub fn new() -> Self {
        Self {
            tracks: HashMap::new(),
            overviews: HashMap::new(),
            detail_windows: HashMap::new(),
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

    pub fn overview(&self, key: &str) -> Option<OverviewCacheEntry> {
        self.overviews.get(key).cloned()
    }

    pub fn remember_overview(&mut self, key: String, entry: OverviewCacheEntry) {
        self.overviews.insert(key, entry);
    }

    pub fn detail_window(&self, key: &str, cache_key: &DetailCacheKey) -> Option<DetailCacheEntry> {
        self.detail_windows
            .get(key)
            .and_then(|map| map.get(cache_key))
            .cloned()
    }

    pub fn remember_detail_window(
        &mut self,
        key: String,
        cache_key: DetailCacheKey,
        entry: DetailCacheEntry,
    ) {
        let map = self.detail_windows.entry(key).or_default();
        // Keep a small working set so seeks don't grow unboundedly.
        if map.len() >= 8 {
            map.clear();
        }
        map.insert(cache_key, entry);
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

pub(crate) fn duration_secs(audio: &LoadedAudio) -> f64 {
    let channels = usize::from(audio.channels.max(1));
    let frames = audio.samples.len() / channels;
    frames as f64 / f64::from(audio.sample_rate)
}

pub(crate) fn overview_for_audio(audio: &LoadedAudio) -> Vec<SpectralPeak> {
    compute_overview_envelope(audio, &WaveformAnalysisConfig::default())
}

pub(crate) fn detail_window_for_audio(
    audio: &LoadedAudio,
    start_secs: f64,
    end_secs: f64,
    buckets: usize,
) -> DetailWindow {
    let peaks = compute_spectral_window(
        audio,
        start_secs,
        end_secs,
        buckets,
        &WaveformAnalysisConfig::default(),
    );
    DetailWindow {
        peaks,
        start_secs,
        end_secs,
    }
}

pub(crate) fn window_range(
    position_secs: f64,
    visible_secs: f64,
    buffer_ratio: f64,
    duration_secs: f64,
) -> (f64, f64) {
    let buffer_secs = visible_secs * buffer_ratio.max(0.0);
    let start = (position_secs - visible_secs / 2.0 - buffer_secs).max(0.0);
    let end = (position_secs + visible_secs / 2.0 + buffer_secs).min(duration_secs);
    (start, end)
}

pub(crate) fn detail_cache_key(start_secs: f64, end_secs: f64, buckets: usize) -> DetailCacheKey {
    DetailCacheKey {
        start_ms: (start_secs * 1000.0).round() as u64,
        end_ms: (end_secs * 1000.0).round() as u64,
        buckets,
    }
}

pub(crate) async fn get_or_decode(
    app_state: &State<'_, Arc<Mutex<crate::AppState>>>,
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

pub(crate) async fn get_or_compute_overview(
    app_state: &State<'_, Arc<Mutex<crate::AppState>>>,
    cache_key: String,
    path: String,
) -> Result<(Vec<SpectralPeak>, f64), String> {
    if let Some(cached) = app_state
        .lock()
        .map_err(|e| e.to_string())?
        .audio_cache
        .overview(&cache_key)
    {
        return Ok((cached.peaks, cached.duration_secs));
    }

    let audio = get_or_decode(app_state, cache_key.clone(), path).await?;
    let duration = duration_secs(&audio);
    let peaks = tauri::async_runtime::spawn_blocking({
        let audio = Arc::clone(&audio);
        move || overview_for_audio(&audio)
    })
    .await
    .map_err(|e| e.to_string())?;

    app_state
        .lock()
        .map_err(|e| e.to_string())?
        .audio_cache
        .remember_overview(
            cache_key,
            OverviewCacheEntry {
                peaks: peaks.clone(),
                duration_secs: duration,
            },
        );

    Ok((peaks, duration))
}

pub(crate) async fn get_or_compute_detail(
    app_state: &State<'_, Arc<Mutex<crate::AppState>>>,
    cache_key: String,
    path: String,
    position_secs: f64,
    visible_secs: f64,
    buffer_ratio: f64,
    width_px: usize,
) -> Result<Option<DetailWindow>, String> {
    let audio = get_or_decode(app_state, cache_key.clone(), path).await?;
    let duration = duration_secs(&audio);
    let (start_secs, end_secs) = window_range(position_secs, visible_secs, buffer_ratio, duration);
    if end_secs <= start_secs {
        return Ok(None);
    }

    // Quantize positions so nearby playhead updates reuse the same window job.
    let start_q = (start_secs * 4.0).floor() / 4.0;
    let end_q = (end_secs * 4.0).ceil() / 4.0;
    let buckets = waveform_buckets_for_window(end_q - start_q, width_px);
    let detail_key = detail_cache_key(start_q, end_q, buckets);

    if let Some(cached) = app_state
        .lock()
        .map_err(|e| e.to_string())?
        .audio_cache
        .detail_window(&cache_key, &detail_key)
    {
        return Ok(Some(DetailWindow {
            peaks: cached.peaks,
            start_secs: cached.start_secs,
            end_secs: cached.end_secs,
        }));
    }

    let window = tauri::async_runtime::spawn_blocking({
        let audio = Arc::clone(&audio);
        move || detail_window_for_audio(&audio, start_q, end_q, buckets)
    })
    .await
    .map_err(|e| e.to_string())?;

    app_state
        .lock()
        .map_err(|e| e.to_string())?
        .audio_cache
        .remember_detail_window(
            cache_key,
            detail_key,
            DetailCacheEntry {
                peaks: window.peaks.clone(),
                start_secs: window.start_secs,
                end_secs: window.end_secs,
            },
        );

    Ok(Some(window))
}
