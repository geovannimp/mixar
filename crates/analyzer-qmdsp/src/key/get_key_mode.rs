//! qm-dsp `GetKeyMode`: decimate → CQT chromagram → HPCP mean → Krumhansl
//! profile correlation → median filter. Key indices follow Mixxx
//! (`1..=12` major C..B, `13..=24` minor C..B, `0` = none).

use crate::key::cqt::{Chromagram, ChromagramConfig};
use crate::math::{get_max_index, mean, pitch_frequency};

const BINS_PER_OCTAVE: usize = 36;
/// Raw samples per key block: the chromagram frame times this decimation.
const DECIMATION_FACTOR: usize = 8;

#[rustfmt::skip]
const MAJ_PROFILE: [f64; BINS_PER_OCTAVE] = [
    0.0384, 0.0629, 0.0258, 0.0121, 0.0146, 0.0106, 0.0364, 0.0610, 0.0267,
    0.0126, 0.0121, 0.0086, 0.0364, 0.0623, 0.0279, 0.0275, 0.0414, 0.0186,
    0.0173, 0.0248, 0.0145, 0.0364, 0.0631, 0.0262, 0.0129, 0.0150, 0.0098,
    0.0312, 0.0521, 0.0235, 0.0129, 0.0142, 0.0095, 0.0289, 0.0478, 0.0239,
];

#[rustfmt::skip]
const MIN_PROFILE: [f64; BINS_PER_OCTAVE] = [
    0.0375, 0.0682, 0.0299, 0.0119, 0.0138, 0.0093, 0.0296, 0.0543, 0.0257,
    0.0292, 0.0519, 0.0246, 0.0159, 0.0234, 0.0135, 0.0291, 0.0544, 0.0248,
    0.0137, 0.0176, 0.0104, 0.0352, 0.0670, 0.0302, 0.0222, 0.0349, 0.0164,
    0.0174, 0.0297, 0.0166, 0.0222, 0.0401, 0.0202, 0.0175, 0.0270, 0.0146,
];

pub struct GetKeyMode {
    bpo: usize,
    chroma: Chromagram,
    chroma_frame_size: usize,
    chroma_buffer_size: usize,
    chroma_buffer: Vec<f64>,
    chroma_buffer_filling: usize,
    buffer_index: usize,
    mean_hpcp: [f64; BINS_PER_OCTAVE],
    maj_corr: [f64; BINS_PER_OCTAVE],
    min_corr: [f64; BINS_PER_OCTAVE],
    maj_profile_norm: [f64; BINS_PER_OCTAVE],
    min_profile_norm: [f64; BINS_PER_OCTAVE],
    median_win_size: usize,
    median_buffer: Vec<i32>,
    sorted_buffer: Vec<i32>,
    median_buffer_filling: usize,
}

impl GetKeyMode {
    pub fn new(sample_rate: u32) -> Self {
        let hpcp_average = 10.0;
        let median_average = 10.0;
        let tuning_frequency = 440.0;

        let fs = sample_rate as f64 / DECIMATION_FACTOR as f64;
        let cents_offset = -12.0 / BINS_PER_OCTAVE as f64 * 100.0;
        let f_min = pitch_frequency(48, cents_offset, tuning_frequency);
        let f_max = pitch_frequency(96, cents_offset, tuning_frequency);

        let chroma = Chromagram::new(ChromagramConfig {
            fs,
            min: f_min,
            max: f_max,
            bpo: BINS_PER_OCTAVE,
            cq_thresh: 0.0054,
        });
        let chroma_frame_size = chroma.frame_size();
        let chroma_buffer_size = (hpcp_average * fs / chroma_frame_size as f64).ceil() as usize;
        let median_win_size = (median_average * fs / chroma_frame_size as f64).ceil() as usize;

        let maj_mean = mean(&MAJ_PROFILE);
        let min_mean = mean(&MIN_PROFILE);
        let mut maj_profile_norm = [0.0f64; BINS_PER_OCTAVE];
        let mut min_profile_norm = [0.0f64; BINS_PER_OCTAVE];
        for i in 0..BINS_PER_OCTAVE {
            maj_profile_norm[i] = MAJ_PROFILE[i] - maj_mean;
            min_profile_norm[i] = MIN_PROFILE[i] - min_mean;
        }

        Self {
            bpo: BINS_PER_OCTAVE,
            chroma,
            chroma_frame_size,
            chroma_buffer_size,
            chroma_buffer: vec![0.0; BINS_PER_OCTAVE * chroma_buffer_size],
            chroma_buffer_filling: 0,
            buffer_index: 0,
            mean_hpcp: [0.0; BINS_PER_OCTAVE],
            maj_corr: [0.0; BINS_PER_OCTAVE],
            min_corr: [0.0; BINS_PER_OCTAVE],
            maj_profile_norm,
            min_profile_norm,
            median_win_size,
            median_buffer: vec![0; median_win_size],
            sorted_buffer: vec![0; median_win_size],
            median_buffer_filling: 0,
        }
    }

    /// Decimated samples per `process` frame.
    pub fn frame_size(&self) -> usize {
        self.chroma_frame_size
    }

    /// Raw samples one key block covers: the chromagram frame times the ×8
    /// decimation. `overlap::feed` frames the key path in these.
    pub fn block_size(&self) -> usize {
        self.chroma_frame_size * DECIMATION_FACTOR
    }

    /// Length of the zero-ended input the shared ×8 pass must cover for the key
    /// path to stay bit-identical: `overlap::feed`'s whole key window sequence —
    /// `block_size/2` of centred zero pre-roll, the track, then the silence its
    /// `finalize` appends to complete the last window.
    ///
    /// The per-block `Decimator` this shared pass replaced filtered exactly that
    /// concatenation, so the final chroma frame carried the filter's ring-out.
    /// Sweeping only the track would hand that frame hard zeros instead, and on
    /// clips shorter than one block it would be the only frame.
    pub fn decimation_input_len(&self, track_len: usize) -> usize {
        let block = self.block_size();
        let preroll = block / 2;
        // One window per `block` accumulated, plus the one `finalize` always
        // emits on top of them.
        (preroll + track_len) / block * block + block
    }

    /// Process one block of `chroma_frame_size` decimated samples; returns the
    /// (median-filtered) key index.
    pub fn process(&mut self, dec: &[f64]) -> i32 {
        debug_assert_eq!(dec.len(), self.chroma_frame_size, "decimated frame length");

        let mut chroma_vals = [0.0f64; BINS_PER_OCTAVE];
        chroma_vals.copy_from_slice(self.chroma.process(dec));

        for j in 0..self.bpo {
            self.chroma_buffer[self.buffer_index * self.bpo + j] = chroma_vals[j];
        }

        let old_index = self.buffer_index;
        self.buffer_index += 1;
        if old_index >= self.chroma_buffer_size - 1 {
            self.buffer_index = 0;
        }

        let old_filling = self.chroma_buffer_filling;
        self.chroma_buffer_filling += 1;
        if old_filling >= self.chroma_buffer_size {
            self.chroma_buffer_filling = self.chroma_buffer_size;
        }

        for k in 0..self.bpo {
            let mut mn = 0.0;
            for j in 0..self.chroma_buffer_filling {
                mn += self.chroma_buffer[k + j * self.bpo];
            }
            self.mean_hpcp[k] = mn / self.chroma_buffer_filling as f64;
        }

        let mhpcp = mean(&self.mean_hpcp);
        for k in 0..self.bpo {
            self.mean_hpcp[k] -= mhpcp;
        }

        for k in 0..self.bpo {
            self.maj_corr[k] = krum_corr(&self.mean_hpcp, &self.maj_profile_norm, k as i32 - 1);
            self.min_corr[k] = krum_corr(&self.mean_hpcp, &self.min_profile_norm, k as i32 - 1);
        }

        let (max_maj_bin, max_maj) = get_max_index(&self.maj_corr);
        let (max_min_bin, max_min) = get_max_index(&self.min_corr);
        let max_bin = if max_maj > max_min {
            max_maj_bin
        } else {
            max_min_bin + BINS_PER_OCTAVE
        };
        let mut key = (max_bin / 3 + 1) as i32;

        let old_median = self.median_buffer_filling;
        self.median_buffer_filling += 1;
        if old_median >= self.median_win_size {
            self.median_buffer_filling = self.median_win_size;
        }

        for k in 1..self.median_win_size {
            self.median_buffer[k - 1] = self.median_buffer[k];
        }
        self.median_buffer[self.median_win_size - 1] = key;

        for k in 0..self.median_win_size {
            self.sorted_buffer[k] = self.median_buffer[self.median_win_size - 1 - k];
        }
        let filling = self.median_buffer_filling;
        self.sorted_buffer[..filling].sort_unstable();

        let mut midpoint = ((filling as f64) / 2.0).ceil() as usize;
        if midpoint == 0 {
            midpoint = 1;
        }
        key = self.sorted_buffer[midpoint - 1];

        key
    }
}

/// qm-dsp `GetKeyMode::krumCorr` (circular cross-correlation of zero-mean
/// vectors).
fn krum_corr(data: &[f64], profile: &[f64], shift_profile: i32) -> f64 {
    let len = data.len();
    let mut num = 0.0;
    let mut sum1 = 0.0;
    let mut sum2 = 0.0;
    for i in 0..len {
        let k = (i as i32 - shift_profile + len as i32).rem_euclid(len as i32) as usize;
        num += data[i] * profile[k];
        sum1 += data[i] * data[i];
        sum2 += profile[k] * profile[k];
    }
    let den = (sum1 * sum2).sqrt();
    if den > 0.0 {
        num / den
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_matches_mixxx_defaults() {
        let gkm = GetKeyMode::new(44100);
        // chroma frame size 4096, decimation 8.
        assert_eq!(gkm.frame_size(), 4096);
    }

    #[test]
    fn silence_does_not_panic() {
        let mut gkm = GetKeyMode::new(44100);
        let block = vec![0.0; gkm.frame_size()];
        let key = gkm.process(&block);
        assert!((0..=24).contains(&key));
    }
}
