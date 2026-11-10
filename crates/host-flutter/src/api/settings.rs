//! Session settings host (mirrors Tauri `AppSettings` / `apply_settings`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::bridge::BridgeResult;
use crate::frb_generated::StreamSink;
use audio_core::{BusConfig, BusId, ChannelMapping, ChannelMode, DeviceId};
use bus_api::JogMode;
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

/// Realtime multiple a device must reach before Realtime Stems can be enabled.
///
/// `1.0` means the CPU keeps up exactly; the margin absorbs scheduler jitter and
/// the deck's own per-hop work, so a device reported at 0.97× is not offered the
/// feature even though it is close.
pub const REALTIME_STEMS_MIN_MULTIPLE: f64 = 1.1;

/// Device benchmark result backing the Realtime Stems gate.
///
/// Session-only: the probe answers whether this machine keeps up right now, so
/// the result is kept in memory and never written to settings or sent anywhere.
#[derive(Clone, Debug)]
pub struct RealtimeStemsCapability {
    /// Mean wall time of one 128-frame hop on the CPU EP.
    pub avg_hop_ms: f64,
    /// `hop budget / avg_hop_ms`.
    pub realtime: f64,
    /// Timed hops in the measurement (warm-up excluded).
    pub hops: u32,
    /// SHA-256 of the model that was measured.
    pub model_sha: String,
    /// `realtime >= REALTIME_STEMS_MIN_MULTIPLE`; the UI shows the switch
    /// only when true.
    pub allowed: bool,
}

/// Progress event for a running benchmark (hop counts, warm-up included).
#[derive(Clone, Debug)]
pub struct RealtimeStemsBenchmarkProgress {
    pub completed: u32,
    pub total: u32,
}

/// Whether an enable request may be honoured.
///
/// Only a *new* enable needs evidence: a preference already on (loaded from
/// settings) and a disable are always allowed, so a restart never re-prompts.
fn enable_allowed(
    was_enabled: bool,
    now_enabled: bool,
    capability: Option<&RealtimeStemsCapability>,
) -> bool {
    !now_enabled || was_enabled || capability.is_some_and(RealtimeStemsCapability::allows_live)
}

impl RealtimeStemsCapability {
    /// Whether a measured realtime multiple passes the gate.
    #[flutter_rust_bridge::frb(ignore)]
    pub fn realtime_allows(realtime: f64) -> bool {
        realtime.is_finite() && realtime >= REALTIME_STEMS_MIN_MULTIPLE
    }

    /// Whether this measured device may enable Realtime Stems.
    #[flutter_rust_bridge::frb(ignore)]
    pub fn allows_live(&self) -> bool {
        Self::realtime_allows(self.realtime)
    }

    #[cfg(feature = "stems")]
    fn from_benchmark(benchmark: analyzer_stems::HopBenchmark, model_sha: String) -> Self {
        Self {
            avg_hop_ms: benchmark.avg_hop_ms,
            realtime: benchmark.realtime,
            hops: benchmark.hops,
            model_sha,
            allowed: Self::realtime_allows(benchmark.realtime),
        }
    }
}

/// SHA-256 of the StemgenRT artifact this build measures.
#[cfg(feature = "stems")]
fn stemgen_model_sha() -> Option<String> {
    let manifest = analyzer_stems::builtin_manifest(analyzer_stems::STEMGEN_MODEL).ok()?;
    let artifact = manifest.resolve_primary_artifact().ok()?;
    Some(artifact.sha256.clone())
}

/// Ensure the model, time the hops, and store the verdict for this session.
#[cfg(feature = "stems")]
fn run_realtime_probe(
    models_root: &Path,
    on_progress: Option<&dyn Fn(u32, u32)>,
) -> Result<RealtimeStemsCapability, String> {
    analyzer_stems::ensure_model(models_root, analyzer_stems::STEMGEN_MODEL).bridge()?;
    let model_sha =
        stemgen_model_sha().ok_or_else(|| "StemgenRT manifest has no artifact".to_string())?;
    let handle = analyzer_stems::cached_model(models_root, analyzer_stems::STEMGEN_MODEL)
        .bridge()?
        .ok_or_else(|| "StemgenRT model is missing after download".to_string())?;

    let benchmark = analyzer_stems::benchmark_stemgen(
        &handle.local_path,
        analyzer_stems::DEFAULT_BENCH_HOPS,
        analyzer_stems::WARMUP_HOPS,
        on_progress,
    )
    .bridge()?;
    let capability = RealtimeStemsCapability::from_benchmark(benchmark, model_sha);
    store_realtime_capability(capability.clone())?;
    Ok(capability)
}

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
        // Turning the live filter on is the gated action. Once enabled it stays
        // enabled across launches without re-benchmarking; the dry-mix fallback
        // covers a machine that cannot keep up later.
        if !enable_allowed(
            host.realtime_stems,
            settings.realtime_stems,
            host.realtime_capability.as_ref(),
        ) {
            return Err(
                "Realtime Stems needs a passing benchmark before it can be enabled.".to_string(),
            );
        }
        let mut candidate = host.clone();
        apply_to_host(&mut candidate, settings)?;
        let saved = parse_settings(settings_from_host(&candidate))?;
        if let Some(path) = host.persist_path.clone() {
            write_settings_file(&path, &saved)?;
        }
        *host = candidate;
        Ok(saved)
    }

    /// Latest device benchmark for this session.
    ///
    /// `None` until the probe has run; the result is not persisted, so every
    /// launch starts unmeasured.
    pub fn realtime_stems_capability(&self) -> Result<Option<RealtimeStemsCapability>, String> {
        let host = self.host.lock().bridge()?;
        Ok(host.realtime_capability.clone())
    }

    /// Measure this device's StemgenRT hop cost on the CPU EP.
    ///
    /// Ensures the model is on disk first (downloading it if needed), then runs
    /// a warm-up plus `DEFAULT_BENCH_HOPS` timed hops on a private CPU session.
    /// It never starts the live producer and never touches playback state, and
    /// it runs on the FRB worker thread rather than the audio callback.
    ///
    /// `sink` receives hop progress; when the stream completes, the result is
    /// available from [`Self::realtime_stems_capability`]. Nothing is persisted:
    /// the verdict lives for this session, and a newer run replaces it.
    #[cfg(feature = "stems")]
    pub fn benchmark_realtime_stems(
        &self,
        sink: StreamSink<RealtimeStemsBenchmarkProgress>,
    ) -> Result<(), String> {
        let models_root = crate::app_paths::ensure().join("models");
        run_realtime_probe(
            &models_root,
            Some(&|completed, total| {
                // A hop costs a few milliseconds; a message per hop would flood
                // the FFI channel. Report in tenths and always finish on the
                // last hop so the bar lands at 100%.
                if completed == total || completed % 10 == 0 {
                    let _ = sink.add(RealtimeStemsBenchmarkProgress { completed, total });
                }
            }),
        )?;
        Ok(())
    }

    /// Realtime Stems is CPU-only and built without ONNX Runtime on this target.
    #[cfg(not(feature = "stems"))]
    pub fn benchmark_realtime_stems(
        &self,
        _sink: StreamSink<RealtimeStemsBenchmarkProgress>,
    ) -> Result<(), String> {
        Err("Realtime Stems is not available in this build (no ONNX Runtime)".to_string())
    }

    /// Replace the live controller audio override without touching disk.
    ///
    /// Callers restart the engine afterwards so the effective buses change.
    pub fn set_controller_audio_override(
        &self,
        override_settings: Option<ControllerAudioOverride>,
    ) -> Result<AppSettings, String> {
        let mut host = self.host.lock().bridge()?;
        let mut candidate = host.clone();
        candidate.controller_audio_override = override_settings;
        let saved = parse_settings(settings_from_host(&candidate))?;
        *host = candidate;
        Ok(saved)
    }

    /// Clear the live override only when [mapping_id] still owns it.
    ///
    /// Avoids a read-then-clear race when another controller switched audio in
    /// between the caller reading settings and clearing.
    pub fn clear_controller_audio_override(
        &self,
        mapping_id: String,
    ) -> Result<AppSettings, String> {
        let mut host = self.host.lock().bridge()?;
        if host
            .controller_audio_override
            .as_ref()
            .is_some_and(|override_settings| override_settings.mapping_id == mapping_id)
        {
            host.controller_audio_override = None;
        }
        parse_settings(settings_from_host(&host))
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
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// A controller mapping's live output override.
///
/// This is kept in memory only. The saved master/preview routes are never
/// replaced, so clearing the override simply resumes the saved configuration.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ControllerAudioOverride {
    /// App-data mapping folder id that owns this override.
    pub mapping_id: String,
    /// Controller label for the Settings badge (e.g. `DDJ-400`).
    pub controller_label: String,
    /// OS output device id the buses are running on.
    pub output_device_id: String,
    pub master_overridden: bool,
    pub master_bus: BusRouteSettings,
    pub preview_overridden: bool,
    pub preview_bus: BusRouteSettings,
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

/// App colour theme preference (Settings → UI). `System` follows the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeSetting {
    #[default]
    System,
    Light,
    Dark,
}

/// Accent palette for brand chrome (Settings → UI). Teal is the website
/// `--accent` hue and stays the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccentPaletteSetting {
    #[default]
    Teal,
    Blue,
    Violet,
    Orange,
    Pink,
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

/// Where the deck waveforms sit (Settings → Layout).
///
/// `top` is the full-width pane above the deck row, `bottom` the horizontal
/// band between the deck row and the library, and the two middle positions
/// replace the center mixer with the lanes inside the deck row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveformPositionSetting {
    #[default]
    Top,
    MiddleHorizontal,
    MiddleVertical,
    Bottom,
}

/// Deck VU meters between the mixer volume faders (Settings → Layout).
///
/// One value instead of a show flag plus a mono flag, so the Settings select can
/// offer `None` beside the two widths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MixerMetersSetting {
    /// No deck meters; the channel columns drop the meter lane.
    None,
    /// One ladder per deck, max of left/right.
    #[default]
    Mono,
    /// Separate left and right ladders per deck.
    Stereo,
}

fn default_vinyl_brake_stop_secs() -> f32 {
    0.5
}
fn default_show_waveform() -> bool {
    true
}
fn default_waveform_gain_eq_shaping() -> bool {
    true
}
fn default_show_mixer() -> bool {
    true
}
fn default_show_fx_bar() -> bool {
    true
}
fn default_show_crossfader() -> bool {
    true
}
fn default_num_decks() -> u8 {
    2
}
fn default_crossfader_curve() -> String {
    bus_api::DEFAULT_CROSSFADER_CURVE_ID.to_string()
}
fn default_tempo_curve() -> String {
    bus_api::DEFAULT_TEMPO_CURVE_ID.to_string()
}
fn default_fx_level_curve() -> String {
    bus_api::DEFAULT_FX_LEVEL_CURVE_ID.to_string()
}

/// Crossfader preset option for the Settings curve picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossfaderPresetOption {
    /// Canonical snake_case id (the value stored in settings).
    pub id: String,
    /// Human-readable name for the dropdown.
    pub display_name: String,
    /// Reserved names ship in the catalog but cannot be selected yet.
    pub available: bool,
}

/// Every crossfader preset the Settings dropdown lists, in catalog order.
///
/// Canonical ids are the only values settings persist; reserved names appear
/// disabled instead of being silently omitted.
#[flutter_rust_bridge::frb(sync)]
pub fn crossfader_preset_options() -> Vec<CrossfaderPresetOption> {
    engine_core::crossfader_preset_catalog()
        .iter()
        .map(|preset| CrossfaderPresetOption {
            id: preset.id.to_string(),
            display_name: preset.display_name.to_string(),
            available: preset.available,
        })
        .collect()
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
    /// Default tempo-fader curve id applied to every deck on engine start.
    #[serde(default = "default_tempo_curve")]
    pub default_tempo_curve: String,
    /// Default FX level/depth curve id applied on every engine start.
    #[serde(default = "default_fx_level_curve")]
    pub default_fx_level_curve: String,
    /// Default crossfader curve id applied on every engine start.
    #[serde(default = "default_crossfader_curve")]
    pub default_crossfader_curve: String,
    /// Curve bypass and hamster flags applied on every engine start.
    #[serde(default)]
    pub crossfader_behavior: bus_api::CrossfaderBehaviorSetting,
    /// Named custom curve groups and the curve sets a knob can sweep.
    #[serde(default)]
    pub curve_library: bus_api::CurveLibrarySpec,
    #[serde(default)]
    pub default_key_lock: bool,
    #[serde(default)]
    pub vinyl_brake_pause: bool,
    #[serde(default = "default_vinyl_brake_stop_secs")]
    pub vinyl_brake_stop_secs: f32,
    pub waveform_display_mode: WaveformDisplayModeSetting,
    /// Shape the scrolling lane with the deck's channel gain and EQ
    /// (Settings → Waveform). Off draws the recorded peaks at unity, so the
    /// lane stays still while the knobs move.
    #[serde(default = "default_waveform_gain_eq_shaping")]
    pub waveform_gain_eq_shaping: bool,
    #[serde(default)]
    pub key_display_mode: KeyDisplayModeSetting,
    #[serde(default)]
    pub key_color_mode: KeyColorModeSetting,
    #[serde(default)]
    pub trusted_controller_device_ids: Vec<String>,
    /// Controllers whose audio interface is switched in automatically on attach.
    #[serde(default)]
    pub always_switch_audio_device_ids: Vec<String>,
    /// Live controller audio override; memory only, never persisted.
    #[serde(skip)]
    pub controller_audio_override: Option<ControllerAudioOverride>,
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
    /// Deck VU meters between the mixer volume faders.
    #[serde(default)]
    pub mixer_meters: MixerMetersSetting,
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
    ///
    /// Gated when it is turned on: enabling requires a session benchmark at
    /// realtime with margin. A saved opt-in is honoured on later launches
    /// without re-running the probe.
    #[serde(default)]
    pub realtime_stems: bool,
    #[serde(default)]
    pub ui_language: UiLanguageSetting,
    /// App colour theme. `system` follows the host's light/dark preference.
    #[serde(default)]
    pub theme: ThemeSetting,
    /// Brand accent palette; only chrome re-tints, deck colours stay put.
    #[serde(default)]
    pub accent_palette: AccentPaletteSetting,
    /// Open the library as an overlay drawer instead of the docked panel.
    #[serde(default)]
    pub library_drawer_enabled: bool,
    /// Edge the drawer opens from.
    #[serde(default)]
    pub library_drawer_side: LibraryDrawerSideSetting,
    /// Where the deck waveforms sit: above the decks, below them, or in the
    /// middle of the deck row in place of the mixer.
    #[serde(default)]
    pub waveform_position: WaveformPositionSetting,
    /// Draw the deck waveforms at all (Settings → Layout).
    #[serde(default = "default_show_waveform")]
    pub show_waveform: bool,
    /// Show the channel strip between the decks (Settings → Layout). The decks
    /// themselves stay visible either way.
    #[serde(default = "default_show_mixer")]
    pub show_mixer: bool,
    /// Show the FX bar at the foot of the mixer pane (Settings → Layout).
    #[serde(default = "default_show_fx_bar")]
    pub show_fx_bar: bool,
    /// Show the crossfader at the foot of the mixer strip (Settings → Layout).
    #[serde(default = "default_show_crossfader")]
    pub show_crossfader: bool,
    #[serde(default)]
    pub analytics_enabled: bool,
    /// First-run setup finished or skipped. Fresh installs start `false`.
    #[serde(default)]
    pub onboarding_completed: bool,
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
    waveform_gain_eq_shaping: bool,
    key_display_mode: KeyDisplayModeSetting,
    key_color_mode: KeyColorModeSetting,
    trusted_controller_device_ids: Vec<String>,
    always_switch_audio_device_ids: Vec<String>,
    controller_audio_override: Option<ControllerAudioOverride>,
    history_enabled: bool,
    history_session_idle_minutes: u32,
    history_min_play_seconds: u32,
    history_min_deck_volume: f32,
    show_tooltips: bool,
    mixer_meters: MixerMetersSetting,
    select_style: SelectStyleSetting,
    dim_played_tracks: bool,
    stems_format: String,
    stems_model: String,
    realtime_stems: bool,
    /// Last CPU benchmark backing the Realtime Stems gate.
    realtime_capability: Option<RealtimeStemsCapability>,
    ui_language: UiLanguageSetting,
    theme: ThemeSetting,
    accent_palette: AccentPaletteSetting,
    library_drawer_enabled: bool,
    library_drawer_side: LibraryDrawerSideSetting,
    waveform_position: WaveformPositionSetting,
    show_waveform: bool,
    show_mixer: bool,
    show_fx_bar: bool,
    show_crossfader: bool,
    analytics_enabled: bool,
    onboarding_completed: bool,
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
            waveform_gain_eq_shaping: default_waveform_gain_eq_shaping(),
            key_display_mode: KeyDisplayModeSetting::Musical,
            key_color_mode: KeyColorModeSetting::Off,
            trusted_controller_device_ids: Vec::new(),
            always_switch_audio_device_ids: Vec::new(),
            controller_audio_override: None,
            history_enabled: default_history_enabled(),
            history_session_idle_minutes: default_history_session_idle_minutes(),
            history_min_play_seconds: default_history_min_play_seconds(),
            history_min_deck_volume: default_history_min_deck_volume(),
            show_tooltips: default_show_tooltips(),
            mixer_meters: MixerMetersSetting::default(),
            select_style: SelectStyleSetting::default(),
            dim_played_tracks: default_dim_played_tracks(),
            stems_format: default_stems_format(),
            stems_model: default_stems_model(),
            realtime_stems: false,
            realtime_capability: None,
            ui_language: UiLanguageSetting::default(),
            theme: ThemeSetting::default(),
            accent_palette: AccentPaletteSetting::default(),
            library_drawer_enabled: false,
            library_drawer_side: LibraryDrawerSideSetting::Bottom,
            waveform_position: WaveformPositionSetting::Top,
            show_waveform: default_show_waveform(),
            show_mixer: default_show_mixer(),
            show_fx_bar: default_show_fx_bar(),
            show_crossfader: default_show_crossfader(),
            analytics_enabled: false,
            onboarding_completed: false,
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

fn default_dim_played_tracks() -> bool {
    true
}

fn default_stems_format() -> String {
    "opus".into()
}

fn default_stems_model() -> String {
    library::DEFAULT_STEMS_MODEL.into()
}

fn normalize_stems_model(model: &str) -> String {
    let model = model.trim();
    // The catalog lives in the library crate so it stays available in builds
    // without the `stems` feature (no ONNX Runtime, e.g. Android x86_64).
    if library::STEM_MODELS.contains(&model) {
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
            default_crossfader_curve: Some(bus_api::DEFAULT_CROSSFADER_CURVE_ID.to_string()),
            default_channel_fader_curve: None,
            default_tempo_curve: Some(bus_api::DEFAULT_TEMPO_CURVE_ID.to_string()),
            default_fx_level_curve: Some(bus_api::DEFAULT_FX_LEVEL_CURVE_ID.to_string()),
            crossfader_behavior: Some(bus_api::CrossfaderBehaviorSetting::default()),
            curve_library: Some(bus_api::CurveLibrarySpec::default()),
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
    if let Some(override_settings) = &settings.controller_audio_override {
        if override_settings.mapping_id.trim().is_empty() {
            errors.push("controller_audio_override.mapping_id must be non-empty".into());
        }
        if override_settings.output_device_id.trim().is_empty() {
            errors.push("controller_audio_override.output_device_id must be non-empty".into());
        }
        if override_settings.controller_label.trim().is_empty() {
            errors.push("controller_audio_override.controller_label must be non-empty".into());
        }
        if override_settings.master_overridden
            && override_settings.master_bus.device_id != override_settings.output_device_id
        {
            errors.push(
                "controller_audio_override.master_bus must target its output_device_id".into(),
            );
        }
        if override_settings.preview_overridden
            && override_settings.preview_bus.device_id != override_settings.output_device_id
        {
            errors.push(
                "controller_audio_override.preview_bus must target its output_device_id".into(),
            );
        }
        if let Err(e) = parse_bus(
            "controller_audio_override.master_bus",
            &override_settings.master_bus,
        ) {
            errors.push(e);
        }
        if let Err(e) = parse_bus(
            "controller_audio_override.preview_bus",
            &override_settings.preview_bus,
        ) {
            errors.push(e);
        }
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

    // Preset aliases ("Constant Power") are accepted on the way in but always
    // stored as canonical snake_case. An unknown id is coerced to the equal-power
    // default (like a retired stem model), so a stale settings file still loads.
    let requested = settings.default_crossfader_curve.trim().to_string();
    let canonical = engine_core::curve_presets::canonical_id(&requested).map(str::to_string);
    let known_custom = settings
        .curve_library
        .groups
        .iter()
        .any(|group| group.id == requested)
        || settings
            .curve_library
            .sets
            .iter()
            .any(|set| set.id == requested);
    settings.default_crossfader_curve = match canonical {
        Some(id) => id,
        None if known_custom => requested,
        None => default_crossfader_curve(),
    };
    let requested_tempo = settings.default_tempo_curve.trim().to_string();
    settings.default_tempo_curve = if requested_tempo.is_empty()
        || requested_tempo.eq_ignore_ascii_case(bus_api::DEFAULT_TEMPO_CURVE_ID)
    {
        default_tempo_curve()
    } else {
        requested_tempo
    };
    let requested_fx_level = settings.default_fx_level_curve.trim().to_string();
    settings.default_fx_level_curve = if requested_fx_level.is_empty()
        || requested_fx_level.eq_ignore_ascii_case(bus_api::DEFAULT_FX_LEVEL_CURVE_ID)
    {
        default_fx_level_curve()
    } else {
        requested_fx_level
    };
    if let Err(error) = engine_core::library_from_spec(&settings.curve_library) {
        errors.push(format!("curve_library: {error}"));
    }

    // The preference list drives auto-switch; duplicates add no meaning.
    let mut audio_ids = std::collections::HashSet::new();
    settings
        .always_switch_audio_device_ids
        .retain(|id| !id.trim().is_empty() && audio_ids.insert(id.clone()));

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
        // Report the live state so the Settings switch reflects the controller
        // override; the saved config keeps its own preview flag.
        preview_enabled: preview_bus_config.is_some()
            || host
                .controller_audio_override
                .as_ref()
                .is_some_and(|override_settings| override_settings.preview_overridden),
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
        default_tempo_curve: config.default_tempo_curve(),
        default_fx_level_curve: config.default_fx_level_curve(),
        default_crossfader_curve: config.default_crossfader_curve(),
        crossfader_behavior: config.crossfader_behavior(),
        curve_library: config.curve_library(),
        default_key_lock: config.default_key_lock(),
        vinyl_brake_pause: config.vinyl_brake_pause(),
        vinyl_brake_stop_secs: config.vinyl_brake_stop_secs(),
        waveform_display_mode: host.waveform_display_mode,
        waveform_gain_eq_shaping: host.waveform_gain_eq_shaping,
        key_display_mode: host.key_display_mode,
        key_color_mode: host.key_color_mode,
        trusted_controller_device_ids: host.trusted_controller_device_ids.clone(),
        always_switch_audio_device_ids: host.always_switch_audio_device_ids.clone(),
        controller_audio_override: host.controller_audio_override.clone(),
        history_enabled: host.history_enabled,
        history_session_idle_minutes: host.history_session_idle_minutes,
        history_min_play_seconds: host.history_min_play_seconds,
        history_min_deck_volume: host.history_min_deck_volume,
        show_tooltips: host.show_tooltips,
        mixer_meters: host.mixer_meters,
        select_style: host.select_style,
        dim_played_tracks: host.dim_played_tracks,
        stems_format: host.stems_format.clone(),
        stems_model: host.stems_model.clone(),
        realtime_stems: host.realtime_stems,
        ui_language: host.ui_language,
        theme: host.theme,
        accent_palette: host.accent_palette,
        library_drawer_enabled: host.library_drawer_enabled,
        library_drawer_side: host.library_drawer_side,
        waveform_position: host.waveform_position,
        show_waveform: host.show_waveform,
        show_mixer: host.show_mixer,
        show_fx_bar: host.show_fx_bar,
        show_crossfader: host.show_crossfader,
        analytics_enabled: host.analytics_enabled,
        onboarding_completed: host.onboarding_completed,
    }
}

fn apply_to_host(host: &mut SettingsHost, settings: AppSettings) -> Result<(), String> {
    let settings = parse_settings(settings)?;
    let mut config = host.engine_config.clone();
    config.num_decks = usize::from(settings.num_decks);
    config.buses = buses_from_settings(&settings);
    config.buffer_size = settings.buffer_size;
    config.analysis_duration = settings.analysis_duration.into();
    // The GUI does not edit the channel-fader curve yet; keep a headless or
    // hand-edited value instead of dropping it on every settings save.
    let default_channel_fader_curve = config
        .audio
        .as_ref()
        .and_then(|audio| audio.default_channel_fader_curve.clone());
    config.audio = Some(AudioConfig {
        resampler_quality: Some(settings.resampler_quality.clone()),
        sampler_strip_route: Some(settings.sampler_strip_route.into()),
        default_tempo_range: Some(settings.default_tempo_range),
        tempo_range_steps: Some(settings.tempo_range_steps.clone()),
        default_key_lock: Some(settings.default_key_lock),
        vinyl_brake_pause: Some(settings.vinyl_brake_pause),
        vinyl_brake_stop_secs: Some(settings.vinyl_brake_stop_secs),
        default_crossfader_curve: Some(settings.default_crossfader_curve.clone()),
        default_channel_fader_curve,
        default_tempo_curve: Some(settings.default_tempo_curve.clone()),
        default_fx_level_curve: Some(settings.default_fx_level_curve.clone()),
        crossfader_behavior: Some(settings.crossfader_behavior),
        curve_library: Some(settings.curve_library.clone()),
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
    host.waveform_gain_eq_shaping = settings.waveform_gain_eq_shaping;
    host.key_display_mode = settings.key_display_mode;
    host.key_color_mode = settings.key_color_mode;
    host.trusted_controller_device_ids = settings.trusted_controller_device_ids;
    host.always_switch_audio_device_ids = settings.always_switch_audio_device_ids;
    // The Flutter draft carries the live override, including buses the user
    // edited back to their saved route; applying it keeps the engine and the
    // badge in step. The field itself is never serialized.
    host.controller_audio_override = settings.controller_audio_override;
    host.history_enabled = settings.history_enabled;
    host.history_session_idle_minutes = settings.history_session_idle_minutes;
    host.history_min_play_seconds = settings.history_min_play_seconds;
    host.history_min_deck_volume = settings.history_min_deck_volume;
    host.show_tooltips = settings.show_tooltips;
    host.mixer_meters = settings.mixer_meters;
    host.select_style = settings.select_style;
    host.dim_played_tracks = settings.dim_played_tracks;
    host.stems_format = settings.stems_format;
    host.stems_model = settings.stems_model;
    // The opt-in is a preference. It is gated when it is turned on
    // (`save_settings`), not on every load, so a restart keeps the feature on
    // without prompting for another benchmark.
    host.realtime_stems = settings.realtime_stems;
    host.ui_language = settings.ui_language;
    host.theme = settings.theme;
    host.accent_palette = settings.accent_palette;
    host.library_drawer_enabled = settings.library_drawer_enabled;
    host.analytics_enabled = settings.analytics_enabled;
    host.onboarding_completed = settings.onboarding_completed;
    host.library_drawer_side = settings.library_drawer_side;
    host.waveform_position = settings.waveform_position;
    host.show_waveform = settings.show_waveform;
    host.show_mixer = settings.show_mixer;
    host.show_fx_bar = settings.show_fx_bar;
    host.show_crossfader = settings.show_crossfader;
    if !settings.preview_enabled {
        if let Some(override_settings) = &mut host.controller_audio_override {
            override_settings.preview_overridden = false;
        }
    }
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

    // A file written before the move has no per-bus rate/flag keys at all; one
    // written after it always has both (serialised, possibly null/false). Judging
    // that per file keeps a retired global from overwriting a route the new format
    // already stored, while still migrating genuinely old files.
    let structured_buses = ["master_bus", "preview_bus"].iter().any(|bus| {
        value.get(bus).is_some_and(|route| {
            route.get("low_latency").is_some() || route.get("sample_rate").is_some()
        })
    });

    let legacy_low_latency = value
        .get("low_latency")
        .and_then(serde_json::Value::as_bool);
    let legacy_sample_rate = value
        .get("sample_rate")
        .and_then(serde_json::Value::as_u64)
        .and_then(|rate| u32::try_from(rate).ok());

    let has_meter_mode = value.get("mixer_meters").is_some();
    let legacy_show_meters = value
        .get("show_mixer_meters")
        .and_then(serde_json::Value::as_bool);
    let legacy_meters_mono = value
        .get("mixer_meters_mono")
        .and_then(serde_json::Value::as_bool);

    let mut settings: AppSettings = serde_json::from_value(value).ok()?;

    // `show_mixer_meters` + `mixer_meters_mono` collapsed into one mode. Only a
    // file written before the collapse carries the pair, so the new key wins.
    if !has_meter_mode {
        if let Some(show) = legacy_show_meters {
            settings.mixer_meters = match (show, legacy_meters_mono) {
                (false, _) => MixerMetersSetting::None,
                (true, Some(false)) => MixerMetersSetting::Stereo,
                (true, _) => MixerMetersSetting::Mono,
            };
        }
    }

    if structured_buses {
        return Some(settings);
    }

    // `low_latency` used to be app-wide, and `true` was always an explicit choice.
    if legacy_low_latency == Some(true) {
        settings.master_bus.low_latency = true;
        settings.preview_bus.low_latency = true;
    }
    // `sample_rate` used to be app-wide. 48000 was the built-in default and was
    // written even when never chosen, so migrating it would pin every existing
    // install to 48 kHz — the behaviour this change removes. Any other value was a
    // real choice, kept on the master bus only: the old global described the
    // engine's stream, and the preview bus may sit on a device that cannot run it.
    if let Some(rate) =
        legacy_sample_rate.filter(|rate| *rate > 0 && *rate != LEGACY_DEFAULT_SAMPLE_RATE)
    {
        settings.master_bus.sample_rate = Some(rate);
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
///
/// The saved `engine_config` remains the base; a live controller override
/// replaces only the buses it owns, so clearing it needs no disk write.
pub(crate) fn settings_engine_config() -> Result<EngineConfig, String> {
    let host = shared_host();
    let host = host.lock().bridge()?;
    Ok(effective_engine_config(&host))
}

fn effective_engine_config(host: &SettingsHost) -> EngineConfig {
    let Some(override_settings) = &host.controller_audio_override else {
        return host.engine_config.clone();
    };
    let mut config = host.engine_config.clone();
    if override_settings.master_overridden {
        replace_bus(
            &mut config.buses,
            MASTER_BUS_ID,
            "Master",
            &override_settings.master_bus,
        );
    }
    if override_settings.preview_overridden {
        // A mapping's headphone route intentionally owns the preview bus even
        // when the saved config had preview disabled.
        replace_bus(
            &mut config.buses,
            PREVIEW_BUS_ID,
            "Preview",
            &override_settings.preview_bus,
        );
    }
    config
}

fn replace_bus(buses: &mut Vec<BusConfig>, id: &str, name: &str, route: &BusRouteSettings) {
    let replacement = bus_config(id, name, route);
    if let Some(position) = buses.iter().position(|bus| bus.id.as_str() == id) {
        buses[position] = replacement;
    } else {
        // A controller's headphone route can intentionally turn on a preview
        // bus that the saved config had disabled, so an absent bus is valid.
        buses.push(replacement);
    }
}

/// Whether the opt-in live StemgenRT filter is on for this session.
///
/// Defaults to `false`; a poisoned host lock is treated as disabled so the
/// audio/load path never starts live separation unexpectedly. The opt-in is
/// gated when it is turned on (`save_settings`); once saved it is honoured on
/// later launches without re-running the benchmark.
#[cfg(feature = "stems")]
pub(crate) fn settings_realtime_stems_enabled() -> bool {
    let host = shared_host();
    host.lock()
        .map(|guard| guard.realtime_stems)
        .unwrap_or(false)
}

/// Keep a fresh benchmark for this session.
///
/// The saved preference is left alone: `settings_realtime_stems_enabled` reads
/// the verdict at the live path, so a failing run stops production without
/// silently turning the user's switch off.
#[cfg(feature = "stems")]
fn store_realtime_capability(capability: RealtimeStemsCapability) -> Result<(), String> {
    let host = shared_host();
    let mut guard = host.lock().bridge()?;
    guard.realtime_capability = Some(capability);
    Ok(())
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

    fn controller_route(left: u16, right: u16) -> BusRouteSettings {
        BusRouteSettings {
            device_id: "cpal:ddj".into(),
            left_channel: left,
            right_channel: right,
            mode: BusChannelMode::Stereo,
            sample_rate: None,
            low_latency: false,
        }
    }

    fn controller_override(
        mapping_id: &str,
        master_overridden: bool,
        preview_overridden: bool,
    ) -> ControllerAudioOverride {
        ControllerAudioOverride {
            mapping_id: mapping_id.into(),
            controller_label: "DDJ-400".into(),
            output_device_id: "cpal:ddj".into(),
            master_overridden,
            master_bus: if master_overridden {
                controller_route(1, 2)
            } else {
                default_master_bus_route()
            },
            preview_overridden,
            preview_bus: if preview_overridden {
                controller_route(3, 4)
            } else {
                default_preview_bus_route()
            },
        }
    }

    fn sample_settings() -> AppSettings {
        settings_from_host(&SettingsHost::default())
    }

    /// `sample_settings()` as JSON with one key removed, for default-value
    /// tests on files written by older builds.
    fn settings_without_key(key: &str) -> serde_json::Value {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value.as_object_mut().expect("object").remove(key);
        value
    }

    /// A file from the two-flag era: no `mixer_meters`, but both legacy keys.
    fn write_legacy_meter_file(path: &Path, show: bool, mono: bool) {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        let object = value.as_object_mut().expect("object");
        object.remove("mixer_meters");
        object.insert("show_mixer_meters".into(), show.into());
        object.insert("mixer_meters_mono".into(), mono.into());
        std::fs::write(path, serde_json::to_vec(&value).expect("write")).expect("disk");
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
        assert_eq!(parsed.mixer_meters, MixerMetersSetting::Mono);
        assert_eq!(parsed.stems_format, "opus");
        assert_eq!(parsed.stems_model, library::DEFAULT_STEMS_MODEL);
        assert!(!parsed.realtime_stems);
        assert!(parsed.waveform_gain_eq_shaping);
    }

    #[test]
    fn controller_audio_override_is_not_written_to_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let base = sample_settings();
        let mut settings = base.clone();
        settings.controller_audio_override = Some(controller_override("ddj-400", true, true));

        write_settings_file(&path, &settings).expect("write");
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("json");
        assert!(
            json.get("controller_audio_override").is_none(),
            "runtime override must never reach disk"
        );

        // The saved routes are untouched, and a restart has no override.
        let restored = settings_from_host(&load_host(&path));
        assert_eq!(restored.controller_audio_override, None);
        assert_eq!(restored.master_bus, base.master_bus);
    }

    #[test]
    fn controller_audio_override_affects_the_effective_engine_config() {
        let host = SettingsHost {
            controller_audio_override: Some(controller_override("ddj-400", true, false)),
            ..Default::default()
        };
        let effective = effective_engine_config(&host);
        let master = effective
            .buses
            .iter()
            .find(|bus| bus.id.as_str() == MASTER_BUS_ID)
            .expect("master bus");
        assert_eq!(master.device.as_str(), "cpal:ddj");
        // The saved base config stays on its device.
        let saved_master = host
            .engine_config
            .buses
            .iter()
            .find(|bus| bus.id.as_str() == MASTER_BUS_ID)
            .expect("saved master bus");
        assert_eq!(saved_master.device.as_str(), "default");
    }

    #[test]
    fn settings_save_can_clear_the_runtime_override() {
        let mut host = SettingsHost {
            controller_audio_override: Some(controller_override("ddj-400", true, true)),
            ..Default::default()
        };
        let mut settings = settings_from_host(&host);
        assert!(settings.controller_audio_override.is_some());
        settings.controller_audio_override = None;
        apply_to_host(&mut host, settings).expect("clear override");
        assert!(host.controller_audio_override.is_none());
    }

    #[test]
    fn preview_override_adds_a_disabled_preview_bus() {
        let host = SettingsHost {
            controller_audio_override: Some(controller_override("ddj-400", false, true)),
            ..Default::default()
        };
        let effective = effective_engine_config(&host);
        let preview = effective
            .buses
            .iter()
            .find(|bus| bus.id.as_str() == PREVIEW_BUS_ID)
            .expect("preview bus is created");
        assert_eq!(preview.device.as_str(), "cpal:ddj");
    }

    #[test]
    fn unrelated_save_keeps_the_preview_override() {
        let mut host = SettingsHost {
            controller_audio_override: Some(controller_override("ddj-400", false, true)),
            ..Default::default()
        };
        let mut settings = settings_from_host(&host);
        assert!(settings.preview_enabled, "preview reports the live state");
        settings.buffer_size = 1024;
        apply_to_host(&mut host, settings).expect("unrelated save");
        assert!(
            host.controller_audio_override
                .expect("override kept")
                .preview_overridden
        );
    }

    #[test]
    fn always_switch_audio_device_ids_are_deduped() {
        let mut settings = sample_settings();
        settings.always_switch_audio_device_ids = vec![
            "pioneer.ddj-400".into(),
            " ".into(),
            "pioneer.ddj-400".into(),
        ];
        let parsed = parse_settings(settings).expect("parse");
        assert_eq!(
            parsed.always_switch_audio_device_ids,
            vec!["pioneer.ddj-400".to_string()]
        );
    }

    #[test]
    fn clear_controller_audio_override_only_clears_its_owner() {
        let dir = tempfile::tempdir().expect("tempdir");
        let transport = SettingsTransport::open_at(&dir.path().join("settings.json"));
        let override_settings = controller_override("ddj-400", true, false);
        transport
            .set_controller_audio_override(Some(override_settings))
            .expect("set override");

        let kept = transport
            .clear_controller_audio_override("other".into())
            .expect("clear other");
        assert!(kept.controller_audio_override.is_some());

        let cleared = transport
            .clear_controller_audio_override("ddj-400".into())
            .expect("clear owner");
        assert!(cleared.controller_audio_override.is_none());
    }

    #[test]
    fn controller_audio_override_rejects_empty_mapping_id() {
        let mut settings = sample_settings();
        settings.controller_audio_override = Some(controller_override(" ", true, false));
        assert!(parse_settings(settings).is_err());
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
        assert_eq!(settings.stems_model, library::DEFAULT_STEMS_MODEL);

        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .insert("stems_model".into(), serde_json::json!("nope"));
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        let normalized = parse_settings(settings).expect("normalize");
        assert_eq!(normalized.stems_model, library::DEFAULT_STEMS_MODEL);
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
    fn missing_mixer_meters_defaults_mono() {
        let settings: AppSettings =
            serde_json::from_value(settings_without_key("mixer_meters")).expect("parse");
        assert_eq!(settings.mixer_meters, MixerMetersSetting::Mono);
    }

    #[test]
    fn mixer_meters_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        for mode in [
            MixerMetersSetting::None,
            MixerMetersSetting::Mono,
            MixerMetersSetting::Stereo,
        ] {
            let mut settings = sample_settings();
            settings.mixer_meters = mode;
            write_settings_file(&path, &settings).expect("write");

            let host = load_host(&path);
            assert_eq!(settings_from_host(&host).mixer_meters, mode);
        }
    }

    #[test]
    fn legacy_meter_flags_migrate_to_the_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");

        write_legacy_meter_file(&path, false, true);
        assert_eq!(
            settings_from_host(&load_host(&path)).mixer_meters,
            MixerMetersSetting::None
        );

        write_legacy_meter_file(&path, true, false);
        assert_eq!(
            settings_from_host(&load_host(&path)).mixer_meters,
            MixerMetersSetting::Stereo
        );

        write_legacy_meter_file(&path, true, true);
        assert_eq!(
            settings_from_host(&load_host(&path)).mixer_meters,
            MixerMetersSetting::Mono
        );
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
    fn missing_waveform_position_defaults_top() {
        let settings: AppSettings =
            serde_json::from_value(settings_without_key("waveform_position")).expect("parse");
        assert_eq!(settings.waveform_position, WaveformPositionSetting::Top);
    }

    #[test]
    fn waveform_position_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.waveform_position = WaveformPositionSetting::MiddleVertical;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert_eq!(
            restored.waveform_position,
            WaveformPositionSetting::MiddleVertical
        );
    }

    #[test]
    fn missing_show_waveform_defaults_on() {
        let settings: AppSettings =
            serde_json::from_value(settings_without_key("show_waveform")).expect("parse");
        assert!(settings.show_waveform);
    }

    #[test]
    fn missing_waveform_gain_eq_shaping_defaults_on() {
        let settings: AppSettings =
            serde_json::from_value(settings_without_key("waveform_gain_eq_shaping"))
                .expect("parse");
        assert!(
            settings.waveform_gain_eq_shaping,
            "older files keep shaping on, as #668 shipped it"
        );
    }

    #[test]
    fn waveform_gain_eq_shaping_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.waveform_gain_eq_shaping = false;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(!restored.waveform_gain_eq_shaping);
    }

    #[test]
    fn show_waveform_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.show_waveform = false;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(!restored.show_waveform);
    }

    #[test]
    fn missing_layout_panel_visibility_defaults_on() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        let object = value.as_object_mut().expect("object");
        for key in ["show_mixer", "show_fx_bar", "show_crossfader"] {
            object.remove(key);
        }
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert!(settings.show_mixer);
        assert!(settings.show_fx_bar);
        assert!(settings.show_crossfader);
    }

    #[test]
    fn layout_panel_visibility_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.show_mixer = false;
        settings.show_fx_bar = false;
        settings.show_crossfader = false;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert!(!restored.show_mixer);
        assert!(!restored.show_fx_bar);
        assert!(!restored.show_crossfader);
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
    fn missing_theme_defaults_system() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value.as_object_mut().expect("object").remove("theme");
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert_eq!(settings.theme, ThemeSetting::System);
    }

    #[test]
    fn theme_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.theme = ThemeSetting::Light;
        write_settings_file(&path, &settings).expect("write");
        assert_eq!(
            settings_from_host(&load_host(&path)).theme,
            ThemeSetting::Light
        );

        settings.theme = ThemeSetting::Dark;
        write_settings_file(&path, &settings).expect("write");
        assert_eq!(
            settings_from_host(&load_host(&path)).theme,
            ThemeSetting::Dark
        );
    }

    #[test]
    fn missing_accent_palette_defaults_teal() {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove("accent_palette");
        let settings: AppSettings = serde_json::from_value(value).expect("parse");
        assert_eq!(settings.accent_palette, AccentPaletteSetting::Teal);
    }

    #[test]
    fn accent_palette_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.accent_palette = AccentPaletteSetting::Violet;
        write_settings_file(&path, &settings).expect("write");
        assert_eq!(
            settings_from_host(&load_host(&path)).accent_palette,
            AccentPaletteSetting::Violet
        );
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
    fn always_switch_audio_device_ids_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.always_switch_audio_device_ids = vec!["pioneer.ddj-400".into()];
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        let restored = settings_from_host(&host);
        assert_eq!(
            restored.always_switch_audio_device_ids,
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
    fn apply_to_host_persists_analytics_enabled() {
        let mut host = SettingsHost::default();
        let mut settings = sample_settings();
        settings.analytics_enabled = true;
        apply_to_host(&mut host, settings).expect("settings apply");
        assert!(settings_from_host(&host).analytics_enabled);
    }

    #[test]
    fn apply_to_host_persists_onboarding_completed() {
        let mut host = SettingsHost::default();
        let mut settings = sample_settings();
        settings.onboarding_completed = true;
        apply_to_host(&mut host, settings).expect("settings apply");
        assert!(settings_from_host(&host).onboarding_completed);
    }

    #[test]
    fn onboarding_completed_round_trip_survives_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut settings = sample_settings();
        settings.onboarding_completed = true;
        write_settings_file(&path, &settings).expect("write");

        let host = load_host(&path);
        assert!(settings_from_host(&host).onboarding_completed);
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

    /// A settings file written before rates/latency moved onto the buses: the
    /// route objects have neither key.
    fn pre_move_settings_value() -> serde_json::Value {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        let object = value.as_object_mut().expect("object");
        for bus in ["master_bus", "preview_bus"] {
            if let Some(route) = object.get_mut(bus).and_then(|bus| bus.as_object_mut()) {
                route.remove("sample_rate");
                route.remove("low_latency");
            }
        }
        value
    }

    #[test]
    fn legacy_global_low_latency_moves_to_both_buses() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = pre_move_settings_value();
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
        let mut value = pre_move_settings_value();
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
        let mut value = pre_move_settings_value();
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
    fn legacy_zero_sample_rate_is_not_migrated() {
        // A zero rate cannot open a stream, so it must not be carried onto the bus
        // where `parse_bus` would reject the whole settings file.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = pre_move_settings_value();
        value
            .as_object_mut()
            .expect("object")
            .insert("sample_rate".into(), serde_json::json!(0));
        std::fs::write(&path, serde_json::to_vec(&value).expect("encode")).expect("write");

        let loaded = read_settings_file(&path).expect("legacy settings");
        assert_eq!(loaded.master_bus.sample_rate, None);
    }

    #[test]
    fn per_bus_values_suppress_the_legacy_globals() {
        // New-format files always carry both route keys, so the retired globals in a
        // hand-edited file must not overwrite them.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        let object = value.as_object_mut().expect("object");
        object.insert("low_latency".into(), serde_json::Value::Bool(true));
        object.insert("sample_rate".into(), serde_json::json!(44_100));
        object["master_bus"]["low_latency"] = serde_json::Value::Bool(false);
        object["master_bus"]["sample_rate"] = serde_json::json!(96_000);
        std::fs::write(&path, serde_json::to_vec(&value).expect("encode")).expect("write");

        let loaded = read_settings_file(&path).expect("mixed settings");
        assert!(!loaded.master_bus.low_latency);
        assert_eq!(loaded.master_bus.sample_rate, Some(96_000));
        assert!(!loaded.preview_bus.low_latency);
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
    /// [AppSettings] parsed from a settings file written before [key] existed.
    fn legacy_settings_without(key: &str) -> AppSettings {
        let mut value = serde_json::to_value(sample_settings()).expect("json");
        value
            .as_object_mut()
            .expect("object")
            .remove(key)
            .unwrap_or_else(|| panic!("{key} is not part of the settings"));
        serde_json::from_value(value).expect("legacy settings")
    }

    #[test]
    fn legacy_settings_default_analytics_opt_out() {
        assert!(!legacy_settings_without("analytics_enabled").analytics_enabled);
    }

    #[test]
    fn legacy_settings_default_onboarding_incomplete() {
        assert!(!legacy_settings_without("onboarding_completed").onboarding_completed);
    }

    #[test]
    fn default_settings_carry_the_equal_power_curve_and_no_flags() {
        let parsed = parse_settings(sample_settings()).expect("defaults parse");
        assert_eq!(
            parsed.default_crossfader_curve,
            bus_api::DEFAULT_CROSSFADER_CURVE_ID
        );
        assert_eq!(parsed.default_tempo_curve, bus_api::DEFAULT_TEMPO_CURVE_ID);
        assert_eq!(
            parsed.default_fx_level_curve,
            bus_api::DEFAULT_FX_LEVEL_CURVE_ID
        );
        assert_eq!(
            parsed.crossfader_behavior,
            bus_api::CrossfaderBehaviorSetting::default()
        );
        assert!(parsed.curve_library.groups.is_empty());
        assert!(parsed.curve_library.sets.is_empty());
        assert_eq!(
            legacy_settings_without("default_tempo_curve").default_tempo_curve,
            bus_api::DEFAULT_TEMPO_CURVE_ID
        );
        assert_eq!(
            legacy_settings_without("default_fx_level_curve").default_fx_level_curve,
            bus_api::DEFAULT_FX_LEVEL_CURVE_ID
        );
        assert!(legacy_settings_without("default_crossfader_curve")
            .curve_library
            .groups
            .is_empty());
    }

    #[test]
    fn crossfader_curve_settings_persist_and_normalize() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut host = SettingsHost {
            persist_path: Some(path.clone()),
            ..Default::default()
        };

        // A display alias is accepted and stored as the canonical snake_case id.
        let mut draft = settings_from_host(&host);
        draft.default_crossfader_curve = "Linear".into();
        draft.default_tempo_curve = "Linear".into();
        draft.default_fx_level_curve = "Linear".into();
        draft.crossfader_behavior = bus_api::CrossfaderBehaviorSetting {
            disabled: false,
            hamster: true,
        };
        apply_to_host(&mut host, draft).expect("apply");
        let saved = parse_settings(settings_from_host(&host)).expect("parse");
        assert_eq!(saved.default_crossfader_curve, "linear");
        assert_eq!(saved.default_tempo_curve, "linear");
        assert_eq!(saved.default_fx_level_curve, "linear");
        assert!(saved.crossfader_behavior.hamster);
        write_settings_file(&path, &saved).expect("write");

        let reloaded = settings_from_host(&load_host(&path));
        assert_eq!(reloaded.default_crossfader_curve, "linear");
        assert_eq!(reloaded.default_tempo_curve, "linear");
        assert_eq!(reloaded.default_fx_level_curve, "linear");
        assert!(reloaded.crossfader_behavior.hamster);
        assert!(!reloaded.crossfader_behavior.disabled);
    }

    #[test]
    fn curve_library_persists_and_duplicate_ids_are_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut host = SettingsHost {
            persist_path: Some(path.clone()),
            ..Default::default()
        };

        let mut draft = settings_from_host(&host);
        draft.curve_library = bus_api::CurveLibrarySpec {
            groups: vec![curve_group("tight")],
            sets: vec![bus_api::CurveSetSpec {
                id: "sweep".into(),
                display_name: "Sweep".into(),
                members: vec![
                    bus_api::CurveRefSpec {
                        id: "tight".into(),
                        parameter: None,
                    },
                    bus_api::CurveRefSpec {
                        id: "linear".into(),
                        parameter: None,
                    },
                ],
            }],
        };
        draft.default_crossfader_curve = "sweep".into();
        apply_to_host(&mut host, draft).expect("apply");
        let saved = parse_settings(settings_from_host(&host)).expect("parse");
        assert_eq!(saved.default_crossfader_curve, "sweep");
        assert_eq!(saved.curve_library.sets.len(), 1);
        write_settings_file(&path, &saved).expect("write");
        assert_eq!(
            settings_from_host(&load_host(&path))
                .curve_library
                .sets
                .len(),
            1
        );

        // Duplicate ids are rejected before anything is written, and the host
        // keeps the library it already had.
        let mut duplicate = saved;
        duplicate.curve_library.groups.push(curve_group("tight"));
        let error = apply_to_host(&mut host, duplicate).unwrap_err();
        assert!(error.to_lowercase().contains("duplicate"), "{error}");
        assert_eq!(settings_from_host(&host).curve_library.groups.len(), 1);
    }

    #[test]
    fn preset_options_expose_canonical_ids_and_reserved_names() {
        let options = crossfader_preset_options();
        let available: Vec<&str> = options
            .iter()
            .filter(|option| option.available)
            .map(|option| option.id.as_str())
            .collect();
        assert_eq!(available, bus_api::CROSSFADER_CURVE_CYCLE.to_vec());
        let reserved = options
            .iter()
            .find(|option| option.id == "full")
            .expect("reserved names stay in the catalog");
        assert!(!reserved.available);
        assert_eq!(reserved.display_name, "Full");
        assert!(
            options.iter().all(|option| option
                .id
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')),
            "dropdown values are canonical snake_case ids"
        );
    }

    /// A straight two-channel custom group with canonical endpoints.
    fn curve_group(id: &str) -> bus_api::CurveGroupSpec {
        let segment = bus_api::CurveSegmentSpec {
            kind: bus_api::CurveSegmentKind::Linear,
            rate: 0.0,
            c1x: 0.0,
            c1y: 0.0,
            c2x: 0.0,
            c2y: 0.0,
        };
        let curve = |y0: f32, y1: f32| bus_api::CurveSpecSpec {
            points: vec![
                bus_api::CurvePointSpec { x: 0.0, y: y0 },
                bus_api::CurvePointSpec { x: 1.0, y: y1 },
            ],
            segments: vec![segment],
        };
        bus_api::CurveGroupSpec {
            id: id.into(),
            display_name: id.into(),
            curves: vec![curve(1.0, 0.0), curve(0.0, 1.0)],
        }
    }

    #[test]
    fn realtime_stems_needs_a_margin_over_exact_realtime() {
        assert!(
            !RealtimeStemsCapability::realtime_allows(1.0),
            "1.0x is exact realtime and has no headroom"
        );
        assert!(!RealtimeStemsCapability::realtime_allows(1.099));
        assert!(RealtimeStemsCapability::realtime_allows(
            REALTIME_STEMS_MIN_MULTIPLE
        ));
        assert!(RealtimeStemsCapability::realtime_allows(1.5));
        assert!(!RealtimeStemsCapability::realtime_allows(f64::NAN));
    }

    /// A measured result for this session; `allowed` derives from `realtime`.
    #[cfg(feature = "stems")]
    fn capability_with(realtime: f64) -> RealtimeStemsCapability {
        RealtimeStemsCapability {
            avg_hop_ms: analyzer_stems::HOP_BUDGET_MS / realtime,
            realtime,
            hops: analyzer_stems::DEFAULT_BENCH_HOPS,
            model_sha: stemgen_model_sha().expect("builtin StemgenRT manifest"),
            allowed: RealtimeStemsCapability::realtime_allows(realtime),
        }
    }

    #[test]
    #[cfg(feature = "stems")]
    fn enabling_realtime_stems_requires_a_passing_benchmark() {
        let too_slow = capability_with(0.97);
        let fast_enough = capability_with(1.4);

        assert!(
            !enable_allowed(false, true, None),
            "enabling cannot skip the benchmark"
        );
        assert!(
            !enable_allowed(false, true, Some(&too_slow)),
            "0.97x is below the 1.1x margin"
        );
        assert!(enable_allowed(false, true, Some(&fast_enough)));
        assert!(
            enable_allowed(true, true, None),
            "an opt-in loaded from settings must not need a fresh probe"
        );
        assert!(
            enable_allowed(true, false, Some(&fast_enough)),
            "disabling is always allowed"
        );
    }

    #[test]
    #[cfg(feature = "stems")]
    fn saved_opt_in_survives_reload_without_a_stored_measurement() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let mut host = SettingsHost::default();
        let mut requested = sample_settings();
        requested.realtime_stems = true;
        apply_to_host(&mut host, requested).expect("apply");
        write_settings_file(&path, &settings_from_host(&host)).expect("write");

        let bytes = std::fs::read(&path).expect("read");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert!(
            value.get("realtime_stems_capability").is_none(),
            "the benchmark stays in memory, not in settings"
        );

        let loaded = load_host(&path);
        assert!(loaded.realtime_capability.is_none());
        assert!(
            loaded.realtime_stems,
            "the opt-in survives the launch; no re-benchmark prompt"
        );
    }
}
