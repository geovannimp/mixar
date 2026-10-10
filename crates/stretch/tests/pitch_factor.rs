//! Realtime pitch factor: semitone math, octave-up/down frequency checks.

use stretch::{semitones_to_pitch, TimeStretcher, TimestretchStretcher};

const SR: u32 = 48_000;

#[test]
fn semitone_math_is_exact() {
    assert!((semitones_to_pitch(0.0) - 1.0).abs() < 1e-12);
    assert!((semitones_to_pitch(12.0) - 2.0).abs() < 1e-12);
    assert!((semitones_to_pitch(-12.0) - 0.5).abs() < 1e-12);
    assert!((semitones_to_pitch(1.0) - 1.059_463_094_359_295_4).abs() < 1e-9);
}

fn produced_freq(semitones: f32) -> f64 {
    let mut s = TimestretchStretcher::new(SR, 512).unwrap();
    s.set_tempo_rate(1.0);
    s.set_pitch_factor(semitones_to_pitch(semitones));
    let total = SR as usize * 2;
    let mut buf = vec![0.0f32; total * 2];
    let mut phase = 0.0f64;
    let step = 440.0 / SR as f64;
    let n = s
        .pull_interleaved(total, &mut buf, &mut |need, out| {
            for i in 0..need {
                let v = (phase * std::f64::consts::TAU).sin() as f32 * 0.5;
                out[i * 2] = v;
                out[i * 2 + 1] = v;
                phase += step;
            }
            need
        })
        .out_frames;
    let tail = &buf[(n - SR as usize / 4) * 2..n * 2];
    // Zero-crossing frequency estimate on the left channel.
    let mut crossings = 0usize;
    for w in tail.as_chunks::<2>().0.windows(2) {
        if w[0][0] <= 0.0 && w[1][0] > 0.0 {
            crossings += 1;
        }
    }
    crossings as f64 * (SR as f64) / (SR as f64 / 4.0)
}

#[test]
fn pitch_up_octave_doubles_frequency() {
    let f = produced_freq(12.0);
    assert!((f - 880.0).abs() / 880.0 < 0.03, "got {f}");
}

#[test]
fn pitch_down_octave_halves_frequency() {
    let f = produced_freq(-12.0);
    assert!((f - 220.0).abs() / 220.0 < 0.03, "got {f}");
}

/// Pull `total` frames of a deterministic 440 Hz sine through `s`.
fn pull_sine(s: &mut TimestretchStretcher, total: usize) -> Vec<f32> {
    let mut buf = vec![0.0f32; total * 2];
    let mut phase = 0.0f64;
    let step = 440.0 / SR as f64;
    s.pull_interleaved(total, &mut buf, &mut |need, out| {
        for i in 0..need {
            let v = (phase * std::f64::consts::TAU).sin() as f32 * 0.5;
            out[i * 2] = v;
            out[i * 2 + 1] = v;
            phase += step;
        }
        need
    });
    buf
}

/// `set_pitch_factor(1.0)` must stay on the resampler-bypass path, so its output
/// is byte-identical to a stretcher that never called it. (The deck-level
/// `key_shift_zero_matches_unset_output` only compares two interpolated decks.)
#[test]
fn unity_pitch_bypass_is_bit_identical() {
    let mut plain = TimestretchStretcher::new(SR, 512).unwrap();
    let mut explicit = TimestretchStretcher::new(SR, 512).unwrap();
    plain.set_tempo_rate(1.0);
    explicit.set_tempo_rate(1.0);
    explicit.set_pitch_factor(1.0);

    let plain_out = pull_sine(&mut plain, SR as usize);
    let explicit_out = pull_sine(&mut explicit, SR as usize);
    assert_eq!(
        plain_out, explicit_out,
        "explicit unity pitch must not engage the pitch resampler"
    );
}
