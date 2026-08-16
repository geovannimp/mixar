/// Fixed overview length stored per track in the library DB.
pub const OVERVIEW_SAMPLE_COUNT: usize = 2048;

/// Bump when analysis or blob layout changes.
pub const WAVEFORM_SCHEMA_VERSION: u32 = 2;

pub const DEFAULT_LOW_CROSSOVER_HZ: f32 = 600.0;
pub const DEFAULT_MID_HIGH_CROSSOVER_HZ: f32 = 4000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmplitudeMode {
    Peak,
    Rms,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelMode {
    Mono,
    Stereo,
}

impl ChannelMode {
    pub fn bytes_per_sample(self) -> usize {
        match self {
            ChannelMode::Mono => 3,
            ChannelMode::Stereo => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterKind {
    OnePole,
    Biquad,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WaveformAnalysisConfig {
    pub amplitude_mode: AmplitudeMode,
    pub channel_mode: ChannelMode,
    pub filter_kind: FilterKind,
    pub low_crossover_hz: f32,
    pub mid_high_crossover_hz: f32,
}

impl Default for WaveformAnalysisConfig {
    fn default() -> Self {
        Self {
            amplitude_mode: AmplitudeMode::Peak,
            channel_mode: ChannelMode::Mono,
            filter_kind: FilterKind::Biquad,
            low_crossover_hz: DEFAULT_LOW_CROSSOVER_HZ,
            mid_high_crossover_hz: DEFAULT_MID_HIGH_CROSSOVER_HZ,
        }
    }
}

impl WaveformAnalysisConfig {
    pub fn bytes_per_sample(self) -> usize {
        self.channel_mode.bytes_per_sample()
    }
}
