//! qm-dsp `TempoTrackV2` (Davies' comb-filter/Viterbi tempo contour +
//! Ellis' dynamic-programming beat tracker), ported faithfully.
//!
//! Mixxx calls the two default entry points (`inputtempo = 120`,
//! `constraintempo = false`, `alpha = 0.9`, `tightness = 4.0`), so those are the
//! only paths ported. Beat periods are in detection-function increment units.

use crate::math::{adaptive_threshold, get_max_index};

/// qm-dsp's arbitrary small constant.
const EPS: f64 = 0.0000008;
const WV_LEN: usize = 128;
const WIN_LEN: usize = 512;
const HOP_SIZE: usize = 128;

/// `TempoTrackV2::calculateBeatPeriod` over the default rayleigh weighting.
///
/// Returns a beat period per 128-frame block, sized exactly like Mixxx's
/// `required_size / 128 + 1` (trailing entries may stay zero for the
/// exact-multiple edge case).
pub fn calculate_beat_period(df: &[f64]) -> Vec<i32> {
    // Magic constant: 60 * 44100 / 512, independent of the real sample rate
    // (qm-dsp hard-codes it).
    let rayparam = (60.0 * 44100.0 / 512.0) / 120.0;

    let mut wv = vec![0.0f64; WV_LEN];
    for (i, w) in wv.iter_mut().enumerate() {
        let fi = i as f64;
        *w = (fi / (rayparam * rayparam)) * (-(fi * fi) / (2.0 * rayparam * rayparam)).exp();
    }

    let df_len = df.len() as i64;
    let mut rcfmat: Vec<Vec<f64>> = Vec::with_capacity(df.len() / HOP_SIZE + 1);
    let mut dfframe = vec![0.0f64; WIN_LEN];
    let mut acf = vec![0.0f64; WIN_LEN];
    let mut rcf = vec![0.0f64; WV_LEN];
    let mut i: i64 = -(WIN_LEN as i64) / 2;
    while i < df_len - (WIN_LEN as i64) / 2 {
        dfframe.fill(0.0);
        rcf.fill(0.0);
        let k = if i < 0 { -i } else { 0 };
        let l = if i + WIN_LEN as i64 > df_len {
            df_len - i
        } else {
            WIN_LEN as i64
        };
        let count = (l - k) as usize;
        let src = (i + k) as usize;
        dfframe[k as usize..k as usize + count].copy_from_slice(&df[src..src + count]);
        rcfmat.push(get_rcf(&mut dfframe, &wv, &mut acf, &mut rcf));
        i += HOP_SIZE as i64;
    }

    let mut beat_period = vec![0i32; df.len() / HOP_SIZE + 1];
    viterbi_decode(&rcfmat, &wv, &mut beat_period);
    beat_period
}

/// `TempoTrackV2::get_rcf`: autocorrelation + resonator comb filter bank,
/// adaptive-thresholded and normalised to unit sum.
///
/// `dfframe`, `acf` and `rcf` are caller-owned scratch so the ~95 blocks of a
/// run share one allocation each. `acf` is fully overwritten by the lag loop;
/// `rcf` accumulates into zeroed scratch, which the caller refills per block.
fn get_rcf(dfframe: &mut [f64], wv: &[f64], acf: &mut [f64], rcf: &mut [f64]) -> Vec<f64> {
    debug_assert_eq!(dfframe.len(), WIN_LEN);
    debug_assert_eq!(acf.len(), WIN_LEN);
    debug_assert_eq!(rcf.len(), WV_LEN);
    adaptive_threshold(dfframe);

    let len = dfframe.len();
    debug_assert!(acf.len() >= len, "autocorrelation scratch too small");
    let acf = &mut acf[..len];
    for lag in 0..len {
        let mut sum = 0.0;
        for n in 0..(len - lag) {
            sum += dfframe[n] * dfframe[n + lag];
        }
        acf[lag] = sum / (len - lag) as f64;
    }

    let numelem: i64 = 4;
    for i in 2..rcf.len() {
        for a in 1..=numelem {
            for b in (1 - a)..a {
                let idx = (a * i as i64 + b) - 1;
                rcf[i - 1] += acf[idx as usize] * wv[i - 1] / (2.0 * a as f64 - 1.0);
            }
        }
    }

    adaptive_threshold(rcf);

    for v in rcf.iter_mut() {
        *v += EPS;
    }
    let rcfsum: f64 = rcf.iter().sum();
    for v in rcf.iter_mut() {
        *v /= rcfsum + EPS;
    }
    rcf.to_vec()
}

/// `TempoTrackV2::viterbi_decode`.
fn viterbi_decode(rcfmat: &[Vec<f64>], wv: &[f64], beat_period: &mut [i32]) {
    if rcfmat.len() < 2 {
        return;
    }
    let t_len = rcfmat.len();
    let q = rcfmat[0].len();

    // Gaussian transition matrix; only the 20..Q-20 window is filled.
    let mut tmat = vec![0.0f64; q * q];
    let sigma = 8.0;
    for i in 20..q.saturating_sub(20) {
        for j in 20..q.saturating_sub(20) {
            let mu = i as f64;
            tmat[i * q + j] = (-((j as f64 - mu).powi(2)) / (2.0 * sigma * sigma)).exp();
        }
    }

    let mut delta = vec![0.0f64; t_len * q];
    let mut psi = vec![0usize; t_len * q];

    for j in 0..q {
        delta[j] = wv[j] * rcfmat[0][j];
    }
    let mut deltasum = 0.0;
    for d in delta[..q].iter() {
        deltasum += *d;
    }
    for d in delta[..q].iter_mut() {
        *d /= deltasum + EPS;
    }

    // Only the `20..q-20` transition band is non-zero, and `delta` is
    // non-negative, so products outside it are `0.0` and can never beat a
    // running max seeded at `0.0` with strict `>`. Skipping them is exact.
    let lo = 20usize;
    let hi = q.saturating_sub(20);
    for t in 1..t_len {
        let (head, tail) = delta.split_at_mut(t * q);
        let prev = &head[(t - 1) * q..t * q];
        let curr = &mut tail[..q];
        for j in 0..q {
            let mut best = 0.0f64;
            let mut arg = 0usize;
            for i in lo..hi {
                // Flat index matches the original 2D access `tmat[j][i]`
                // (destination `j`, source `i`). Population wrote
                // `tmat[i * q + j]`, so this read looks transposed — it is not.
                let v = prev[i] * tmat[j * q + i];
                if v > best {
                    best = v;
                    arg = i;
                }
            }
            curr[j] = best * rcfmat[t][j];
            psi[t * q + j] = arg;
        }
        let ds: f64 = curr.iter().sum();
        for d in curr.iter_mut() {
            *d /= ds + EPS;
        }
    }

    let (ind, _) = crate::math::get_max_index(&delta[(t_len - 1) * q..t_len * q]);
    beat_period[t_len - 1] = ind as i32;

    let mut t = t_len as i64 - 2;
    while t > 0 {
        let tu = t as usize;
        let source = psi[(t + 1) as usize * q + beat_period[tu + 1] as usize];
        beat_period[tu] = source as i32;
        t -= 1;
    }
    // qm-dsp's "weird but necessary" final step.
    beat_period[0] = psi[q + beat_period[1] as usize] as i32;
}

/// `TempoTrackV2::calculateBeats` with `alpha = 0.9`, `tightness = 4.0`.
///
/// Returns beat positions in detection-function increment units.
pub fn calculate_beats(df: &[f64], beat_period: &[i32]) -> Vec<f64> {
    if df.is_empty() || beat_period.is_empty() {
        return Vec::new();
    }
    let df_len = df.len();
    let mut cumscore = vec![0.0f64; df_len];
    let mut backlink = vec![-1i64; df_len];

    let alpha = 0.9;
    let tightness = 4.0;
    let mut old_period = 0i32;
    let mut txwt: Vec<f64> = Vec::new();
    let mut txwt_len = 0usize;

    for i in 0..df_len {
        let period = beat_period[i / HOP_SIZE];
        let prange_min = period * -2;
        if period != old_period {
            old_period = period;
            let prange_max = period / -2;
            txwt_len = (prange_max - prange_min + 1).max(0) as usize;
            txwt.clear();
            txwt.reserve(txwt_len);
            for j in 0..txwt_len {
                let mu = period as f64;
                let arg = ((2.0 * mu).round() - j as f64) / mu;
                txwt.push((-0.5 * (tightness * arg.ln()).powi(2)).exp());
            }
        }

        let mut vv = 0.0;
        let mut xx = 0i64;
        for j in 0..txwt_len {
            let cscore_ind = i as i64 + prange_min as i64 + j as i64;
            if cscore_ind >= 0 {
                let scorecands = txwt[j] * cumscore[cscore_ind as usize];
                if scorecands > vv {
                    vv = scorecands;
                    xx = cscore_ind;
                }
            }
        }

        cumscore[i] = alpha * vv + (1.0 - alpha) * df[i];
        backlink[i] = xx;
    }

    let last_period = beat_period[beat_period.len() - 1];
    let start_i = df_len as i64 - last_period as i64;
    let mut tmp = Vec::new();
    let mut i = start_i;
    while i < df_len as i64 {
        if i >= 0 {
            tmp.push(cumscore[i as usize]);
        }
        i += 1;
    }
    let mut startpoint = get_max_index(&tmp).0 as i64 + start_i;
    if startpoint < 0 {
        startpoint = 0;
    }
    if startpoint as usize >= backlink.len() {
        startpoint = backlink.len() as i64 - 1;
    }

    let mut ibeats = vec![startpoint as usize];
    while ibeats.len() <= df_len {
        let b = *ibeats.last().unwrap();
        let bl = backlink[b];
        if bl <= 0 || bl as usize == b {
            break;
        }
        ibeats.push(bl as usize);
    }

    ibeats.iter().rev().map(|&b| b as f64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_onset_train_yields_a_plausible_period() {
        // Clicks every 43 df-frames (~120 BPM at the qm-dsp frame rate).
        let mut df = vec![0.0f64; 128 * 40];
        for i in (0..df.len()).step_by(43) {
            df[i] = 1.0;
        }
        let bp = calculate_beat_period(&df);
        assert!(!bp.is_empty());
        let nonzero: Vec<i32> = bp.iter().copied().filter(|&p| p != 0).collect();
        assert!(!nonzero.is_empty());
        let beats = calculate_beats(&df, &bp);
        assert!(beats.len() > 1);
    }
}
