mod config;
mod encode;
mod filters;

use crate::LoadedAudio;
use config::AmplitudeMode;
use filters::new_band_filter_bank;
use serde::{Deserialize, Serialize};

pub use config::{
    AmplitudeMode as WaveformAmplitudeMode, ChannelMode as WaveformChannelMode,
    FilterKind as WaveformFilterKind, WaveformAnalysisConfig, OVERVIEW_SAMPLE_COUNT,
    WAVEFORM_SCHEMA_VERSION,
};
pub use encode::{peaks_to_rgb_bytes, rgb_bytes_to_peaks};

/// Minimum buckets returned for any track (hi-res window analysis).
pub const MIN_WAVEFORM_BUCKETS: usize = 4_096;
/// Maximum buckets (caps memory and generation time for long files).
pub const MAX_WAVEFORM_BUCKETS: usize = 16_384;
/// Target temporal resolution when choosing an adaptive bucket count (~13 ms).
pub const WAVEFORM_MS_PER_BUCKET: f64 = 13.0;

/// Three-band peak envelope for spectral waveform coloring.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpectralPeak {
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

/// Choose a bucket count from track length (~77 buckets/s).
///
/// Non-positive `duration_ms` returns `0`; otherwise
/// `ceil(duration_ms / WAVEFORM_MS_PER_BUCKET).max(1)` with no min/max clamp.
pub fn waveform_buckets_for_duration(duration_ms: i32) -> usize {
    if duration_ms <= 0 {
        return 0;
    }
    let buckets = (duration_ms as f64 / WAVEFORM_MS_PER_BUCKET).ceil() as usize;
    buckets.max(1)
}

/// Buckets for a visible time window (~1 bucket per pixel).
pub fn waveform_buckets_for_window(visible_secs: f64, width_px: usize) -> usize {
    let buckets = width_px.max(16);
    if visible_secs <= 0.0 {
        return buckets;
    }
    buckets.clamp(16, MAX_WAVEFORM_BUCKETS)
}

/// Downsampled peak envelope for waveform display (values 0.0–1.0).
pub fn compute_peak_envelope(audio: &LoadedAudio, buckets: usize) -> Vec<f32> {
    compute_spectral_envelope(audio, buckets, &WaveformAnalysisConfig::default())
        .into_iter()
        .map(|peak| peak.low.max(peak.mid).max(peak.high))
        .collect()
}

/// Downsampled three-band **peak** envelope for spectral waveform display.
pub fn compute_spectral_envelope(
    audio: &LoadedAudio,
    buckets: usize,
    config: &WaveformAnalysisConfig,
) -> Vec<SpectralPeak> {
    let channels = usize::from(audio.channels.max(1));
    let frame_count = audio.samples.len() / channels;
    compute_spectral_envelope_frames(
        &audio.samples,
        audio.sample_rate,
        channels,
        0,
        frame_count,
        buckets,
        config,
    )
}

/// Fixed-length overview stored in the library database.
pub fn compute_overview_envelope(
    audio: &LoadedAudio,
    config: &WaveformAnalysisConfig,
) -> Vec<SpectralPeak> {
    compute_spectral_envelope(audio, OVERVIEW_SAMPLE_COUNT, config)
}

/// Hi-res envelope for a time slice of the track.
pub fn compute_spectral_window(
    audio: &LoadedAudio,
    start_secs: f64,
    end_secs: f64,
    buckets: usize,
    config: &WaveformAnalysisConfig,
) -> Vec<SpectralPeak> {
    let channels = usize::from(audio.channels.max(1));
    let frame_count = audio.samples.len() / channels;
    if frame_count == 0 || end_secs <= start_secs {
        return vec![
            SpectralPeak {
                low: 0.0,
                mid: 0.0,
                high: 0.0,
            };
            buckets.max(1)
        ];
    }

    let sample_rate = f64::from(audio.sample_rate);
    let start_frame = (start_secs * sample_rate).floor() as usize;
    let end_frame = (end_secs * sample_rate).ceil() as usize;
    let start_frame = start_frame.min(frame_count);
    let end_frame = end_frame.min(frame_count).max(start_frame);

    compute_spectral_envelope_frames(
        &audio.samples,
        audio.sample_rate,
        channels,
        start_frame,
        end_frame,
        buckets,
        config,
    )
}

pub fn compute_spectral_envelope_frames(
    samples: &[f32],
    sample_rate: u32,
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    buckets: usize,
    config: &WaveformAnalysisConfig,
) -> Vec<SpectralPeak> {
    let buckets = buckets.clamp(16, MAX_WAVEFORM_BUCKETS);
    let frame_count = end_frame.saturating_sub(start_frame);

    if frame_count == 0 || channels == 0 {
        return vec![
            SpectralPeak {
                low: 0.0,
                mid: 0.0,
                high: 0.0,
            };
            buckets
        ];
    }

    let frames_per_bucket = frame_count.div_ceil(buckets);
    let mut peaks = Vec::with_capacity(buckets);

    for bucket in 0..buckets {
        let rel_start = bucket * frames_per_bucket;
        if rel_start >= frame_count {
            peaks.push(SpectralPeak {
                low: 0.0,
                mid: 0.0,
                high: 0.0,
            });
            continue;
        }
        let rel_end = ((bucket + 1) * frames_per_bucket).min(frame_count);
        let abs_start = start_frame + rel_start;
        let abs_end = start_frame + rel_end;
        peaks.push(spectral_peak_for_range(
            &samples[abs_start * channels..abs_end * channels],
            sample_rate,
            channels,
            config,
        ));
    }

    normalize_spectral_peaks(&mut peaks);
    peaks
}

fn spectral_peak_for_range(
    samples: &[f32],
    sample_rate: u32,
    channels: usize,
    config: &WaveformAnalysisConfig,
) -> SpectralPeak {
    if samples.is_empty() || channels == 0 {
        return SpectralPeak {
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        };
    }

    let mut filters = new_band_filter_bank(
        config.filter_kind,
        sample_rate,
        config.low_crossover_hz,
        config.mid_high_crossover_hz,
    );

    let mut low_peak = 0.0f32;
    let mut mid_peak = 0.0f32;
    let mut high_peak = 0.0f32;

    let frame_count = samples.len() / channels;
    for frame in 0..frame_count {
        let base = frame * channels;
        let mono = if channels == 1 {
            samples[base]
        } else {
            (samples[base] + samples[base + 1]) * 0.5
        };

        let (low, mid, high) = filters.split(mono);
        let amplitude = |v: f32| match config.amplitude_mode {
            AmplitudeMode::Peak => v.abs(),
            AmplitudeMode::Rms => v * v,
        };

        low_peak = low_peak.max(amplitude(low));
        mid_peak = mid_peak.max(amplitude(mid));
        high_peak = high_peak.max(amplitude(high));
    }

    if config.amplitude_mode == AmplitudeMode::Rms {
        let frames = frame_count.max(1) as f32;
        low_peak = (low_peak / frames).sqrt();
        mid_peak = (mid_peak / frames).sqrt();
        high_peak = (high_peak / frames).sqrt();
    }

    SpectralPeak {
        low: low_peak,
        mid: mid_peak,
        high: high_peak,
    }
}

fn normalize_spectral_peaks(peaks: &mut [SpectralPeak]) {
    let mut max_peak = 0.0f32;
    for peak in peaks.iter() {
        max_peak = max_peak.max(peak.low).max(peak.mid).max(peak.high);
    }
    if max_peak > 0.0 {
        for peak in peaks {
            peak.low /= max_peak;
            peak.mid /= max_peak;
            peak.high /= max_peak;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LoadedAudio;

    fn mono_audio(samples: Vec<f32>, sample_rate: u32) -> LoadedAudio {
        LoadedAudio {
            samples,
            sample_rate,
            channels: 1,
            source_id: "test".into(),
        }
    }

    #[test]
    fn empty_audio_returns_zeros() {
        let audio = mono_audio(vec![], 48_000);
        let peaks = compute_spectral_envelope(&audio, 32, &WaveformAnalysisConfig::default());
        assert_eq!(peaks.len(), 32);
        assert!(peaks
            .iter()
            .all(|p| p.low == 0.0 && p.mid == 0.0 && p.high == 0.0));
    }

    #[test]
    fn adaptive_buckets_scale_with_duration() {
        use crate::secs_to_ms;

        assert_eq!(waveform_buckets_for_duration(0), 0);
        assert_eq!(waveform_buckets_for_duration(secs_to_ms(30.0)), 2_308);
        let long = waveform_buckets_for_duration(secs_to_ms(600.0));
        assert_eq!(long, 46_154);
    }

    #[test]
    fn single_spike_normalized() {
        let mut samples = vec![0.0; 100];
        samples[50] = 0.5;
        let peaks = compute_spectral_envelope(
            &mono_audio(samples, 48_000),
            4,
            &WaveformAnalysisConfig::default(),
        );
        let max = peaks
            .iter()
            .map(|p| p.low.max(p.mid).max(p.high))
            .fold(0.0f32, f32::max);
        assert!((max - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn overview_has_fixed_length() {
        let samples = vec![0.1; 48_000];
        let peaks = compute_overview_envelope(
            &mono_audio(samples, 48_000),
            &WaveformAnalysisConfig::default(),
        );
        assert_eq!(peaks.len(), OVERVIEW_SAMPLE_COUNT);
    }

    #[test]
    fn encode_round_trip_through_overview() {
        use config::ChannelMode;

        let samples = vec![0.0; 48_000 * 2];
        let peaks = compute_overview_envelope(
            &mono_audio(samples, 48_000),
            &WaveformAnalysisConfig::default(),
        );
        let bytes = peaks_to_rgb_bytes(&peaks, ChannelMode::Mono);
        let decoded =
            rgb_bytes_to_peaks(&bytes, peaks.len(), ChannelMode::Mono).expect("decode overview");
        assert_eq!(decoded.len(), peaks.len());
    }
}
