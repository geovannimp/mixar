//! Port of Mixxx's `BeatUtils` (`src/track/beatutils.cpp`).
//!
//! Mixxx does not report the raw detector tempo. It irons the QM detector's
//! ±12 ms (512-frame) jitter into constant-tempo regions, averages a long stable
//! region to a precise beat length, and snaps the result to a musically
//! plausible integer/fraction ([`round_bpm_within_range`]). Reproducing this is
//! what makes the backend report the same BPM Mixxx displays — the raw detector
//! can only produce quantised tempos (e.g. 123.04 / 126.05 BPM near 120), and a
//! median locks onto the wrong one.
//!
//! Beats are frame positions; the sample rate is in Hz.

// Faithful index-based port of the C++.
#![allow(clippy::needless_range_loop)]

/// 25 ms — inaudible, and > 2× the QM detector's 12 ms step.
const MAX_SECS_PHASE_ERROR: f64 = 0.025;
/// Avoid using a constant region across an offset shift.
const MAX_SECS_PHASE_ERROR_SUM: f64 = 0.1;
const MAX_OUTLIERS_COUNT: i64 = 1;
const MIN_REGION_BEAT_COUNT: i64 = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstRegion {
    pub first_beat: f64,
    pub beat_length: f64,
}

/// `BeatUtils::calculateBpm`: coarse beats (frames) → BPM (`None` if it cannot
/// be inferred).
pub fn calculate_bpm(beats: &[f64], sample_rate: f64) -> Option<f64> {
    if beats.len() < 2 || !sample_rate.is_finite() || sample_rate <= 0.0 {
        return None;
    }
    if (beats.len() as i64) < MIN_REGION_BEAT_COUNT {
        return calculate_average_bpm(
            beats.len() as i64 - 1,
            sample_rate,
            beats[0],
            *beats.last().unwrap(),
        );
    }
    let regions = retrieve_const_regions(beats, sample_rate);
    make_const_bpm(&regions, sample_rate)
}

/// Convenience for backends whose beats are in **seconds** (`f32` or `f64`).
/// Converts to frame positions and runs [`calculate_bpm`].
pub fn calculate_bpm_from_seconds<T>(beats: &[T], sample_rate: u32) -> Option<f64>
where
    T: Into<f64> + Copy,
{
    if beats.len() < 2 {
        return None;
    }
    let sr = f64::from(sample_rate.max(1));
    let frames: Vec<f64> = beats.iter().map(|&b| b.into() * sr).collect();
    calculate_bpm(&frames, sr)
}

/// `BeatUtils::calculateAverageBpm`.
pub fn calculate_average_bpm(
    number_of_beats: i64,
    sample_rate: f64,
    lower: f64,
    upper: f64,
) -> Option<f64> {
    let frames = upper - lower;
    if frames <= 0.0 || number_of_beats < 1 {
        return None;
    }
    valid_bpm(60.0 * number_of_beats as f64 * sample_rate / frames)
}

/// `BeatUtils::retrieveConstRegions`.
pub fn retrieve_const_regions(coarse_beats: &[f64], sample_rate: f64) -> Vec<ConstRegion> {
    let mut regions = Vec::new();
    let n = coarse_beats.len();
    if n < 2 {
        return regions;
    }
    let max_phase_error = MAX_SECS_PHASE_ERROR * sample_rate;
    let max_phase_error_sum = MAX_SECS_PHASE_ERROR_SUM * sample_rate;

    let mut left_index: i64 = 0;
    let mut right_index: i64 = n as i64 - 1;
    while left_index < n as i64 - 1 {
        if right_index <= left_index {
            break;
        }
        let mean_beat_length = (coarse_beats[right_index as usize]
            - coarse_beats[left_index as usize])
            / (right_index - left_index) as f64;
        let mut outliers_count: i64 = 0;
        let mut ironed_beat = coarse_beats[left_index as usize];
        let mut phase_error_sum = 0.0;
        let mut i = left_index + 1;
        while i <= right_index {
            ironed_beat += mean_beat_length;
            let phase_error = ironed_beat - coarse_beats[i as usize];
            phase_error_sum += phase_error;
            if phase_error.abs() > max_phase_error {
                outliers_count += 1;
                // The first beat must not be an outlier.
                if outliers_count > MAX_OUTLIERS_COUNT || i == left_index + 1 {
                    break;
                }
            }
            if phase_error_sum.abs() > max_phase_error_sum {
                break;
            }
            i += 1;
        }
        if i > right_index {
            // Reject a region whose first and last beats are correction beats in
            // the same direction (that would bend the mean away from the optimum).
            let mut region_border_error = 0.0;
            if right_index > left_index + 2 {
                let first_beat_length =
                    coarse_beats[(left_index + 1) as usize] - coarse_beats[left_index as usize];
                let last_beat_length =
                    coarse_beats[right_index as usize] - coarse_beats[(right_index - 1) as usize];
                region_border_error =
                    (first_beat_length + last_beat_length - 2.0 * mean_beat_length).abs();
            }
            if region_border_error < max_phase_error / 2.0 {
                regions.push(ConstRegion {
                    first_beat: coarse_beats[left_index as usize],
                    beat_length: mean_beat_length,
                });
                left_index = right_index;
                right_index = n as i64 - 1;
                continue;
            }
        }
        right_index -= 1;
    }

    // Zero-length final region marking the end.
    regions.push(ConstRegion {
        first_beat: *coarse_beats.last().unwrap(),
        beat_length: 0.0,
    });
    regions
}

/// `BeatUtils::makeConstBpm` (`pFirstBeat == nullptr`).
pub fn make_const_bpm(regions: &[ConstRegion], sample_rate: f64) -> Option<f64> {
    if regions.is_empty() {
        return None;
    }

    // Longest region, somewhere in the middle, to start with.
    let mut mid_region_index = 0usize;
    let mut longest_region_length = 0.0;
    let mut longest_region_beat_length = 0.0;
    for i in 0..regions.len() - 1 {
        let length = regions[i + 1].first_beat - regions[i].first_beat;
        if length > longest_region_length {
            longest_region_length = length;
            longest_region_beat_length = regions[i].beat_length;
            mid_region_index = i;
        }
    }
    if longest_region_length == 0.0 {
        return None;
    }

    let mut longest_region_number_of_beats =
        ((longest_region_length / longest_region_beat_length) + 0.5) as i64;
    let mut longest_region_beat_length_min = longest_region_beat_length
        - ((MAX_SECS_PHASE_ERROR * sample_rate) / longest_region_number_of_beats as f64);
    let mut longest_region_beat_length_max = longest_region_beat_length
        + ((MAX_SECS_PHASE_ERROR * sample_rate) / longest_region_number_of_beats as f64);
    let mut start_region_index = mid_region_index;

    // Extend to a similar region at the beginning.
    for i in 0..mid_region_index {
        let length = regions[i + 1].first_beat - regions[i].first_beat;
        let number_of_beats = ((length / regions[i].beat_length) + 0.5) as i64;
        if number_of_beats < MIN_REGION_BEAT_COUNT {
            continue;
        }
        let this_min = regions[i].beat_length
            - ((MAX_SECS_PHASE_ERROR * sample_rate) / number_of_beats as f64);
        let this_max = regions[i].beat_length
            + ((MAX_SECS_PHASE_ERROR * sample_rate) / number_of_beats as f64);
        if longest_region_beat_length > this_min && longest_region_beat_length < this_max {
            let new_longest_region_length =
                regions[mid_region_index + 1].first_beat - regions[i].first_beat;
            let beat_length_min = longest_region_beat_length_min.max(this_min);
            let beat_length_max = longest_region_beat_length_max.min(this_max);
            let max_number_of_beats = (new_longest_region_length / beat_length_min).round() as i64;
            let min_number_of_beats = (new_longest_region_length / beat_length_max).round() as i64;
            if min_number_of_beats != max_number_of_beats {
                continue;
            }
            let number_of_beats = min_number_of_beats;
            let new_beat_length = new_longest_region_length / number_of_beats as f64;
            if new_beat_length > longest_region_beat_length_min
                && new_beat_length < longest_region_beat_length_max
            {
                longest_region_length = new_longest_region_length;
                longest_region_beat_length = new_beat_length;
                longest_region_number_of_beats = number_of_beats;
                longest_region_beat_length_min = longest_region_beat_length
                    - ((MAX_SECS_PHASE_ERROR * sample_rate)
                        / longest_region_number_of_beats as f64);
                longest_region_beat_length_max = longest_region_beat_length
                    + ((MAX_SECS_PHASE_ERROR * sample_rate)
                        / longest_region_number_of_beats as f64);
                start_region_index = i;
                break;
            }
        }
    }

    // Extend to a similar region at the end (note: min/max are *not* updated
    // inside this loop, matching the C++).
    let mut i = regions.len() as i64 - 2;
    while i > mid_region_index as i64 {
        let idx = i as usize;
        let length = regions[idx + 1].first_beat - regions[idx].first_beat;
        let number_of_beats = ((length / regions[idx].beat_length) + 0.5) as i64;
        if number_of_beats >= MIN_REGION_BEAT_COUNT {
            let this_min = regions[idx].beat_length
                - ((MAX_SECS_PHASE_ERROR * sample_rate) / number_of_beats as f64);
            let this_max = regions[idx].beat_length
                + ((MAX_SECS_PHASE_ERROR * sample_rate) / number_of_beats as f64);
            if longest_region_beat_length > this_min && longest_region_beat_length < this_max {
                let new_longest_region_length =
                    regions[idx + 1].first_beat - regions[start_region_index].first_beat;
                let min_beat_length = longest_region_beat_length_min.max(this_min);
                let max_beat_length = longest_region_beat_length_max.min(this_max);
                let max_number_of_beats =
                    (new_longest_region_length / min_beat_length).round() as i64;
                let min_number_of_beats =
                    (new_longest_region_length / max_beat_length).round() as i64;
                if min_number_of_beats == max_number_of_beats {
                    let number_of_beats = min_number_of_beats;
                    let new_beat_length = new_longest_region_length / number_of_beats as f64;
                    if new_beat_length > longest_region_beat_length_min
                        && new_beat_length < longest_region_beat_length_max
                    {
                        longest_region_beat_length = new_beat_length;
                        longest_region_number_of_beats = number_of_beats;
                        break;
                    }
                }
            }
        }
        i -= 1;
    }
    let _ = longest_region_length;

    longest_region_beat_length_min = longest_region_beat_length
        - ((MAX_SECS_PHASE_ERROR * sample_rate) / longest_region_number_of_beats as f64);
    longest_region_beat_length_max = longest_region_beat_length
        + ((MAX_SECS_PHASE_ERROR * sample_rate) / longest_region_number_of_beats as f64);

    let min_round_bpm = 60.0 * sample_rate / longest_region_beat_length_max;
    let max_round_bpm = 60.0 * sample_rate / longest_region_beat_length_min;
    let center_bpm = 60.0 * sample_rate / longest_region_beat_length;

    round_bpm_within_range(min_round_bpm, center_bpm, max_round_bpm)
        .filter(|bpm| is_valid_bpm(*bpm))
}

/// `BeatUtils::roundBpmWithinRange`.
pub fn round_bpm_within_range(min_bpm: f64, center_bpm: f64, max_bpm: f64) -> Option<f64> {
    if !is_valid_bpm(min_bpm) || !is_valid_bpm(center_bpm) || !is_valid_bpm(max_bpm) {
        return Some(center_bpm);
    }

    // Full integer BPM first.
    if let Some(bpm) = try_snap(min_bpm, center_bpm, max_bpm, 1.0) {
        return Some(bpm);
    }

    if center_bpm < 85.0 {
        if let Some(bpm) = try_snap(min_bpm, center_bpm, max_bpm, 2.0) {
            return Some(bpm);
        }
    }

    if center_bpm > 127.0 {
        // 2/3 going down to 85.
        if let Some(bpm) = try_snap(min_bpm, center_bpm, max_bpm, 2.0 / 3.0) {
            return Some(bpm);
        }
    }

    // 1/3 BPM (3/2 and 3/4 multipliers).
    if let Some(bpm) = try_snap(min_bpm, center_bpm, max_bpm, 3.0) {
        return Some(bpm);
    }

    // 1/12 BPM (other typical multipliers).
    if let Some(bpm) = try_snap(min_bpm, center_bpm, max_bpm, 12.0) {
        return Some(bpm);
    }

    Some(center_bpm)
}

fn try_snap(min_bpm: f64, center_bpm: f64, max_bpm: f64, fraction: f64) -> Option<f64> {
    let snap_bpm = (center_bpm * fraction).round() / fraction;
    if snap_bpm > min_bpm && snap_bpm < max_bpm {
        Some(snap_bpm)
    } else {
        None
    }
}

fn is_valid_bpm(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn valid_bpm(value: f64) -> Option<f64> {
    is_valid_bpm(value).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_constant_train_keeps_its_tempo() {
        let sr = 44100.0;
        let period = 60.0 * sr / 120.0;
        let beats: Vec<f64> = (0..200).map(|i| i as f64 * period).collect();
        let bpm = calculate_bpm(&beats, sr).unwrap();
        assert!((bpm - 120.0).abs() < 0.5, "bpm = {bpm}");
    }

    #[test]
    fn snaps_to_integer_when_within_tolerance() {
        // 124.0 BPM with ±12 ms jitter (the QM detector step) → 124.
        let sr = 44100.0;
        let period = 60.0 * sr / 124.0;
        let step = 0.01161 * sr;
        let beats: Vec<f64> = (0..300)
            .map(|i| {
                let jitter = match i % 5 {
                    0 => -step / 2.0,
                    3 => step / 2.0,
                    _ => 0.0,
                };
                i as f64 * period + jitter
            })
            .collect();
        let bpm = calculate_bpm(&beats, sr).unwrap();
        assert!((bpm - 124.0).abs() < 0.05, "bpm = {bpm}");
    }

    #[test]
    fn short_train_uses_average() {
        let sr = 44100.0;
        let period = 0.5 * sr;
        let beats: Vec<f64> = (0..8).map(|i| i as f64 * period).collect();
        let bpm = calculate_bpm(&beats, sr).unwrap();
        assert!((bpm - 120.0).abs() < 1e-9, "bpm = {bpm}");
    }
}
