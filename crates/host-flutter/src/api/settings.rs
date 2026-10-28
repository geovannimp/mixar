//! Session settings host (mirrors Tauri `AppSettings` / `apply_settings`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::bridge::BridgeResult;
use audio_core::{BusConfig, BusId, ChannelMapping, ChannelMode, DeviceId};
use engine_api::JogMode;
use engine_core::{
    validate_buffer_size, AnalysisDurationMode, AudioConfig, EngineConfig,
    SamplerStripRouteSetting, DEFAULT_TEMPO_RANGE, DEFAULT_TEMPO_RANGE_STEPS,
};
use resampler::normalize_resampler_quality;

const MASTER_BUS_ID: &str = "master";
const PREVIEW_BUS_ID: &str = "cue";
const TARGET_LUFS_MIN: f32 = -24.0;
const TARGET_LUFS_MAX: f32 = -9.0;
const TARGET_LUFS_DEFAULT: f32 = -18.0;
/// Sample rate `EngineConfig::default()` used before rates moved onto the buses.
const LEGACY_DEFAULT_SAMPLE_RATE: u32 = 48_000;

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
    /// Persists `{app-support}/settings.json`, next to `library.db`.
    pub fn open() -> Self {
        Self::open_at(&crate::app_paths::settings_json())
    }

    /// Open against an explicit JSON path (tests: temp directories).
    pub(crate) fn open_at(path: &Path) -> Self {
        let persist = path.to_path_buf();
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
        let host = self.host.lock().bridge()?;
        parse_settings(settings_from_host(&host))
    }

    /// Apply settings. Does not restart the engine or update the library —
    /// callers apply those separately (mirrors Flutter `saveAppSettings`).
    pub fn save_settings(&self, settings: AppSettings) -> Result<AppSettings, String> {
        let mut host = self.host.lock().bridge()?;
        let mut candidate = host.clone();
        apply_to_host(&mut candidate, settings)?;
        let saved = parse_settings(settings_from_host(&candidate))?;
        if let Some(path) = host.persist_path.clone() {
            write_settings_file(&path, &saved)?;
        }
        *host = candidate;
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

/// Output bus route (device + channels + rate).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BusRouteSettings {
    pub device_id: String,
    pub left_channel: u16,
    pub right_channel: u16,
    #[serde(default)]
    pub mode: BusChannelMode,
    /// Rate this bus's device stream runs at. `None` follows the rate the system
    /// mixes the device at (Settings → Audio shows it as "Device default").
    #[serde(default)]
    pub sample_rate: Option<u32>,
    /// Ask the backend for a shorter path on this bus's device.
    #[serde(default)]
    pub low_latency: bool,
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

/// Musical vs Camelot key labels in the UI (maps to [`library_core::KeyDisplayMode`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyDisplayModeSetting {
    #[default]
    Musical,
    Camelot,
}

impl From<KeyDisplayModeSetting> for library_core::KeyDisplayMode {
    fn from(value: KeyDisplayModeSetting) -> Self {
        match value {
            KeyDisplayModeSetting::Musical => Self::Musical,
            KeyDisplayModeSetting::Camelot => Self::Camelot,
        }
    }
}

impl From<library_core::KeyDisplayMode> for KeyDisplayModeSetting {
    fn from(value: library_core::KeyDisplayMode) -> Self {
        match value {
            library_core::KeyDisplayMode::Musical => Self::Musical,
            library_core::KeyDisplayMode::Camelot => Self::Camelot,
        }
    }
}

/// Track key color coding in deck chrome and library table (UI-only).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyColorModeSetting {
    #[default]
    Off,
    Absolute,
    Harmonic,
}

/// Row layout of the library track list. `Compact` is the dense single line,
/// `Comfortable` the two-line default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryRowDensitySetting {
    Compact,
    #[default]
    Comfortable,
}

/// One metadata field a library track row can show (Settings → UI → Library
/// list → Visible fields).
///
/// The order of [`AppSettings::library_row_fields`] is the render order: chips
/// paint in that order on the row's single metadata line, and a chip that
/// cannot fit is left out on its own — the fields after it still get their
/// chance. The comment is the one exception: free text is cut to an ellipsized
/// snippet, because dropping it would hide the field rather than shorten it.
/// See [`default_library_row_fields`] for today's layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryRowFieldSetting {
    Album,
    Genre,
    Year,
    TrackNumber,
    DiscNumber,
    Comment,
    Isrc,
    Duration,
    Rating,
    Color,
    Bpm,
    Key,
}

/// Row fields shown before the user changes them: the comfortable row's
/// album/genre/length pills on the left and rating/colour/BPM/key trailing.
fn default_library_row_fields() -> Vec<LibraryRowFieldSetting> {
    vec![
        LibraryRowFieldSetting::Album,
        LibraryRowFieldSetting::Genre,
        LibraryRowFieldSetting::Duration,
        LibraryRowFieldSetting::Rating,
        LibraryRowFieldSetting::Color,
        LibraryRowFieldSetting::Bpm,
        LibraryRowFieldSetting::Key,
    ]
}

/// Read the saved field order, dropping names this build does not know so a
/// settings file from a newer (or older) build still loads instead of falling
/// back to defaults wholesale.
fn deserialize_library_row_fields<'de, D>(
    deserializer: D,
) -> Result<Vec<LibraryRowFieldSetting>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let values = Vec::<serde_json::Value>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect())
}

/// Select / dropdown presentation. `Auto` picks desktop vs mobile from the host OS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectStyleSetting {
    #[default]
    Auto,
    Desktop,
    Mobile,
}

/// App UI language preference (Settings → UI).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiLanguageSetting {
    #[default]
    System,
    En,
    PtBr,
}

/// Edge the library drawer opens from (Settings → UI).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryDrawerSideSetting {
    Left,
    Right,
    Top,
    #[default]
    Bottom,
}

fn default_vinyl_brake_stop_secs() -> f32 {
    0.5
}
fn default_num_decks() -> u8 {
    2
}

/// Full app settings DTO (mirrors Tauri `AppSettings`).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub buffer_size: u32,
    pub resampler_quality: String,
    pub master_bus: BusRouteSettings,
    pub preview_enabled: bool,
    pub preview_bus: BusRouteSettings,
    pub analysis_duration: AnalysisDurationSetting,
    #[serde(default)]
    pub library_row_density: LibraryRowDensitySetting,
    /// Metadata fields a library row shows, in render order. A chip that does
    /// not fit the row's single metadata line is dropped on its own, except the
    /// comment, which is cut to an ellipsized snippet; the trailing cluster
    /// keeps its fixed rating/colour/BPM/key order.
    #[serde(
        default = "default_library_row_fields",
        deserialize_with = "deserialize_library_row_fields"
    )]
    pub library_row_fields: Vec<LibraryRowFieldSetting>,
    /// Mirror app-managed BPM/key into the file's tags after analysis (off by default).
    #[serde(default)]
    pub tag_write_back: bool,
    pub volume_normalizer_enabled: bool,
    pub target_lufs: f32,
    pub sampler_play_mode: SamplerPlayModeSetting,
    pub sampler_strip_route: SamplerStripRouteSettingFrb,
    #[serde(default = "default_num_decks")]
    pub num_decks: u8,
    pub deck_default_sampler_bank_id: Vec<Option<String>>,
    pub default_top_jog_mode: JogModeSetting,
    pub default_outer_jog_mode: JogModeSetting,
    pub default_tempo_range: f32,
    pub tempo_range_steps: Vec<f32>,
    #[serde(default)]
    pub default_key_lock: bool,
    #[serde(default)]
    pub vinyl_brake_pause: bool,
    #[serde(default = "default_vinyl_brake_stop_secs")]
    pub vinyl_brake_stop_secs: f32,
    pub waveform_display_mode: WaveformDisplayModeSetting,
    #[serde(default)]
    pub key_display_mode: KeyDisplayModeSetting,
    #[serde(default)]
    pub key_color_mode: KeyColorModeSetting,
    #[serde(default)]
    pub trusted_controller_device_ids: Vec<String>,
    #[serde(default = "default_history_enabled")]
    pub history_enabled: bool,
    #[serde(default = "default_history_session_idle_minutes")]
    pub history_session_idle_minutes: u32,
    #[serde(default = "default_history_min_play_seconds")]
    pub history_min_play_seconds: u32,
    #[serde(default = "default_history_min_deck_volume")]
    pub history_min_deck_volume: f32,
    #[serde(default = "default_show_tooltips")]
    pub show_tooltips: bool,
    /// Show vertical deck VU meters between mixer volume faders.
    #[serde(default = "default_show_mixer_meters")]
    pub show_mixer_meters: bool,
    /// When mixer meters are shown, use mono (max L/R) instead of stereo ladders.
    #[serde(default = "default_mixer_meters_mono")]
    pub mixer_meters_mono: bool,
    #[serde(default)]
    pub select_style: SelectStyleSetting,
    #[serde(default = "default_dim_played_tracks")]
    pub dim_played_tracks: bool,
    /// Stem cache codec: `opus` (default) or `flac`.
    #[serde(default = "default_stems_format")]
    pub stems_format: String,
    /// Offline separation model id (`htdemucs_mixxx_v1` | `scnet_int8_v1`).
    #[serde(default = "default_stems_model")]
    pub stems_model: String,
    /// Live separation fallback for tracks without cached stems. Offline
    /// stems always take priority; quality is lower and it uses a CPU core.
    #[serde(default)]
    pub realtime_stems: bool,
    #[serde(default)]
    pub ui_language: UiLanguageSetting,
    /// Open the library as an overlay drawer instead of the docked panel.
    #[serde(default)]
    pub library_drawer_enabled: bool,
    /// Edge the drawer opens from.
    #[serde(default)]
    pub library_drawer_side: LibraryDrawerSideSetting,
    #[serde(default)]
    pub analytics_enabled: bool,
}

#[flutter_rust_bridge::frb(ignore)]
#[derive(Clone)]
struct SettingsHost {
    configured: bool,
    persist_path: Option<PathBuf>,
    engine_config: EngineConfig,
    library_row_density: LibraryRowDensitySetting,
    library_row_fields: Vec<LibraryRowFieldSetting>,
    tag_write_back: bool,
    volume_normalizer_enabled: bool,
    target_lufs: f32,
    sampler_play_mode: SamplerPlayModeSetting,
    sampler_strip_route: SamplerStripRouteSetting,
    deck_default_sampler_bank_id: Vec<Option<String>>,
    default_top_jog_mode: JogModeSetting,
    default_outer_jog_mode: JogModeSetting,
    waveform_display_mode: WaveformDisplayModeSetting,
    key_display_mode: KeyDisplayModeSetting,
    key_color_mode: KeyColorModeSetting,
    trusted_controller_device_ids: Vec<String>,
    history_enabled: bool,
    history_session_idle_minutes: u32,
    history_min_play_seconds: u32,
    history_min_deck_volume: f32,
    show_tooltips: bool,
    show_mixer_meters: bool,
    mixer_meters_mono: bool,
    select_style: SelectStyleSetting,
    dim_played_tracks: bool,
    stems_format: String,
    stems_model: String,
    realtime_stems: bool,
    ui_language: UiLanguageSetting,
    library_drawer_enabled: bool,
    library_drawer_side: LibraryDrawerSideSetting,
    analytics_enabled: bool,
}

impl Default for SettingsHost {
    fn default() -> Self {
        Self {
            configured: false,
            persist_path: None,
            engine_config: default_engine_config(),
            library_row_density: LibraryRowDensitySetting::default(),
            library_row_fields: default_library_row_fields(),
            tag_write_back: false,
            volume_normalizer_enabled: true,
            target_lufs: TARGET_LUFS_DEFAULT,
            sampler_play_mode: SamplerPlayModeSetting::Oneshot,
            sampler_strip_route: SamplerStripRouteSetting::Before,
            deck_default_sampler_bank_id: vec![None, None],
            default_top_jog_mode: JogModeSetting::Vinyl,
            default_outer_jog_mode: JogModeSetting::PitchBend,
            waveform_display_mode: WaveformDisplayModeSetting::Rgb,
            key_display_mode: KeyDisplayModeSetting::Musical,
            key_color_mode: KeyColorModeSetting::Off,
            trusted_controller_device_ids: Vec::new(),
            history_enabled: default_history_enabled(),
            history_session_idle_minutes: default_history_session_idle_minutes(),
            history_min_play_seconds: default_history_min_play_seconds(),
            history_min_deck_volume: default_history_min_deck_volume(),
            show_tooltips: default_show_tooltips(),
            show_mixer_meters: default_show_mixer_meters(),
            mixer_meters_mono: default_mixer_meters_mono(),
            select_style: SelectStyleSetting::default(),
            dim_played_tracks: default_dim_played_tracks(),
            stems_format: default_stems_format(),
            stems_model: default_stems_model(),
            realtime_stems: false,
            ui_language: UiLanguageSetting::default(),
            library_drawer_enabled: false,
            library_drawer_side: LibraryDrawerSideSetting::Bottom,
            analytics_enabled: false,
        }
    }
}

fn default_history_enabled() -> bool {
    true
}

fn default_history_session_idle_minutes() -> u32 {
    5
}

fn default_history_min_play_seconds() -> u32 {
    5
}

fn default_history_min_deck_volume() -> f32 {
    0.05
}

fn default_show_tooltips() -> bool {
    true
}

fn default_show_mixer_meters() -> bool {
    true
}

fn default_mixer_meters_mono() -> bool {
    true
}

fn default_dim_played_tracks() -> bool {
    true
}

fn default_stems_format() -> String {
    "opus".into()
}

fn default_stems_model() -> String {
    analyzer_stems::DEFAULT_MODEL.into()
}

fn normalize_stems_model(model: &str) -> String {
    let model = model.trim();
    if analyzer_stems::STEM_MODELS.contains(&model) {
        model.into()
    } else {
        default_stems_model()
    }
}

fn normalize_stems_format(format: &str) -> String {
    match format.trim() {
        "flac" => "flac".into(),
        "aac" => "opus".into(), // not implemented yet
        _ => "opus".into(),
    }
}

fn default_master_bus_route() -> BusRouteSettings {
    BusRouteSettings {
        device_id: "default".into(),
        left_channel: 1,
        right_channel: 2,
        mode: BusChannelMode::Stereo,
        sample_rate: None,
        low_latency: false,
    }
}

fn default_preview_bus_route() -> BusRouteSettings {
    BusRouteSettings {
        device_id: "default".into(),
        left_channel: 3,
        right_channel: 4,
        mode: BusChannelMode::Stereo,
        sample_rate: None,
        low_latency: false,
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
            default_key_lock: Some(false),
            vinyl_brake_pause: Some(false),
            vinyl_brake_stop_secs: Some(0.5),
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
    if !matches!(settings.num_decks, 2 | 4) {
        errors.push("num_decks must be 2 or 4".into());
        settings.num_decks = default_num_decks();
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
    settings.vinyl_brake_stop_secs = if settings.vinyl_brake_stop_secs.is_finite() {
        settings.vinyl_brake_stop_secs.clamp(0.05, 3.0)
    } else {
        0.5
    };

    let mut banks = settings.deck_default_sampler_bank_id;
    let num_decks = usize::from(settings.num_decks);
    banks.resize(num_decks, None);
    banks.truncate(num_decks);
    settings.deck_default_sampler_bank_id = banks
        .into_iter()
        .map(|id| match id {
            Some(s) if s.trim().is_empty() => None,
            other => other,
        })
        .collect();

    if settings.history_session_idle_minutes == 0 {
        errors.push("history_session_idle_minutes must be > 0".into());
    }
    if settings.history_min_play_seconds == 0 {
        errors.push("history_min_play_seconds must be > 0".into());
    }
    if !settings.history_min_deck_volume.is_finite()
        || !(0.0..=1.0).contains(&settings.history_min_deck_volume)
    {
        errors.push("history_min_deck_volume must be between 0 and 1".into());
    }

    settings.stems_format = normalize_stems_format(&settings.stems_format);
    settings.stems_model = normalize_stems_model(&settings.stems_model);

    // Order is the user's priority, so keep the first occurrence of each field.
    // An empty list is a real choice (no metadata pills), not a broken value.
    let mut seen = Vec::with_capacity(settings.library_row_fields.len());
    settings.library_row_fields.retain(|field| {
        if seen.contains(field) {
            false
        } else {
            seen.push(*field);
            true
        }
    });

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
    if route.sample_rate == Some(0) {
        return Err(format!("{field}.sample_rate must be > 0"));
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
        sample_rate: bus.sample_rate,
        low_latency: bus.low_latency,
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
    .with_sample_rate(route.sample_rate)
    .with_low_latency(route.low_latency)
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
        buffer_size: config.buffer_size,
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
        library_row_density: host.library_row_density,
        library_row_fields: host.library_row_fields.clone(),
        tag_write_back: host.tag_write_back,
        volume_normalizer_enabled: host.volume_normalizer_enabled,
        target_lufs: host.target_lufs,
        sampler_play_mode: host.sampler_play_mode,
        sampler_strip_route: host.sampler_strip_route.into(),
        num_decks: config.num_decks as u8,
        deck_default_sampler_bank_id: host.deck_default_sampler_bank_id.clone(),
        default_top_jog_mode: host.default_top_jog_mode,
        default_outer_jog_mode: host.default_outer_jog_mode,
        default_tempo_range: config.default_tempo_range(),
        tempo_range_steps: config.tempo_range_steps(),
        default_key_lock: config.default_key_lock(),
        vinyl_brake_pause: config.vinyl_brake_pause(),
        vinyl_brake_stop_secs: config.vinyl_brake_stop_secs(),
        waveform_display_mode: host.waveform_display_mode,
        key_display_mode: host.key_display_mode,
        key_color_mode: host.key_color_mode,
        trusted_controller_device_ids: host.trusted_controller_device_ids.clone(),
        history_enabled: host.history_enabled,
        history_session_idle_minutes: host.history_session_idle_minutes,
        history_min_play_seconds: host.history_min_play_seconds,
        history_min_deck_volume: host.history_min_deck_volume,
        show_tooltips: host.show_tooltips,
        show_mixer_meters: host.show_mixer_meters,
        mixer_meters_mono: host.mixer_meters_mono,
        select_style: host.select_style,
        dim_played_tracks: host.dim_played_tracks,
        stems_format: host.stems_format.clone(),
        stems_model: host.stems_model.clone(),
        realtime_stems: host.realtime_stems,
        ui_language: host.ui_language,
        library_drawer_enabled: host.library_drawer_enabled,
        library_drawer_side: host.library_drawer_side,
        analytics_enabled: host.analytics_enabled,
    }
}

fn apply_to_host(host: &mut SettingsHost, settings: AppSettings) -> Result<(), String> {
    let settings = parse_settings(settings)?;
    let mut config = host.engine_config.clone();
    config.num_decks = usize::from(settings.num_decks);
    config.buses = buses_from_settings(&settings);
    config.buffer_size = settings.buffer_size;
    config.analysis_duration = settings.analysis_duration.into();
    config.audio = Some(AudioConfig {
        resampler_quality: Some(settings.resampler_quality.clone()),
        sampler_strip_route: Some(settings.sampler_strip_route.into()),
        default_tempo_range: Some(settings.default_tempo_range),
        tempo_range_steps: Some(settings.tempo_range_steps.clone()),
        default_key_lock: Some(settings.default_key_lock),
        vinyl_brake_pause: Some(settings.vinyl_brake_pause),
        vinyl_brake_stop_secs: Some(settings.vinyl_brake_stop_secs),
    });
    config.validate().bridge()?;
    host.engine_config = config;
    host.library_row_density = settings.library_row_density;
    host.library_row_fields = settings.library_row_fields;
    host.tag_write_back = settings.tag_write_back;
    host.volume_normalizer_enabled = settings.volume_normalizer_enabled;
    host.target_lufs = settings.target_lufs;
    host.sampler_play_mode = settings.sampler_play_mode;
    host.sampler_strip_route = settings.sampler_strip_route.into();
    host.deck_default_sampler_bank_id = settings.deck_default_sampler_bank_id;
    host.default_top_jog_mode = settings.default_top_jog_mode;
    host.default_outer_jog_mode = settings.default_outer_jog_mode;
    host.waveform_display_mode = settings.waveform_display_mode;
    host.key_display_mode = settings.key_display_mode;
    host.key_color_mode = settings.key_color_mode;
    host.trusted_controller_device_ids = settings.trusted_controller_device_ids;
    host.history_enabled = settings.history_enabled;
    host.history_session_idle_minutes = settings.history_session_idle_minutes;
    host.history_min_play_seconds = settings.history_min_play_seconds;
    host.history_min_deck_volume = settings.history_min_deck_volume;
    host.show_tooltips = settings.show_tooltips;
    host.show_mixer_meters = settings.show_mixer_meters;
    host.mixer_meters_mono = settings.mixer_meters_mono;
    host.select_style = settings.select_style;
    host.dim_played_tracks = settings.dim_played_tracks;
    host.stems_format = settings.stems_format;
    host.stems_model = settings.stems_model;
    host.realtime_stems = settings.realtime_stems;
    host.ui_language = settings.ui_language;
    host.library_drawer_enabled = settings.library_drawer_enabled;
    host.analytics_enabled = settings.analytics_enabled;
    host.library_drawer_side = settings.library_drawer_side;
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
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    // `low_latency` used to be a single app-wide flag. `true` was always an explicit
    // choice, so fold it into both routes; `false` is what the bus defaults already
    // carry. The key itself is dropped on the next save.
    let legacy_low_latency = value
        .get("low_latency")
        .and_then(serde_json::Value::as_bool);
    let legacy_sample_rate = value
        .get("sample_rate")
        .and_then(serde_json::Value::as_u64)
        .and_then(|rate| u32::try_from(rate).ok());
    let mut settings: AppSettings = serde_json::from_value(value).ok()?;
    if legacy_low_latency == Some(true) {
        settings.master_bus.low_latency = true;
        settings.preview_bus.low_latency = true;
    }
    // `sample_rate` used to be app-wide. 48000 was the built-in default and was
    // written even when never chosen, so migrating it would pin every existing
    // install to 48 kHz — the behaviour this change removes. Any other value was
    // a real choice, so keep it on the routes that had none.
    if let Some(rate) = legacy_sample_rate.filter(|rate| *rate != LEGACY_DEFAULT_SAMPLE_RATE) {
        // Only the master bus inherits it: the old global described the engine's
        // stream, and the preview bus may sit on a device that cannot run it.
        if settings.master_bus.sample_rate.is_none() {
            settings.master_bus.sample_rate = Some(rate);
        }
    }
    Some(settings)
}

fn write_settings_file(path: &Path, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).bridge()?;
    }
    let json = serde_json::to_vec_pretty(settings).bridge()?;
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, json).bridge()?;
    std::fs::rename(&tmp, path).bridge()?;
    Ok(())
}

fn normalizer_target(enabled: bool, target_lufs: f32) -> Option<f32> {
    enabled.then_some(target_lufs)
}

/// Engine config for [`EngineTransport::start`] / restart.
pub(crate) fn settings_engine_config() -> Result<EngineConfig, String> {
    let host = shared_host();
    let host = host.lock().bridge()?;
    Ok(host.engine_config.clone())
}

/// Whether the opt-in live StemgenRT filter is enabled for this session.
///
/// Defaults to `false`; a poisoned host lock is treated as disabled so the
/// audio/load path never starts live separation unexpectedly.
pub(crate) fn settings_realtime_stems_enabled() -> bool {
    let host = shared_host();
    host.lock()
        .map(|guard| guard.realtime_stems)
        .unwrap_or(false)
}

pub(crate) fn settings_host_runtime() -> Result<(Option<f32>, JogMode, JogMode), String> {
    let host = shared_host();
    let host = host.lock().bridge()?;
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
        // Fresh installs follow the device's mix rate rather than pinning one.
        assert_eq!(parsed.master_bus.sample_rate, None);
        assert_eq!(parsed.preview_bus.sample_rate, None);
        assert!(parsed.volume_normalizer_enabled);
        assert_eq!(
            parsed.waveform_display_mode,
            WaveformDisplayModeSetting::Rgb
        );
        assert_eq!(parsed.key_display_mode, KeyDisplayModeSetting::Musical);
        assert_eq!(parsed.key_color_mode, KeyColorModeSetting::Off);
        assert!(parsed.trusted_controller_device_ids.is_empty());
        assert!(parsed.show_tooltips);
        assert!(parsed.show_mixer_meters);
        assert!(parsed.mixer_meters_mono);
        assert_eq!(parsed.stems_format, "opus");
        assert_eq!(parsed.stems_model, analyzer_stems::DEFAULT_MODEL);
        assert!(!parsed.realtime_stems);
    }

    #[test]
    fn deck_profile_and_sampler_defaults_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.num_decks = 4;
        settings.deck_default_sampler_bank_id =
            vec![Some("a".into()), None, Some("c".into()), Some("d".into())];
        write_settings_file(&path, &settings).expect("write");

        let restored = settings_from_host(&load_host(&path));
        assert_eq!(restored.num_decks, 4);
        assert_eq!(
            restored.deck_default_sampler_bank_id,
            [Some("a".into()), None, Some("c".into()), Some("d".into())]
        );
    }

    #[test]
    fn missing_deck_profile_defaults_to_two_decks() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value.as_object_mut().expect("object").remove("num_decks");
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert_eq!(settings.num_decks, 2);
    }

    #[test]
    fn settings_reject_unsupported_deck_counts() {
        let mut settings = sample_settings();
        settings.num_decks = 3;
        assert!(parse_settings(settings).unwrap_err().contains("num_decks"));
    }

    #[test]
    fn legacy_backend_key_loads_and_is_dropped_on_save() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("backend".into(), serde_json::json!("cpal"));
        std::fs::write(&path, serde_json::to_vec_pretty(&value).expect("encode"))
            .expect("write legacy settings");

        let host = load_host(&path);
        assert!(host.configured);
        assert_eq!(settings_from_host(&host).master_bus.sample_rate, None);

        write_settings_file(&path, &settings_from_host(&host)).expect("save");
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(
            !text.contains("\"backend\""),
            "saved settings must drop the retired backend key: {text}"
        );
        let reloaded: AppSettings = serde_json::from_str(&text).expect("reload");
        assert_eq!(reloaded.master_bus.sample_rate, None);
    }

    #[test]
    fn settings_file_cannot_select_the_null_backend() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("backend".into(), serde_json::json!("null"));
        std::fs::write(&path, serde_json::to_vec_pretty(&value).expect("encode"))
            .expect("write legacy settings");

        let host = load_host(&path);
        assert_eq!(
            host.engine_config.backend, "cpal",
            "settings must not be able to select the headless null backend"
        );
    }

    #[test]
    fn missing_or_unknown_stems_model_defaults_htdemucs() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value.as_object_mut().expect("object").remove("stems_model");
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert_eq!(settings.stems_model, analyzer_stems::DEFAULT_MODEL);

        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("stems_model".into(), serde_json::json!("nope"));
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        let normalized = parse_settings(settings).expect("normalize");
        assert_eq!(normalized.stems_model, analyzer_stems::DEFAULT_MODEL);
    }

    #[test]
    fn missing_realtime_stems_defaults_disabled() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("realtime_stems");
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert!(!settings.realtime_stems);
    }

    #[test]
    fn missing_stems_format_defaults_opus() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("stems_format");
        std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
        let host = SettingsHost {
            persist_path: Some(path),
            ..Default::default()
        };
        // Load via serde of AppSettings missing field
        let raw = std::fs::read_to_string(host.persist_path.as_ref().unwrap()).unwrap();
        let settings: AppSettings = serde_json::from_str(&raw).unwrap();
        assert_eq!(settings.stems_format, "opus");
    }

    #[test]
    fn legacy_stems_enabled_key_is_ignored() {
        // Settings files written before stems became a standalone action still
        // carry `stems_enabled`. Loading must not fail and must not resurrect a
        // gate that no longer exists.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("stems_enabled".into(), serde_json::Value::Bool(true));
        std::fs::write(&path, serde_json::to_vec(&value).expect("write")).expect("disk");

        let host = load_host(&path);
        assert!(host.configured);
        assert_eq!(settings_from_host(&host).stems_format, "opus");
    }

    #[test]
    fn missing_library_row_density_defaults_comfortable() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("library_row_density");
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert_eq!(
            settings.library_row_density,
            LibraryRowDensitySetting::Comfortable
        );
    }

    #[test]
    fn library_row_density_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.library_row_density = LibraryRowDensitySetting::Compact;
        let mut host = SettingsHost {
            persist_path: Some(path.clone()),
            ..Default::default()
        };
        apply_to_host(&mut host, settings).expect("apply");
        let saved = parse_settings(settings_from_host(&host)).expect("parse");
        write_settings_file(&path, &saved).expect("write");
        assert_eq!(
            load_host(&path).library_row_density,
            LibraryRowDensitySetting::Compact
        );
    }

    #[test]
    fn missing_library_row_fields_default_to_todays_pills() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("library_row_fields");
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert_eq!(
            settings.library_row_fields,
            default_library_row_fields(),
            "a settings file written before the field existed keeps today's row"
        );
    }

    #[test]
    fn library_row_fields_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.library_row_fields = vec![
            LibraryRowFieldSetting::Isrc,
            LibraryRowFieldSetting::Duration,
            LibraryRowFieldSetting::Album,
        ];
        let mut host = SettingsHost {
            persist_path: Some(path.clone()),
            ..Default::default()
        };
        apply_to_host(&mut host, settings).expect("apply");
        let saved = parse_settings(settings_from_host(&host)).expect("parse");
        write_settings_file(&path, &saved).expect("write");
        assert_eq!(
            load_host(&path).library_row_fields,
            vec![
                LibraryRowFieldSetting::Isrc,
                LibraryRowFieldSetting::Duration,
                LibraryRowFieldSetting::Album,
            ],
            "render order is the saved order"
        );
    }

    #[test]
    fn unknown_library_row_field_names_are_dropped() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value.as_object_mut().expect("object").insert(
            "library_row_fields".into(),
            serde_json::json!(["isrc", "mood", 3, "album"]),
        );
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert_eq!(
            settings.library_row_fields,
            vec![LibraryRowFieldSetting::Isrc, LibraryRowFieldSetting::Album],
            "unknown names must not reset the whole field list"
        );
    }

    #[test]
    fn duplicate_library_row_fields_collapse_to_the_first_occurrence() {
        let mut settings = sample_settings();
        settings.library_row_fields = vec![
            LibraryRowFieldSetting::Bpm,
            LibraryRowFieldSetting::Album,
            LibraryRowFieldSetting::Bpm,
        ];
        let parsed = parse_settings(settings).expect("parse");
        assert_eq!(
            parsed.library_row_fields,
            vec![LibraryRowFieldSetting::Bpm, LibraryRowFieldSetting::Album]
        );
    }

    #[test]
    fn empty_library_row_fields_is_kept() {
        let mut settings = sample_settings();
        settings.library_row_fields.clear();
        let parsed = parse_settings(settings).expect("parse");
        assert!(
            parsed.library_row_fields.is_empty(),
            "hiding every field is a valid choice, not a missing value"
        );
    }

    #[test]
    fn tag_write_back_defaults_off_and_round_trips_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");

        // Off unless the user opts in, including for settings files written before
        // the field existed.
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("tag_write_back");
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert!(!settings.tag_write_back);

        let mut settings = sample_settings();
        settings.tag_write_back = true;
        let mut host = SettingsHost {
            persist_path: Some(path.clone()),
            ..Default::default()
        };
        apply_to_host(&mut host, settings).expect("apply");
        let saved = parse_settings(settings_from_host(&host)).expect("parse");
        assert!(saved.tag_write_back);
        write_settings_file(&path, &saved).expect("write");
        assert!(load_host(&path).tag_write_back);
    }

    #[test]
    fn legacy_library_table_columns_key_is_ignored() {
        // Settings files written while the track list was still a column table
        // carry `library_table_columns`. The key is gone from the DTO, so loading
        // must drop it silently rather than fail on an unknown field.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value.as_object_mut().expect("object").insert(
            "library_table_columns".into(),
            serde_json::json!(["title", "artist", "bpm"]),
        );
        std::fs::write(&path, serde_json::to_vec(&value).expect("write")).expect("disk");

        let host = load_host(&path);
        assert!(host.configured);
        let saved = parse_settings(settings_from_host(&host)).expect("parse");
        let reloaded = serde_json::to_value(&saved).expect("json");
        assert!(!reloaded
            .as_object()
            .expect("object")
            .contains_key("library_table_columns"));
    }

    #[test]
    fn missing_default_key_lock_defaults_false() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("default_key_lock");
        std::fs::write(&path, serde_json::to_vec(&value).expect("write")).expect("disk");
        let host = load_host(&path);
        assert!(!settings_from_host(&host).default_key_lock);
    }

    #[test]
    fn default_key_lock_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.default_key_lock = true;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(restored.default_key_lock);
    }

    #[test]
    fn missing_vinyl_brake_fields_default_off_and_half_second() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        let object = value.as_object_mut().expect("object");
        object.remove("vinyl_brake_pause");
        object.remove("vinyl_brake_stop_secs");
        std::fs::write(&path, serde_json::to_vec(&value).expect("write")).expect("disk");
        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(!restored.vinyl_brake_pause);
        assert!((restored.vinyl_brake_stop_secs - 0.5).abs() < 1e-6);
    }

    #[test]
    fn vinyl_brake_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.vinyl_brake_pause = true;
        settings.vinyl_brake_stop_secs = 1.25;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(restored.vinyl_brake_pause);
        assert!((restored.vinyl_brake_stop_secs - 1.25).abs() < 1e-6);
    }

    #[test]
    fn missing_show_tooltips_defaults_true() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("show_tooltips");
        std::fs::write(&path, serde_json::to_vec(&value).expect("write")).expect("disk");
        let host = load_host(&path);
        assert!(settings_from_host(&host).show_tooltips);
    }

    #[test]
    fn missing_mixer_meter_settings_default_shown_mono() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        let obj = value.as_object_mut().expect("object");
        obj.remove("show_mixer_meters");
        obj.remove("mixer_meters_mono");
        std::fs::write(&path, serde_json::to_vec(&value).expect("write")).expect("disk");
        let host = load_host(&path);
        let settings = settings_from_host(&host);
        assert!(settings.show_mixer_meters);
        assert!(settings.mixer_meters_mono);
    }

    #[test]
    fn mixer_meter_settings_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.show_mixer_meters = false;
        settings.mixer_meters_mono = false;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(!restored.show_mixer_meters);
        assert!(!restored.mixer_meters_mono);
    }

    #[test]
    fn show_tooltips_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.show_tooltips = false;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(!restored.show_tooltips);
    }

    #[test]
    fn missing_select_style_defaults_auto() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("select_style");
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert_eq!(settings.select_style, SelectStyleSetting::Auto);
    }

    #[test]
    fn select_style_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.select_style = SelectStyleSetting::Mobile;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert_eq!(restored.select_style, SelectStyleSetting::Mobile);
    }

    #[test]
    fn missing_library_drawer_defaults() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        let object = value.as_object_mut().expect("object");
        object.remove("library_drawer_enabled");
        object.remove("library_drawer_side");
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert!(!settings.library_drawer_enabled);
        assert_eq!(
            settings.library_drawer_side,
            LibraryDrawerSideSetting::Bottom
        );
    }

    #[test]
    fn library_drawer_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.library_drawer_enabled = true;
        settings.library_drawer_side = LibraryDrawerSideSetting::Left;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(restored.library_drawer_enabled);
        assert_eq!(restored.library_drawer_side, LibraryDrawerSideSetting::Left);
    }

    #[test]
    fn missing_ui_language_defaults_system() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value.as_object_mut().expect("object").remove("ui_language");
        std::fs::write(&path, serde_json::to_vec(&value).expect("write")).expect("disk");
        let host = load_host(&path);
        assert_eq!(
            settings_from_host(&host).ui_language,
            UiLanguageSetting::System
        );
    }

    #[test]
    fn ui_language_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.ui_language = UiLanguageSetting::PtBr;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert_eq!(restored.ui_language, UiLanguageSetting::PtBr);

        settings.ui_language = UiLanguageSetting::En;
        write_settings_file(&path, &settings).expect("write");
        let host = load_host(&path);
        assert_eq!(settings_from_host(&host).ui_language, UiLanguageSetting::En);
    }

    #[test]
    fn history_settings_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.history_enabled = false;
        settings.history_session_idle_minutes = 12;
        settings.history_min_play_seconds = 8;
        settings.history_min_deck_volume = 0.2;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(!restored.history_enabled);
        assert_eq!(restored.history_session_idle_minutes, 12);
        assert_eq!(restored.history_min_play_seconds, 8);
        assert!((restored.history_min_deck_volume - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn trusted_controller_device_ids_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.trusted_controller_device_ids = vec!["pioneer.ddj-400".into()];
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert_eq!(
            restored.trusted_controller_device_ids,
            vec!["pioneer.ddj-400".to_string()]
        );
    }

    #[test]
    fn missing_trusted_controller_device_ids_defaults_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("trusted_controller_device_ids");
        std::fs::write(&path, serde_json::to_vec(&value).expect("write")).expect("disk");
        let host = load_host(&path);
        assert!(settings_from_host(&host)
            .trusted_controller_device_ids
            .is_empty());
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
    fn settings_file_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.waveform_display_mode = WaveformDisplayModeSetting::Filtered;
        settings.master_bus.sample_rate = Some(44_100);
        write_settings_file(&path, &settings).expect("write");

        settings.master_bus.sample_rate = Some(48_000);
        settings.waveform_display_mode = WaveformDisplayModeSetting::Rgb;
        write_settings_file(&path, &settings).expect("write again");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert_eq!(
            restored.waveform_display_mode,
            WaveformDisplayModeSetting::Rgb
        );
        assert_eq!(restored.master_bus.sample_rate, Some(48_000));
    }

    #[test]
    fn save_settings_keeps_host_when_persist_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let transport = SettingsTransport {
            host: Arc::new(Mutex::new(SettingsHost {
                persist_path: Some(dir.path().to_path_buf()),
                ..SettingsHost::default()
            })),
        };
        let baseline = transport.get_settings().expect("baseline");
        let mut settings = sample_settings();
        settings.master_bus.sample_rate = Some(44_100);
        assert!(transport.save_settings(settings).is_err());
        assert_eq!(
            transport
                .get_settings()
                .expect("unchanged")
                .master_bus
                .sample_rate,
            baseline.master_bus.sample_rate
        );
    }

    #[test]
    fn legacy_global_low_latency_moves_to_both_buses() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("low_latency".into(), serde_json::Value::Bool(true));
        std::fs::write(&path, serde_json::to_vec(&value).expect("encode")).expect("write");

        let loaded = read_settings_file(&path).expect("legacy settings");
        assert!(loaded.master_bus.low_latency);
        assert!(loaded.preview_bus.low_latency);

        write_settings_file(&path, &loaded).expect("save");
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("reload");
        assert!(
            saved.get("low_latency").is_none(),
            "the legacy global flag must not be written back: {saved}"
        );
    }

    #[test]
    fn legacy_global_sample_rate_moves_to_the_master_bus() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("sample_rate".into(), serde_json::json!(44_100));
        std::fs::write(&path, serde_json::to_vec(&value).expect("encode")).expect("write");

        let loaded = read_settings_file(&path).expect("legacy settings");
        assert_eq!(loaded.master_bus.sample_rate, Some(44_100));
        // The old global described the engine stream; the preview bus may sit on a
        // device that cannot run it, so it is not pinned.
        assert_eq!(loaded.preview_bus.sample_rate, None);
    }

    #[test]
    fn legacy_default_sample_rate_does_not_pin_the_buses() {
        // 48 kHz was the built-in default and was written even when never chosen, so
        // migrating it would re-pin every install to the rate this change removes.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("sample_rate".into(), serde_json::json!(48_000));
        std::fs::write(&path, serde_json::to_vec(&value).expect("encode")).expect("write");

        let loaded = read_settings_file(&path).expect("legacy settings");
        assert_eq!(loaded.master_bus.sample_rate, None);
        assert_eq!(loaded.preview_bus.sample_rate, None);
    }

    #[test]
    fn legacy_scan_folder_tree_field_is_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("scan_folder_tree".into(), serde_json::Value::Bool(false));
        std::fs::write(&path, serde_json::to_vec(&value).expect("write")).expect("disk");
        let loaded = read_settings_file(&path).expect("read");
        let mut host = SettingsHost::default();
        apply_to_host(&mut host, loaded).expect("apply");
        let restored = settings_from_host(&host);
        assert_eq!(
            restored.master_bus.sample_rate,
            sample_settings().master_bus.sample_rate
        );
    }

    #[test]
    fn corrupt_settings_file_is_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        std::fs::write(&path, b"not json").expect("write");
        assert!(read_settings_file(&path).is_none());
        let host = load_host(&path);
        assert!(!host.configured);
        assert_eq!(settings_from_host(&host).master_bus.sample_rate, None);
    }
    #[test]
    fn legacy_settings_default_analytics_opt_out() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        let object = value.as_object_mut().expect("object");
        object.remove("analytics_enabled");
        let settings: AppSettings = serde_json::from_value(value).expect("legacy settings");
        assert!(!settings.analytics_enabled);
    }
}
