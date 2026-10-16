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
use crate::key::decimator::{Decimation, Decimator};
use crate::math::{adaptive_threshold, get_max_index, next_power_of_two};

/// qm-dsp `MathAliases::EPS`.
const EPS: f64 = 2.2204e-16;

/// Decimation factor (8 × 2), matching qm-dsp's ~2.7 kHz at 44.1/48 kHz.
const DECIMATION: usize = 16;

/// Return downbeat positions (seconds) among `beats` (seconds), for a given
/// meter. Empty when there are too few beats or when the decimated stream is
/// shorter than one beat's worth of samples (`dec8.len() < 2`, which is exactly
/// the old `samples.len() < 16` guard on the raw track).
///
/// `dec8` is the caller's ×8-decimated stream (shared with the key path); this
/// adds the ×2 stage for qm-dsp's ~1/16 rate.
pub fn detect_downbeats(
    dec8: &[f64],
    sample_rate: u32,
    beats: &[f32],
    beats_per_bar: u8,
) -> Vec<f32> {
    if beats.len() < 2 || dec8.len() < 2 {
        return Vec::new();
    }
    let mut stage = Decimator::new(Decimation::By2, dec8.len());
    let mut downsampled = vec![0.0; dec8.len() / 2];
    stage.process(dec8, &mut downsampled);
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
    // Hann window sized to the beat extents; rebuilt only when the extents
    // actually change (qm-dsp does this by hand, per beat).
    let mut hann_cache: Vec<f64> = Vec::new();
    let mut hann_len = 0usize;

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

        if hann_len != beat_len {
            // Reuse the allocation: beat lengths usually vary, so a fresh
            // `collect()` per beat would undo the cache. `beat_len == 0` is
            // an empty range (the closure never runs — no division by zero).
            hann_cache.clear();
            hann_cache.extend(
                (0..beat_len).map(|j| 0.5 * (1.0 - (2.0 * PI * j as f64 / beat_len as f64).cos())),
            );
            hann_len = beat_len;
        }
        let live = beat_len
            .min(frame_size)
            .min(downsampled.len().saturating_sub(beat_start));
        for j in 0..live {
            frame[j] = downsampled[beat_start + j] * hann_cache[j];
        }
        frame[live..].fill(0.0);

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

/// qm-dsp `DownBeat::measureSpecDiff`: Jensen–Shannon divergence over the first
/// 512 bins (or a quarter of the spectrum, whichever is smaller).
///
/// Two passes instead of two `to_vec()` buffers per beat: the sums first, then
/// the divergence. The operand order of the final expression is unchanged, so
/// the result is bit-identical.
fn spectral_diff(old: &[f64], new: &[f64]) -> f64 {
    let mut size = 512usize;
    if size > old.len() / 4 {
        size = old.len() / 4;
    }
    if size == 0 {
        return 0.0;
    }
    let old = &old[..size];
    let new = &new[..size];

    let mut sum_new = 0.0;
    let mut sum_old = 0.0;
    for i in 0..size {
        sum_new += new[i] + EPS;
        sum_old += old[i] + EPS;
    }

    let mut sd = 0.0;
    for i in 0..size {
        let nb = (new[i] + EPS) / sum_new;
        let ob = (old[i] + EPS) / sum_old;
        let nb = if nb == 0.0 { 1.0 } else { nb };
        let ob = if ob == 0.0 { 1.0 } else { ob };
        let m = 0.5 * ob + 0.5 * nb;
        sd += -m * m.ln() + 0.5 * ob * ob.ln() + 0.5 * nb * nb.ln();
    }
    sd
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The old `detect_downbeats` took the raw track and ran the private
    /// `decimate` (which bailed out on `samples.len() < 16`); it now takes the
    /// caller's ×8 stream and runs only the ×2 stage. This pins that the
    /// acceptance threshold and the downsampled stream are unchanged.
    #[test]
    fn x8_stream_matches_the_old_two_stage_decimate() {
        for &len in &[
            1usize, 7, 15, 16, 17, 31, 32, 33, 63, 64, 1000, 44_100, 100_000,
        ] {
            let y: Vec<f64> = (0..len)
                .map(|i| {
                    let t = i as f64;
                    (t * 0.037).sin() * 0.6 + (t * 0.11).cos() * 0.3
                })
                .collect();

            // Old: `decimate(y)` — bails out below 16 samples.
            let old: Option<Vec<f64>> = if y.len() < 16 {
                None
            } else {
                let mut first = Decimator::new(Decimation::By8, y.len());
                let mut stage = vec![0.0; y.len() / 8];
                first.process(&y, &mut stage);
                let mut second = Decimator::new(Decimation::By2, stage.len());
                let mut out = vec![0.0; stage.len() / 2];
                second.process(&stage, &mut out);
                Some(out)
            };
            // New: the caller's ×8 stream, then the ×2 stage here.
            let dec8_len = y.len() / 8;
            let new: Option<Vec<f64>> = if dec8_len < 2 {
                None
            } else {
                let mut d = Decimator::new(Decimation::By8, y.len());
                let mut dec8 = vec![0.0; dec8_len];
                d.process(&y, &mut dec8);
                let mut second = Decimator::new(Decimation::By2, dec8.len());
                let mut out = vec![0.0; dec8.len() / 2];
                second.process(&dec8, &mut out);
                Some(out)
            };

            // The two decide the same way, for the same reason: `dec8.len() < 2`
            // is exactly `y.len() < 16`.
            assert_eq!(
                old.is_none(),
                new.is_none(),
                "acceptance differs at len {len}: old={:?} new={:?}",
                old.is_none(),
                new.is_none()
            );
            if let (Some(a), Some(b)) = (old, new) {
                assert_eq!(a.len(), b.len(), "downsampled length differs at len {len}");
                for (i, (&x, &y)) in a.iter().zip(b.iter()).enumerate() {
                    assert_eq!(
                        x.to_bits(),
                        y.to_bits(),
                        "downsampled[{i}] differs at len {len}: {x} vs {y}"
                    );
                }
            }
        }
    }

    /// The smallest ×8 stream the production guard accepts. Base accepted this
    /// too (a 16-sample track), and it must not panic.
    #[test]
    fn smallest_accepted_stream_does_not_panic() {
        // One downsampled sample and two beats: the degenerate case both base
        // and the current guard accept.
        let dec8 = vec![0.5f64; 2];
        let beats = [0.0f32, 0.5];
        let out = detect_downbeats(&dec8, 44_100, &beats, 4);
        assert_eq!(out, vec![0.0], "phase 0 of a 4-beat bar over 2 beats");
        // One below the threshold is rejected outright.
        assert!(detect_downbeats(&[0.5], 44_100, &beats, 4).is_empty());
    }

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

        let mut d8 = Decimator::new(Decimation::By8, y64.len());
        let mut dec8 = vec![0.0; y64.len() / 8];
        d8.process(&y64, &mut dec8);
        let downbeats = detect_downbeats(&dec8, sr, &beats, 4);
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
