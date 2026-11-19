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
/// Maximum buckets for whole-track passes that need a far finer fixed
/// resolution than a display window: 150 entries per second for 60 minutes,
/// the detail tier device exports write.
pub const MAX_FULL_TRACK_BUCKETS: usize = 540_000;
/// Target temporal resolution when choosing an adaptive bucket count (~13 ms).
pub const WAVEFORM_MS_PER_BUCKET: f64 = 13.0;

/// Three-band peak envelope for spectral waveform coloring.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

/// Track-wide amplitude reference that keeps window detail stable.
///
/// Window analysis normalizes against this one scalar instead of its own crop,
/// so a louder passage scrolling off screen cannot rescale what is still
/// visible. It is one continuous-filter pass over the complete track at the
/// default analysis configuration, independent of crop bounds and bucket count.
/// Zero sample rate, no complete frames, or a silent/unnormalizable track all
/// return `0.0`.
pub fn spectral_normalization_peak(audio: &LoadedAudio) -> f32 {
    let channels = usize::from(audio.channels.max(1));
    if audio.sample_rate == 0 || audio.samples.len() / channels == 0 {
        return 0.0;
    }
    let peak = spectral_peak_for_range(
        &audio.samples,
        audio.sample_rate,
        channels,
        &WaveformAnalysisConfig::default(),
    );
    let reference = peak.low.max(peak.mid).max(peak.high);
    if reference.is_finite() && reference > 0.0 {
        reference
    } else {
        0.0
    }
}

/// Hi-res envelope for a time slice of the track.
///
/// `normalization_peak` is the track-wide [`spectral_normalization_peak`]
/// reference: every band is divided by it and clamped to `0.0–1.0`. A
/// nonpositive or nonfinite reference yields zero bands rather than dividing.
pub fn compute_spectral_window(
    audio: &LoadedAudio,
    start_secs: f64,
    end_secs: f64,
    buckets: usize,
    config: &WaveformAnalysisConfig,
    normalization_peak: f32,
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

    let mut peaks = envelope_frames(
        &audio.samples,
        audio.sample_rate,
        channels,
        start_frame,
        end_frame,
        buckets,
        config,
        MAX_WAVEFORM_BUCKETS,
        false,
    );
    normalize_to_reference(&mut peaks, normalization_peak);
    peaks
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
    envelope_frames(
        samples,
        sample_rate,
        channels,
        start_frame,
        end_frame,
        buckets,
        config,
        MAX_WAVEFORM_BUCKETS,
        true,
    )
}

/// Whole-track envelope at a fixed, much finer resolution than the display
/// path: device exports sample the detail waveform at 150 entries per second,
/// well past [`MAX_WAVEFORM_BUCKETS`]. Memory and time scale with `buckets`
/// (capped at [`MAX_FULL_TRACK_BUCKETS`]), so callers pass a count derived from
/// the track length rather than a display width.
pub fn compute_full_track_envelope(
    audio: &LoadedAudio,
    buckets: usize,
    config: &WaveformAnalysisConfig,
) -> Vec<SpectralPeak> {
    let channels = usize::from(audio.channels.max(1));
    let frame_count = audio.samples.len() / channels;
    envelope_frames(
        &audio.samples,
        audio.sample_rate,
        channels,
        0,
        frame_count,
        buckets,
        config,
        MAX_FULL_TRACK_BUCKETS,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn envelope_frames(
    samples: &[f32],
    sample_rate: u32,
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    buckets: usize,
    config: &WaveformAnalysisConfig,
    max_buckets: usize,
    normalize: bool,
) -> Vec<SpectralPeak> {
    let buckets = buckets.clamp(16, max_buckets);
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

    if normalize {
        normalize_spectral_peaks(&mut peaks);
    }
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

/// Divide window bands by the track-wide reference instead of the crop's own
/// maximum. Clamping to `0.0–1.0` absorbs filter-start overshoot and any
/// reference rounding.
fn normalize_to_reference(peaks: &mut [SpectralPeak], reference: f32) {
    let usable = reference.is_finite() && reference > 0.0;
    for peak in peaks {
        if !usable {
            *peak = SpectralPeak {
                low: 0.0,
                mid: 0.0,
                high: 0.0,
            };
            continue;
        }
        peak.low = unit_band(peak.low / reference);
        peak.mid = unit_band(peak.mid / reference);
        peak.high = unit_band(peak.high / reference);
    }
}

fn unit_band(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
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

    fn band_max(peak: &SpectralPeak) -> f32 {
        peak.low.max(peak.mid).max(peak.high)
    }

    fn assert_same_band(actual: &SpectralPeak, expected: &SpectralPeak, what: &str) {
        for (a, b, band) in [
            (actual.low, expected.low, "low"),
            (actual.mid, expected.mid, "mid"),
            (actual.high, expected.high, "high"),
        ] {
            assert!(
                (a - b).abs() <= 1e-6,
                "{what}: {band} {a} != {b} (peak {actual:?} vs {expected:?})"
            );
        }
    }

    /// 1 kHz mono tone: `before` amplitude until `step_secs`, `after` past it.
    ///
    /// One helper for every stepped signal in these tests, so the two levels
    /// and the step time are the only things a case chooses.
    fn stepped_tone(
        sample_rate: u32,
        seconds: f32,
        step_secs: f32,
        before: f32,
        after: f32,
    ) -> Vec<f32> {
        let frames = (f64::from(sample_rate) * f64::from(seconds)).round() as usize;
        let sr = sample_rate as f32;
        (0..frames)
            .map(|i| {
                let time = i as f32 / sr;
                let amplitude = if time >= step_secs { after } else { before };
                amplitude * (2.0 * std::f32::consts::PI * 1_000.0 * time).sin()
            })
            .collect()
    }

    fn tone_at(sample_rate: u32, seconds: f32, amplitude: f32) -> Vec<f32> {
        let frames = (f64::from(sample_rate) * f64::from(seconds)).round() as usize;
        sine(1_000.0, sample_rate, frames)
            .into_iter()
            .map(|sample| sample * amplitude)
            .collect()
    }

    fn window(
        audio: &LoadedAudio,
        start_secs: f64,
        end_secs: f64,
        buckets: usize,
        reference: f32,
    ) -> Vec<SpectralPeak> {
        compute_spectral_window(
            audio,
            start_secs,
            end_secs,
            buckets,
            &WaveformAnalysisConfig::default(),
            reference,
        )
    }

    #[test]
    fn expanding_window_keeps_visible_detail_identical() {
        let audio = mono_audio(stepped_tone(48_000, 60.0, 40.0, 0.1, 0.9), 48_000);
        let reference = spectral_normalization_peak(&audio);
        assert!(reference > 0.0, "a loud tail must give a reference");

        // 36 s / 2048 and 54 s / 3072 both ceiling-divide to 844 frames per
        // bucket, so every complete bucket covers identical source frames. The
        // first crop's last bucket is partial and is excluded.
        let short = window(&audio, 0.0, 36.0, 2_048, reference);
        let long = window(&audio, 0.0, 54.0, 3_072, reference);
        assert_eq!(short.len(), 2_048);
        assert_eq!(long.len(), 3_072);

        let shared = 2_047;
        for (index, (a, b)) in short[..shared].iter().zip(&long[..shared]).enumerate() {
            assert_same_band(a, b, &format!("bucket {index}"));
        }

        use config::ChannelMode;
        assert_eq!(
            peaks_to_rgb_bytes(&short[..shared], ChannelMode::Mono),
            peaks_to_rgb_bytes(&long[..shared], ChannelMode::Mono),
            "packed detail bytes must not move when the crop grows"
        );

        // Everything visible here is the quiet half of the track: with the
        // track-wide reference it stays small instead of renormalizing to 1.0.
        let quiet_max = short.iter().map(band_max).fold(0.0f32, f32::max);
        assert!(
            quiet_max < 0.5,
            "quiet crop renormalized against itself: max={quiet_max}"
        );
    }

    #[test]
    fn window_reference_comes_from_the_whole_track() {
        // The loud part starts at 50 s, so neither crop below contains it.
        let audio = mono_audio(stepped_tone(48_000, 60.0, 50.0, 0.1, 0.9), 48_000);
        let reference = spectral_normalization_peak(&audio);

        // 24 s at 48 kHz over 2048 buckets is 562.5 frames, ceiling-divided to
        // 563, so these crops do not share index-aligned frame ranges.
        let left = window(&audio, 0.0, 24.0, 2_048, reference);
        let right = window(&audio, 12.0, 36.0, 2_048, reference);
        assert_eq!(left.len(), 2_048);
        assert_eq!(right.len(), 2_048);

        for (name, crop) in [("left", &left), ("right", &right)] {
            let max = crop.iter().map(band_max).fold(0.0f32, f32::max);
            assert!(
                max < 0.5,
                "{name} crop normalized against its own quiet peak: max={max}"
            );
        }
    }

    #[test]
    fn shifted_overlap_matches_byte_for_byte() {
        let sample_rate = 48_000;
        // Loud for the first second, quiet after: the shared passage the two
        // crops below overlap is entirely in the quiet tail.
        let audio = mono_audio(stepped_tone(sample_rate, 3.0, 1.0, 0.9, 0.1), sample_rate);
        let reference = spectral_normalization_peak(&audio);

        // 2 s at 48 kHz is exactly 96 000 frames: 48 frames per bucket, so the
        // second window's buckets are the first window's shifted by 1000.
        let first = window(&audio, 0.0, 2.0, 2_000, reference);
        let second = window(&audio, 1.0, 3.0, 2_000, reference);
        assert_eq!(first.len(), 2_000);
        assert_eq!(second.len(), 2_000);

        for (offset, (a, b)) in first[1_000..].iter().zip(&second[..1_000]).enumerate() {
            assert_same_band(a, b, &format!("shifted bucket {offset}"));
        }

        use config::ChannelMode;
        assert_eq!(
            peaks_to_rgb_bytes(&first[1_000..], ChannelMode::Mono),
            peaks_to_rgb_bytes(&second[..1_000], ChannelMode::Mono),
        );
    }

    #[test]
    fn uniformly_quiet_tracks_keep_their_normalized_height() {
        let quiet = mono_audio(tone_at(48_000, 2.0, 0.1), 48_000);
        let loud = mono_audio(tone_at(48_000, 2.0, 0.9), 48_000);

        let quiet_peaks = window(&quiet, 0.0, 2.0, 128, spectral_normalization_peak(&quiet));
        let loud_peaks = window(&loud, 0.0, 2.0, 128, spectral_normalization_peak(&loud));
        assert_eq!(quiet_peaks.len(), 128);
        assert_eq!(loud_peaks.len(), 128);

        for (a, b) in quiet_peaks.iter().zip(&loud_peaks) {
            for (x, y, band) in [
                (a.low, b.low, "low"),
                (a.mid, b.mid, "mid"),
                (a.high, b.high, "high"),
            ] {
                assert!((x - y).abs() < 1e-5, "{band} {x} != {y}");
            }
        }
        let max = loud_peaks.iter().map(band_max).fold(0.0f32, f32::max);
        assert!(max > 0.8, "a loud track must still fill the lane: {max}");
    }

    #[test]
    fn silent_and_degenerate_inputs_stay_finite() {
        let silent = mono_audio(vec![0.0; 48_000], 48_000);
        assert_eq!(spectral_normalization_peak(&silent), 0.0);
        let peaks = window(&silent, 0.0, 1.0, 32, spectral_normalization_peak(&silent));
        let zero = SpectralPeak {
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        };
        assert!(peaks.iter().all(|p| *p == zero));

        assert_eq!(
            spectral_normalization_peak(&mono_audio(vec![], 48_000)),
            0.0
        );
        assert_eq!(
            spectral_normalization_peak(&mono_audio(vec![0.1; 480], 0)),
            0.0
        );

        let tone = mono_audio(tone_at(48_000, 1.0, 0.5), 48_000);
        for reference in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let peaks = window(&tone, 0.0, 1.0, 32, reference);
            assert_eq!(peaks.len(), 32);
            assert!(
                peaks.iter().all(|p| p.low.is_finite()
                    && p.mid.is_finite()
                    && p.high.is_finite()
                    && p.low == 0.0
                    && p.mid == 0.0
                    && p.high == 0.0),
                "reference {reference} must yield finite zero bands"
            );
        }

        // An empty / inverted interval keeps the historical zero-length rule.
        let empty = window(&tone, 1.0, 1.0, 32, 1.0);
        assert_eq!(empty.len(), 32);
        assert!(empty.iter().all(|p| *p == zero));
    }

    #[test]
    fn full_track_envelope_still_normalizes_to_one() {
        let audio = mono_audio(stepped_tone(48_000, 3.0, 2.0, 0.1, 0.9), 48_000);
        let peaks = compute_full_track_envelope(&audio, 512, &WaveformAnalysisConfig::default());
        let max = peaks.iter().map(band_max).fold(0.0f32, f32::max);
        assert!((max - 1.0).abs() < 1e-6, "max={max}");
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

    fn sine(freq_hz: f32, sample_rate: u32, frames: usize) -> Vec<f32> {
        let sr = sample_rate as f32;
        (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * freq_hz * i as f32 / sr).sin())
            .collect()
    }

    fn mid_bucket(peaks: &[SpectralPeak]) -> SpectralPeak {
        peaks[peaks.len() / 2]
    }

    #[test]
    fn biquad_1khz_tone_is_strongest_in_mid_band() {
        let peaks = compute_spectral_envelope(
            &mono_audio(sine(1_000.0, 48_000, 48_000), 48_000),
            32,
            &WaveformAnalysisConfig::default(),
        );
        let peak = mid_bucket(&peaks);
        assert!(
            peak.mid > peak.low && peak.mid > peak.high,
            "1 kHz should land in mid, got low={} mid={} high={}",
            peak.low,
            peak.mid,
            peak.high
        );
    }

    #[test]
    fn biquad_100hz_tone_is_strongest_in_low_band() {
        let peaks = compute_spectral_envelope(
            &mono_audio(sine(100.0, 48_000, 48_000), 48_000),
            32,
            &WaveformAnalysisConfig::default(),
        );
        let peak = mid_bucket(&peaks);
        assert!(
            peak.low > peak.mid && peak.low > peak.high,
            "100 Hz should land in low, got low={} mid={} high={}",
            peak.low,
            peak.mid,
            peak.high
        );
    }

    #[test]
    fn biquad_8khz_tone_is_strongest_in_high_band() {
        let peaks = compute_spectral_envelope(
            &mono_audio(sine(8_000.0, 48_000, 48_000), 48_000),
            32,
            &WaveformAnalysisConfig::default(),
        );
        let peak = mid_bucket(&peaks);
        assert!(
            peak.high > peak.low && peak.high > peak.mid,
            "8 kHz should land in high, got low={} mid={} high={}",
            peak.low,
            peak.mid,
            peak.high
        );
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
