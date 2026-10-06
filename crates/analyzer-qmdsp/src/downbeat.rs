//! qm-dsp `DownBeat`: estimate which beats are the first beat of a bar.
//!
//! Port of Davies & Plumbley, "A spectral difference approach to extracting
//! downbeats in musical audio" (EUSIPCO 2006), as implemented by qm-dsp's
//! `DownBeat`. The audio is decimated to ~1/16 rate, each beat is transformed
//! with a Hann window, the Jensen–Shannon divergence between consecutive beat
//! spectra is averaged per candidate phase, and the phase with the greatest
//! spectral change is the downbeat.
//!
//! The meter (beats per bar) is supplied by the caller; this only finds the
//! phase among the beats.

use std::f64::consts::PI;

use crate::fft::RealFft;
use crate::key::decimator::Decimator;
use crate::math::{adaptive_threshold, get_max_index, next_power_of_two};

/// qm-dsp `MathAliases::EPS`.
const EPS: f64 = 2.2204e-16;

/// Decimation factor (8 × 2), matching qm-dsp's ~2.7 kHz at 44.1/48 kHz.
const DECIMATION: usize = 16;

/// Return downbeat positions (seconds) among `beats` (seconds), for a given
/// meter. Empty when there is not enough audio or too few beats.
pub fn detect_downbeats(
    samples: &[f64],
    sample_rate: u32,
    beats: &[f32],
    beats_per_bar: u8,
) -> Vec<f32> {
    if beats.len() < 2 || samples.is_empty() {
        return Vec::new();
    }
    let Some(downsampled) = decimate(samples) else {
        return Vec::new();
    };
    let ds_rate = sample_rate as f64 / DECIMATION as f64;

    let frame_size = next_power_of_two((ds_rate * 1.3) as i32).max(2) as usize;
    let half = frame_size / 2;

    let mut fft = RealFft::new(frame_size);
    let mut frame = vec![0.0f64; frame_size];
    let mut re = vec![0.0f64; frame_size / 2 + 1];
    let mut im = vec![0.0f64; frame_size / 2 + 1];
    let mut new_spec = vec![0.0f64; half];
    let mut old_spec = vec![0.0f64; half];
    let mut beat_sd: Vec<f64> = Vec::with_capacity(beats.len() - 1);

    for i in 0..beats.len() - 1 {
        let beat_start = (f64::from(beats[i]) * ds_rate) as usize;
        let mut beat_end = (f64::from(beats[i + 1]) * ds_rate) as usize;
        if beat_end >= downsampled.len() {
            beat_end = downsampled.len().saturating_sub(1);
        }
        if beat_end < beat_start {
            beat_end = beat_start;
        }
        let beat_len = beat_end - beat_start;

        // Hann window sized to the beat extents (qm-dsp does this by hand).
        for (j, slot) in frame.iter_mut().enumerate() {
            if j < beat_len && beat_start + j < downsampled.len() {
                let mul = 0.5 * (1.0 - (2.0 * PI * j as f64 / beat_len as f64).cos());
                *slot = downsampled[beat_start + j] * mul;
            } else {
                *slot = 0.0;
            }
        }

        fft.forward(&mut frame, &mut re, &mut im);
        for (j, mag) in new_spec.iter_mut().enumerate() {
            *mag = (re[j] * re[j] + im[j] * im[j]).sqrt();
        }
        adaptive_threshold(&mut new_spec);

        if i > 0 {
            beat_sd.push(spectral_diff(&old_spec, &new_spec));
        }
        old_spec.copy_from_slice(&new_spec);
    }

    let timesig = beats_per_bar.max(1) as usize;
    let mut candidate = vec![0.0f64; timesig];
    for (beat, cand) in candidate.iter_mut().enumerate() {
        let mut count = 0usize;
        let mut example = beat as isize - 1;
        while example < beat_sd.len() as isize {
            if example >= 0 {
                *cand += beat_sd[example as usize] / timesig as f64;
                count += 1;
            }
            example += timesig as isize;
        }
        if count > 0 {
            *cand /= count as f64;
        }
    }

    let (first, _) = get_max_index(&candidate);
    let mut downbeats = Vec::new();
    let mut i = first;
    while i < beats.len() {
        downbeats.push(beats[i]);
        i += timesig;
    }
    downbeats
}

/// Decimate by [`DECIMATION`] (8 × 2), as qm-dsp chains two `Decimator`s.
fn decimate(samples: &[f64]) -> Option<Vec<f64>> {
    if samples.len() < DECIMATION {
        return None;
    }
    let mut first = Decimator::new(samples.len(), 8);
    let mut stage = vec![0.0f64; samples.len() / 8];
    first.process(samples, &mut stage);

    let mut second = Decimator::new(stage.len(), 2);
    let mut out = vec![0.0f64; stage.len() / 2];
    second.process(&stage, &mut out);
    Some(out)
}

/// qm-dsp `DownBeat::measureSpecDiff`: Jensen–Shannon divergence over the first
/// 512 bins (or a quarter of the spectrum, whichever is smaller).
fn spectral_diff(old: &[f64], new: &[f64]) -> f64 {
    let mut size = 512usize;
    if size > old.len() / 4 {
        size = old.len() / 4;
    }
    if size == 0 {
        return 0.0;
    }

    let mut new_buf = new[..size].to_vec();
    let mut old_buf = old[..size].to_vec();

    let mut sum_new = 0.0;
    let mut sum_old = 0.0;
    for i in 0..size {
        new_buf[i] += EPS;
        old_buf[i] += EPS;
        sum_new += new_buf[i];
        sum_old += old_buf[i];
    }

    let mut sd = 0.0;
    for i in 0..size {
        new_buf[i] /= sum_new;
        old_buf[i] /= sum_old;
        if new_buf[i] == 0.0 {
            new_buf[i] = 1.0;
        }
        if old_buf[i] == 0.0 {
            old_buf[i] = 1.0;
        }
        let m = 0.5 * old_buf[i] + 0.5 * new_buf[i];
        sd += -m * m.ln() + 0.5 * old_buf[i] * old_buf[i].ln() + 0.5 * new_buf[i] * new_buf[i].ln();
    }
    sd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn too_few_beats_is_empty() {
        let samples = vec![0.0f64; 44100];
        assert!(detect_downbeats(&samples, 44100, &[], 4).is_empty());
        assert!(detect_downbeats(&samples, 44100, &[0.0], 4).is_empty());
    }

    #[test]
    fn finds_the_accented_downbeat() {
        // 32 beats at 120 BPM (0.5 s). Accent beats 2, 6, 10, ... (phase 2), so
        // the detector must not simply default to beat 0.
        let sr = 44100u32;
        let beat_period = 0.5f64;
        let n_beats = 32usize;
        let accent = 2usize;
        let total = (n_beats as f64 * beat_period * f64::from(sr)) as usize;
        let mut y = vec![0.0f32; total];
        for b in 0..n_beats {
            let start = (b as f64 * beat_period * f64::from(sr)) as usize;
            let end = (((b + 1) as f64 * beat_period * f64::from(sr)) as usize).min(total);
            let freq = if b % 4 == accent { 60.0 } else { 220.0 };
            for k in start..end {
                let t = (k - start) as f64 / f64::from(sr);
                let env = (-8.0 * t).exp();
                y[k] = (0.4 * env * (2.0 * std::f64::consts::PI * freq * t).sin()) as f32;
            }
        }
        let beats: Vec<f32> = (0..n_beats)
            .map(|b| (b as f64 * beat_period) as f32)
            .collect();
        let y64: Vec<f64> = y.iter().map(|&s| f64::from(s)).collect();

        let downbeats = detect_downbeats(&y64, sr, &beats, 4);
        assert!(!downbeats.is_empty());
        // qm-dsp detects the beat with the greatest *incoming* spectral change;
        // a single accented beat changes equally on both sides, so the phase can
        // land on the accent or the beat just after it.
        let accent_t = accent as f32 * beat_period as f32;
        assert!(
            (downbeats[0] - accent_t).abs() < 1e-3
                || (downbeats[0] - (accent_t + beat_period as f32)).abs() < 1e-3,
            "expected the accent (beat {accent}) or its successor, got {}",
            downbeats[0]
        );
        for w in downbeats.windows(2) {
            assert!(
                (w[1] - w[0] - 4.0 * beat_period as f32).abs() < 1e-3,
                "downbeat spacing wrong: {:?}",
                w
            );
        }
    }
}
