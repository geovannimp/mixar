//! Coarse check: time-stretch keeps pitch near the source sine frequency.

use stretch::create_stretcher;

fn estimate_period_samples(samples: &[f32], channels: usize) -> Option<f64> {
    if samples.len() < channels * 64 {
        return None;
    }
    let mut crossings = Vec::new();
    let mut prev = samples[0];
    for (i, frame) in samples.chunks_exact(channels).enumerate().skip(1) {
        let s = frame[0];
        if prev < 0.0 && s >= 0.0 {
            crossings.push(i as f64);
        }
        prev = s;
    }
    if crossings.len() < 4 {
        return None;
    }
    let mut gaps = Vec::new();
    for w in crossings.windows(2) {
        gaps.push(w[1] - w[0]);
    }
    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(gaps[gaps.len() / 2])
}

#[test]
fn time_stretch_preserves_sine_pitch() {
    let sr = 48_000u32;
    let freq = 440.0_f64;
    let mut stretcher = create_stretcher(sr, 1024).expect("stretcher");
    stretcher.set_time_ratio(1.12); // +12% tempo, key lock

    let total_out = sr as usize; // 1 s
    let mut output = vec![0.0_f32; total_out * 2];
    let mut src_pos = 0.0_f64;
    let two_pi = std::f64::consts::TAU;

    let mut offset = 0usize;
    while offset < total_out {
        let chunk = (total_out - offset).min(512);
        let stats =
            stretcher.pull_interleaved(chunk, &mut output[offset * 2..], &mut |need, buf| {
                for i in 0..need {
                    let t = src_pos / f64::from(sr);
                    let s = (two_pi * freq * t).sin() as f32 * 0.5;
                    buf[i * 2] = s;
                    buf[i * 2 + 1] = s;
                    src_pos += 1.0;
                }
                need
            });
        assert!(
            stats.out_frames > 0 || offset > 0,
            "stretcher produced silence"
        );
        offset += chunk;
    }

    // Skip start latency / pad region.
    let skip = stretcher
        .start_delay()
        .saturating_add(stretcher.preferred_start_pad())
        + sr as usize / 10;
    let period = estimate_period_samples(&output[skip * 2..], 2).expect("zero crossings");
    let expected = f64::from(sr) / freq;
    let err = (period - expected).abs() / expected;
    assert!(
        err < 0.05,
        "period={period:.3} expected={expected:.3} rel_err={err:.3}"
    );
}
