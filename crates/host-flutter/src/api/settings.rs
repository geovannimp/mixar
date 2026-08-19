//! Session settings host (mirrors Tauri `AppSettings` / `apply_settings`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use library_core::LibraryConfig;

use audio_core::{BusConfig, BusId, ChannelMapping, ChannelMode, DeviceId};
use engine_api::JogMode;
use engine_core::{
    validate_buffer_size, AnalysisDurationMode, AudioConfig, EngineConfig,
    SamplerStripRouteSetting, DEFAULT_TEMPO_RANGE, DEFAULT_TEMPO_RANGE_STEPS,
};
use resampler::normalize_resampler_quality;

const NUM_DECKS: usize = 2;
const MASTER_BUS_ID: &str = "master";
const PREVIEW_BUS_ID: &str = "cue";
const TARGET_LUFS_MIN: f32 = -24.0;
const TARGET_LUFS_MAX: f32 = -9.0;
const TARGET_LUFS_DEFAULT: f32 = -18.0;
const LIBRARY_COLUMN_IDS: &[&str] = &[
    "title", "artist", "album", "genre", "bpm", "key", "duration", "path",
];

static SETTINGS: OnceLock<Arc<Mutex<SettingsHost>>> = OnceLock::new();

fn shared_host() -> Arc<Mutex<SettingsHost>> {
    SETTINGS
        .get_or_init(|| Arc::new(Mutex::new(SettingsHost::default())))
        .clone()
}

/// Session settings host (mirrors Tauri `AppSettings` / `apply_settings`).
#[flutter_rust_bridge::frb(opaque)]
pub struct SettingsTransport {
    host: Arc<Mutex<SettingsHost>>,
}

impl SettingsTransport {
    /// Open the app-wide settings host (singleton per process).
    ///
    /// `path` is the JSON file used to load on first open and write on save
    /// (Flutter: app-support `settings.json`, next to `library.db`).
    pub fn open(path: String) -> Self {
        let persist = PathBuf::from(path);
        let host = SETTINGS
            .get_or_init(|| Arc::new(Mutex::new(load_host(&persist))))
            .clone();
        if let Ok(mut locked) = host.lock() {
            bind_persist_path(&mut locked, persist);
        }
        Self { host }
    }

    /// Current session settings (schema-validated).
    pub fn get_settings(&self) -> Result<AppSettings, String> {
        let host = self.host.lock().map_err(|e| e.to_string())?;
        parse_settings(settings_from_host(&host))
    }

    /// Apply settings. Does not restart the engine or update the library —
    /// callers apply those separately (mirrors Flutter `saveAppSettings`).
    pub fn save_settings(&self, settings: AppSettings) -> Result<AppSettings, String> {
        let mut host = self.host.lock().map_err(|e| e.to_string())?;
        apply_to_host(&mut host, settings)?;
        let saved = parse_settings(settings_from_host(&host))?;
        if let Some(path) = host.persist_path.clone() {
            write_settings_file(&path, &saved)?;
        }
        Ok(saved)
    }
}

/// Bus channel routing mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusChannelMode {
    #[default]
    Stereo,
    Mono,
}

/// Output bus route (device + channels).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BusRouteSettings {
    pub device_id: String,
    pub left_channel: u16,
    pub right_channel: u16,
    #[serde(default)]
    pub mode: BusChannelMode,
}

/// Jog platter policy (maps to [`JogMode`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JogModeSetting {
    Vinyl,
    PitchBend,
    Ignore,
}

impl From<JogModeSetting> for JogMode {
    fn from(value: JogModeSetting) -> Self {
        match value {
            JogModeSetting::Vinyl => Self::Vinyl,
            JogModeSetting::PitchBend => Self::PitchBend,
            JogModeSetting::Ignore => Self::Ignore,
        }
    }
}

impl From<JogMode> for JogModeSetting {
    fn from(value: JogMode) -> Self {
        match value {
            JogMode::Vinyl => Self::Vinyl,
            JogMode::PitchBend => Self::PitchBend,
            JogMode::Ignore => Self::Ignore,
        }
    }
}

/// Offline analysis depth (maps to [`AnalysisDurationMode`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisDurationSetting {
    Fast,
    Precise,
    Complete,
}

impl From<AnalysisDurationSetting> for AnalysisDurationMode {
    fn from(value: AnalysisDurationSetting) -> Self {
        match value {
            AnalysisDurationSetting::Fast => Self::Fast,
            AnalysisDurationSetting::Precise => Self::Precise,
            AnalysisDurationSetting::Complete => Self::Complete,
        }
    }
}

impl From<AnalysisDurationMode> for AnalysisDurationSetting {
    fn from(value: AnalysisDurationMode) -> Self {
        match value {
            AnalysisDurationMode::Fast => Self::Fast,
            AnalysisDurationMode::Precise => Self::Precise,
            AnalysisDurationMode::Complete => Self::Complete,
        }
    }
}

/// Default sampler pad play mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplerPlayModeSetting {
    Oneshot,
    Hold,
    Loop,
}

/// Sampler ↔ channel-strip routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplerStripRouteSettingFrb {
    Before,
    After,
}

impl From<SamplerStripRouteSettingFrb> for SamplerStripRouteSetting {
    fn from(value: SamplerStripRouteSettingFrb) -> Self {
        match value {
            SamplerStripRouteSettingFrb::Before => Self::Before,
            SamplerStripRouteSettingFrb::After => Self::After,
        }
    }
}

impl From<SamplerStripRouteSetting> for SamplerStripRouteSettingFrb {
    fn from(value: SamplerStripRouteSetting) -> Self {
        match value {
            SamplerStripRouteSetting::Before => Self::Before,
            SamplerStripRouteSetting::After => Self::After,
        }
    }
}

/// Waveform lane paint mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveformDisplayModeSetting {
    Rgb,
    Filtered,
}

/// Full app settings DTO (mirrors Tauri `AppSettings`).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub backend: String,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub low_latency: bool,
    pub resampler_quality: String,
    pub master_bus: BusRouteSettings,
    pub preview_enabled: bool,
    pub preview_bus: BusRouteSettings,
    pub analysis_duration: AnalysisDurationSetting,
    #[serde(default = "default_scan_folder_tree")]
    pub scan_folder_tree: bool,
    pub library_table_columns: Vec<String>,
    pub volume_normalizer_enabled: bool,
    pub target_lufs: f32,
    pub sampler_play_mode: SamplerPlayModeSetting,
    pub sampler_strip_route: SamplerStripRouteSettingFrb,
    pub deck_default_sampler_bank_id: Vec<Option<String>>,
    pub default_top_jog_mode: JogModeSetting,
    pub default_outer_jog_mode: JogModeSetting,
    pub default_tempo_range: f32,
    pub tempo_range_steps: Vec<f32>,
    pub waveform_display_mode: WaveformDisplayModeSetting,
}

#[flutter_rust_bridge::frb(ignore)]
struct SettingsHost {
    configured: bool,
    persist_path: Option<PathBuf>,
    engine_config: EngineConfig,
    scan_folder_tree: bool,
    library_table_columns: Vec<String>,
    volume_normalizer_enabled: bool,
    target_lufs: f32,
    sampler_play_mode: SamplerPlayModeSetting,
    sampler_strip_route: SamplerStripRouteSetting,
    deck_default_sampler_bank_id: Vec<Option<String>>,
    default_top_jog_mode: JogModeSetting,
    default_outer_jog_mode: JogModeSetting,
    waveform_display_mode: WaveformDisplayModeSetting,
}

impl Default for SettingsHost {
    fn default() -> Self {
        Self {
            configured: false,
            persist_path: None,
            engine_config: default_engine_config(),
            scan_folder_tree: default_scan_folder_tree(),
            library_table_columns: default_library_table_columns(),
            volume_normalizer_enabled: true,
            target_lufs: TARGET_LUFS_DEFAULT,
            sampler_play_mode: SamplerPlayModeSetting::Oneshot,
            sampler_strip_route: SamplerStripRouteSetting::Before,
            deck_default_sampler_bank_id: vec![None, None],
            default_top_jog_mode: JogModeSetting::Vinyl,
            default_outer_jog_mode: JogModeSetting::PitchBend,
            waveform_display_mode: WaveformDisplayModeSetting::Rgb,
        }
    }
}

fn default_scan_folder_tree() -> bool {
    true
}

fn default_library_table_columns() -> Vec<String> {
    vec![
        "title".into(),
        "artist".into(),
        "bpm".into(),
        "key".into(),
        "duration".into(),
    ]
}

fn default_master_bus_route() -> BusRouteSettings {
    BusRouteSettings {
        device_id: "default".into(),
        left_channel: 1,
        right_channel: 2,
        mode: BusChannelMode::Stereo,
    }
}

fn default_preview_bus_route() -> BusRouteSettings {
    BusRouteSettings {
        device_id: "default".into(),
        left_channel: 3,
        right_channel: 4,
        mode: BusChannelMode::Stereo,
    }
}

fn default_engine_config() -> EngineConfig {
    EngineConfig {
        backend: "cpal".into(),
        audio: Some(AudioConfig {
            resampler_quality: Some("medium".into()),
            sampler_strip_route: Some(SamplerStripRouteSetting::Before),
            default_tempo_range: Some(DEFAULT_TEMPO_RANGE),
            tempo_range_steps: Some(DEFAULT_TEMPO_RANGE_STEPS.to_vec()),
        }),
        buses: vec![bus_config(
            MASTER_BUS_ID,
            "Master",
            &default_master_bus_route(),
        )],
        ..Default::default()
    }
}

/// Zod-style parse: reject invalid values, coerce recoverable ones.
fn parse_settings(mut settings: AppSettings) -> Result<AppSettings, String> {
    let mut errors = Vec::new();

    settings.backend = settings.backend.trim().to_string();
    if settings.backend.is_empty() {
        errors.push("backend must not be empty".into());
    }
    if settings.sample_rate == 0 {
        errors.push("sample_rate must be > 0".into());
    }
    if let Err(e) = validate_buffer_size(settings.buffer_size) {
        errors.push(e.to_string());
    }
    settings.resampler_quality =
        normalize_resampler_quality(Some(settings.resampler_quality.as_str())).to_string();

    if let Err(e) = parse_bus("master_bus", &settings.master_bus) {
        errors.push(e);
    }
    if let Err(e) = parse_bus("preview_bus", &settings.preview_bus) {
        errors.push(e);
    }

    if !settings.target_lufs.is_finite() {
        errors.push("target_lufs must be finite".into());
    } else if !(TARGET_LUFS_MIN..=TARGET_LUFS_MAX).contains(&settings.target_lufs) {
        errors.push(format!(
            "target_lufs must be between {TARGET_LUFS_MIN} and {TARGET_LUFS_MAX}"
        ));
    }

    if !settings.default_tempo_range.is_finite() || settings.default_tempo_range <= 0.0 {
        errors.push("default_tempo_range must be finite and > 0".into());
    }
    settings
        .tempo_range_steps
        .retain(|step| step.is_finite() && *step > 0.0);
    if settings.tempo_range_steps.is_empty() {
        errors.push("tempo_range_steps must contain at least one finite > 0 step".into());
    }

    let allowed: HashSet<&str> = LIBRARY_COLUMN_IDS.iter().copied().collect();
    let mut columns: Vec<String> = settings
        .library_table_columns
        .into_iter()
        .filter(|id| allowed.contains(id.as_str()))
        .collect();
    if !columns.iter().any(|id| id == "title") {
        columns.insert(0, "title".into());
    }
    if columns.is_empty() {
        columns = default_library_table_columns();
    }
    settings.library_table_columns = columns;

    let mut banks = settings.deck_default_sampler_bank_id;
    banks.resize(NUM_DECKS, None);
    banks.truncate(NUM_DECKS);
    settings.deck_default_sampler_bank_id = banks
        .into_iter()
        .map(|id| match id {
            Some(s) if s.trim().is_empty() => None,
            other => other,
        })
        .collect();

    if errors.is_empty() {
        Ok(settings)
    } else {
        Err(errors.join("; "))
    }
}

fn parse_bus(field: &str, route: &BusRouteSettings) -> Result<(), String> {
    if route.device_id.trim().is_empty() {
        return Err(format!("{field}.device_id must not be empty"));
    }
    match route.mode {
        BusChannelMode::Mono => {
            if route.left_channel == 0 {
                return Err(format!("{field}.left_channel must be 1-based"));
            }
        }
        BusChannelMode::Stereo => {
            if route.left_channel == 0 || route.right_channel == 0 {
                return Err(format!("{field} channels must be 1-based"));
            }
            if route.left_channel == route.right_channel {
                return Err(format!("{field} left and right channels must be distinct"));
            }
        }
    }
    Ok(())
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
        BusChannelMode::Stereo => ChannelMapping::stereo(route.left_channel, route.right_channel),
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
        buses.push(bus_config(PREVIEW_BUS_ID, "Preview", &settings.preview_bus));
    }
    buses
}

fn settings_from_host(host: &SettingsHost) -> AppSettings {
    let config = &host.engine_config;
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
        analysis_duration: config.analysis_duration.into(),
        scan_folder_tree: host.scan_folder_tree,
        library_table_columns: host.library_table_columns.clone(),
        volume_normalizer_enabled: host.volume_normalizer_enabled,
        target_lufs: host.target_lufs,
        sampler_play_mode: host.sampler_play_mode,
        sampler_strip_route: host.sampler_strip_route.into(),
        deck_default_sampler_bank_id: host.deck_default_sampler_bank_id.clone(),
        default_top_jog_mode: host.default_top_jog_mode,
        default_outer_jog_mode: host.default_outer_jog_mode,
        default_tempo_range: config.default_tempo_range(),
        tempo_range_steps: config.tempo_range_steps(),
        waveform_display_mode: host.waveform_display_mode,
    }
}

fn apply_to_host(host: &mut SettingsHost, settings: AppSettings) -> Result<(), String> {
    let settings = parse_settings(settings)?;
    let mut config = host.engine_config.clone();
    config.buses = buses_from_settings(&settings);
    config.backend = settings.backend.clone();
    config.sample_rate = settings.sample_rate;
    config.buffer_size = settings.buffer_size;
    config.low_latency = settings.low_latency;
    config.analysis_duration = settings.analysis_duration.into();
    config.audio = Some(AudioConfig {
        resampler_quality: Some(settings.resampler_quality.clone()),
        sampler_strip_route: Some(settings.sampler_strip_route.into()),
        default_tempo_range: Some(settings.default_tempo_range),
        tempo_range_steps: Some(settings.tempo_range_steps.clone()),
    });
    config.validate().map_err(|e| e.to_string())?;
    host.engine_config = config;
    host.scan_folder_tree = settings.scan_folder_tree;
    host.library_table_columns = settings.library_table_columns;
    host.volume_normalizer_enabled = settings.volume_normalizer_enabled;
    host.target_lufs = settings.target_lufs;
    host.sampler_play_mode = settings.sampler_play_mode;
    host.sampler_strip_route = settings.sampler_strip_route.into();
    host.deck_default_sampler_bank_id = settings.deck_default_sampler_bank_id;
    host.default_top_jog_mode = settings.default_top_jog_mode;
    host.default_outer_jog_mode = settings.default_outer_jog_mode;
    host.waveform_display_mode = settings.waveform_display_mode;
    host.configured = true;
    Ok(())
}

fn load_host(path: &Path) -> SettingsHost {
    let mut host = SettingsHost {
        persist_path: Some(path.to_path_buf()),
        ..SettingsHost::default()
    };
    if let Some(settings) = read_settings_file(path) {
        let _ = apply_to_host(&mut host, settings);
    }
    host
}

fn bind_persist_path(host: &mut SettingsHost, path: PathBuf) {
    if host.persist_path.is_some() {
        return;
    }
    host.persist_path = Some(path.clone());
    if !host.configured {
        if let Some(settings) = read_settings_file(&path) {
            let _ = apply_to_host(host, settings);
        }
    }
}

fn read_settings_file(path: &Path) -> Option<AppSettings> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_settings_file(path: &Path, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

fn normalizer_target(enabled: bool, target_lufs: f32) -> Option<f32> {
    enabled.then_some(target_lufs)
}

/// Engine config for [`EngineTransport::start`] / restart.
pub(crate) fn settings_engine_config() -> Result<EngineConfig, String> {
    let host = shared_host();
    let host = host.lock().map_err(|e| e.to_string())?;
    Ok(host.engine_config.clone())
}

/// Library scan config from the settings host (defaults if unset).
pub(crate) fn settings_library_config() -> LibraryConfig {
    shared_host()
        .lock()
        .map(|h| LibraryConfig {
            scan_folder_tree: h.scan_folder_tree,
        })
        .unwrap_or_default()
}

/// Overlay [`EngineStartConfig`] onto the host until the user has saved settings.
pub(crate) fn seed_engine_config_if_unconfigured(
    backend: &str,
    sample_rate: Option<u32>,
    buffer_size: Option<u32>,
) -> Result<EngineConfig, String> {
    let host = shared_host();
    let mut host = host.lock().map_err(|e| e.to_string())?;
    if !host.configured {
        host.engine_config.backend = backend.to_string();
        if let Some(sr) = sample_rate {
            host.engine_config.sample_rate = sr;
        }
        if let Some(bs) = buffer_size {
            host.engine_config.buffer_size = bs;
        }
    }
    Ok(host.engine_config.clone())
}

/// Host-only engine settings applied after start/restart.
pub(crate) fn settings_host_runtime() -> Result<(Option<f32>, JogMode, JogMode), String> {
    let host = shared_host();
    let host = host.lock().map_err(|e| e.to_string())?;
    Ok((
        normalizer_target(host.volume_normalizer_enabled, host.target_lufs),
        host.default_top_jog_mode.into(),
        host.default_outer_jog_mode.into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_settings() -> AppSettings {
        settings_from_host(&SettingsHost::default())
    }

    #[test]
    fn default_settings_round_trip() {
        let parsed = parse_settings(sample_settings()).expect("defaults parse");
        assert_eq!(parsed.backend, "cpal");
        assert_eq!(parsed.sample_rate, 48_000);
        assert!(parsed.volume_normalizer_enabled);
        assert!(parsed.scan_folder_tree);
        assert_eq!(
            parsed.waveform_display_mode,
            WaveformDisplayModeSetting::Rgb
        );
    }

    #[test]
    fn parse_rejects_non_finite_target_lufs() {
        let mut settings = sample_settings();
        settings.target_lufs = f32::NAN;
        let err = parse_settings(settings).unwrap_err();
        assert!(err.contains("target_lufs"), "{err}");
    }

    #[test]
    fn parse_rejects_zero_bus_channel() {
        let mut settings = sample_settings();
        settings.master_bus.left_channel = 0;
        let err = parse_settings(settings).unwrap_err();
        assert!(err.contains("1-based"), "{err}");
    }

    #[test]
    fn apply_to_host_rejects_nan_and_leaves_host_unchanged() {
        let mut host = SettingsHost::default();
        let mut settings = sample_settings();
        settings.target_lufs = f32::NAN;
        assert!(apply_to_host(&mut host, settings).is_err());
        assert!(!host.configured);
        assert_eq!(host.target_lufs, TARGET_LUFS_DEFAULT);
    }

    #[test]
    fn load_parses_host_snapshot() {
        let host = SettingsHost::default();
        let settings = parse_settings(settings_from_host(&host)).expect("load");
        assert_eq!(settings.buffer_size, 512);
        assert_eq!(settings.target_lufs, TARGET_LUFS_DEFAULT);
    }

    #[test]
    fn apply_to_host_persists_waveform_display_mode() {
        let mut host = SettingsHost::default();
        let mut settings = sample_settings();
        settings.waveform_display_mode = WaveformDisplayModeSetting::Filtered;
        apply_to_host(&mut host, settings).expect("settings apply");
        assert_eq!(
            settings_from_host(&host).waveform_display_mode,
            WaveformDisplayModeSetting::Filtered,
        );
    }

    #[test]
    fn apply_to_host_persists_scan_folder_tree() {
        let mut host = SettingsHost::default();
        let mut settings = sample_settings();
        settings.scan_folder_tree = false;
        apply_to_host(&mut host, settings).expect("settings apply");
        assert!(!settings_from_host(&host).scan_folder_tree);
    }

    #[test]
    fn settings_file_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.scan_folder_tree = false;
        settings.waveform_display_mode = WaveformDisplayModeSetting::Filtered;
        settings.sample_rate = 44_100;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(!restored.scan_folder_tree);
        assert_eq!(
            restored.waveform_display_mode,
            WaveformDisplayModeSetting::Filtered
        );
        assert_eq!(restored.sample_rate, 44_100);
    }

    #[test]
    fn missing_scan_folder_tree_defaults_true() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("scan_folder_tree");
        std::fs::write(&path, serde_json::to_vec(&value).expect("write")).expect("disk");
        let loaded = read_settings_file(&path).expect("read");
        assert!(loaded.scan_folder_tree);
    }

    #[test]
    fn corrupt_settings_file_is_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        std::fs::write(&path, b"not json").expect("write");
        assert!(read_settings_file(&path).is_none());
        let host = load_host(&path);
        assert!(!host.configured);
        assert_eq!(host.engine_config.sample_rate, 48_000);
    }
}
