//! Spike-quality musical key estimation.
//!
//! librosa (and therefore `rosa`) has no key detector, so this layer builds one
//! the standard way: average the chroma vector over the whole track, then match
//! it against rotated Krumhansl–Kessler tonal profiles with a Pearson
//! correlation per candidate key.
//!
//! This is deliberately simple and throwaway — it exists only to give the rosa
//! spike a comparable `KeyAnalysis` to stratum's.

/// Sharp note spelling, matching `stratum-dsp::Key::name()`.
const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Krumhansl–Kessler (1982) major-key profile, indexed with C = 0.
const MAJOR_PROFILE: [f64; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];

/// Krumhansl–Kessler (1982) minor-key profile, indexed with C = 0.
const MINOR_PROFILE: [f64; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

/// Result of the chroma→key match.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyEstimate {
    /// e.g. `"C"`, `"F#"`, `"Am"`, `"C#m"`.
    pub musical: String,
    /// Heuristic 0..1 margin between the best and runner-up correlation.
    pub confidence: f32,
    /// Heuristic 0..1 mapping of the best correlation onto a positive scale.
    pub clarity: f32,
}

/// Pearson correlation between two equal-length slices.
fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let mean_a = a.iter().sum::<f64>() / n;
    let mean_b = b.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let dx = x - mean_a;
        let dy = y - mean_b;
        cov += dx * dy;
        var_a += dx * dx;
        var_b += dy * dy;
    }
    if var_a <= f64::MIN_POSITIVE || var_b <= f64::MIN_POSITIVE {
        return 0.0;
    }
    cov / (var_a.sqrt() * var_b.sqrt())
}

/// Estimate the key from a 12-bin chroma vector whose index 0 is C.
pub fn estimate_key(chroma: &[f64; 12]) -> KeyEstimate {
    let mut scores: Vec<(f64, usize, bool)> = Vec::with_capacity(24);
    for tonic in 0..12usize {
        for (is_minor, profile) in [(false, &MAJOR_PROFILE), (true, &MINOR_PROFILE)] {
            // Align the profile so index 0 is the candidate tonic.
            let rotated: Vec<f64> = (0..12).map(|pc| profile[(pc + 12 - tonic) % 12]).collect();
            scores.push((pearson(chroma, &rotated), tonic, is_minor));
        }
    }
    scores.sort_by(|a, b| b.0.total_cmp(&a.0));

    let (best, second) = (scores[0], scores[1]);
    let note = NOTE_NAMES[best.1];
    let musical = if best.2 {
        format!("{note}m")
    } else {
        note.to_string()
    };

    KeyEstimate {
        musical,
        confidence: (best.0 - second.0).clamp(0.0, 1.0) as f32,
        clarity: ((best.0 + 1.0) / 2.0).clamp(0.0, 1.0) as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_major_chroma_reads_as_a() {
        // A major triad-ish energy on A, C#, E.
        let mut chroma = [0.1; 12];
        chroma[9] = 6.0; // A
        chroma[1] = 4.0; // C#
        chroma[4] = 4.0; // E
        let est = estimate_key(&chroma);
        assert_eq!(est.musical, "A");
    }

    #[test]
    fn c_minor_chroma_reads_as_cm() {
        let mut chroma = [0.1; 12];
        chroma[0] = 6.0; // C
        chroma[3] = 4.5; // D#
        chroma[7] = 4.0; // G
        let est = estimate_key(&chroma);
        assert_eq!(est.musical, "Cm");
    }
}
