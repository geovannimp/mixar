//! Session settings host (mirrors Tauri `AppSettings` / `apply_settings`).

use std::sync::{Arc, Mutex, OnceLock};

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
    pub fn open() -> Self {
        Self {
            host: shared_host(),
        }
    }

    /// Current session settings.
    pub fn get_settings(&self) -> Result<AppSettings, String> {
        let host = self.host.lock().map_err(|e| e.to_string())?;
        Ok(settings_from_host(&host))
    }

    /// Apply settings. Does not restart the engine or update the library —
    /// callers apply those separately (mirrors Flutter `saveAppSettings`).
    pub fn save_settings(&self, settings: AppSettings) -> Result<AppSettings, String> {
        let mut host = self.host.lock().map_err(|e| e.to_string())?;
        apply_to_host(&mut host, &settings)?;
        Ok(settings_from_host(&host))
    }
}

/// Bus channel routing mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusChannelMode {
    Stereo,
    Mono,
}

/// Output bus route (device + channels).
#[derive(Clone, Debug)]
pub struct BusRouteSettings {
    pub device_id: String,
    pub left_channel: u16,
    pub right_channel: u16,
    pub mode: BusChannelMode,
}

/// Jog platter policy (maps to [`JogMode`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplerPlayModeSetting {
    Oneshot,
    Hold,
    Loop,
}

/// Sampler ↔ channel-strip routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Full app settings DTO (mirrors Tauri `AppSettings`).
#[derive(Clone, Debug)]
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
}

#[flutter_rust_bridge::frb(ignore)]
struct SettingsHost {
    engine_config: EngineConfig,
    library_table_columns: Vec<String>,
    volume_normalizer_enabled: bool,
    target_lufs: f32,
    sampler_play_mode: SamplerPlayModeSetting,
    sampler_strip_route: SamplerStripRouteSetting,
    deck_default_sampler_bank_id: Vec<Option<String>>,
    default_top_jog_mode: JogModeSetting,
    default_outer_jog_mode: JogModeSetting,
}

impl Default for SettingsHost {
    fn default() -> Self {
        Self {
            engine_config: default_engine_config(),
            library_table_columns: default_library_table_columns(),
            volume_normalizer_enabled: true,
            target_lufs: -18.0,
            sampler_play_mode: SamplerPlayModeSetting::Oneshot,
            sampler_strip_route: SamplerStripRouteSetting::Before,
            deck_default_sampler_bank_id: vec![None, None],
            default_top_jog_mode: JogModeSetting::Vinyl,
            default_outer_jog_mode: JogModeSetting::PitchBend,
        }
    }
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
    }
}

fn apply_to_host(host: &mut SettingsHost, settings: &AppSettings) -> Result<(), String> {
    validate_buffer_size(settings.buffer_size).map_err(|e| e.to_string())?;
    let mut config = host.engine_config.clone();
    config.buses = buses_from_settings(settings);
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
    host.library_table_columns = if settings.library_table_columns.is_empty() {
        default_library_table_columns()
    } else {
        settings.library_table_columns.clone()
    };
    host.volume_normalizer_enabled = settings.volume_normalizer_enabled;
    host.target_lufs = settings.target_lufs;
    host.sampler_play_mode = settings.sampler_play_mode;
    host.sampler_strip_route = settings.sampler_strip_route.into();
    host.deck_default_sampler_bank_id = (0..NUM_DECKS)
        .map(|i| {
            settings
                .deck_default_sampler_bank_id
                .get(i)
                .cloned()
                .flatten()
        })
        .collect();
    host.default_top_jog_mode = settings.default_top_jog_mode;
    host.default_outer_jog_mode = settings.default_outer_jog_mode;
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

    #[test]
    fn default_settings_round_trip() {
        let host = SettingsHost::default();
        let settings = settings_from_host(&host);
        assert_eq!(settings.backend, "cpal");
        assert_eq!(settings.sample_rate, 48_000);
        assert!(settings.volume_normalizer_enabled);
    }
}
