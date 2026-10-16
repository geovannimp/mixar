//! Timing harness for the qm-dsp analysis port.
//!
//! Hand-rolled on purpose: Criterion is a workspace dependency but absent from
//! `Cargo.lock`, and pulling it in would touch the lockfile and the
//! dev-dependency set of a crate that is otherwise dependency-free. The row set
//! is exactly the stage boundaries needed to attribute a change.
//!
//! Usage:
//!   cargo run --release -p analyzer-qmdsp --example qmdsp_timing -- [--secs N]
//!       [--pipeline-only] [--dump PATH]
//!
//! One stdout line per row:
//!   row <name> min_ms=%.3f mean_ms=%.3f reps=%d peak_vmhwm_kb=%d
//!
//! The sub-stage rows reach the crate's pipeline modules directly
//! (`analyzer_qmdsp::key::…`) and take their framing from
//! `analyzer_qmdsp::measurement`, all behind the default-on `sub` feature.
//! Building with `--no-default-features` compiles the example down to the four
//! `analyze_*` rows only, which is what lets it run against sources that
//! predate that surface.

use std::hint::black_box;
use std::time::Instant;

use analyzer_core::{AnalysisConfig, AnalysisTargets, AudioAnalyzer};
use analyzer_qmdsp::QmdspAnalyzer;

const SR: u32 = 44_100;
const WARMUP: usize = 1;
const REPS: usize = 5;

fn corpus(secs: f64) -> Vec<f32> {
    let n = (f64::from(SR) * secs) as usize;
    let beat = 60.0 / 128.0;
    let mut y = vec![0.0f32; n];
    for (i, slot) in y.iter_mut().enumerate() {
        let t = i as f64 / f64::from(SR);
        let env = (-18.0 * (t / beat).fract()).exp();
        let kick = (2.0 * std::f64::consts::PI * 62.0 * t).sin() * env * 0.6;
        let hat = (2.0 * std::f64::consts::PI * 3000.0 * t).sin()
            * (-60.0 * ((t % 0.1) / 0.1)).exp()
            * 0.3;
        let chord = 0.25 * (2.0 * std::f64::consts::PI * 261.63 * t).sin()
            + 0.20 * (2.0 * std::f64::consts::PI * 329.63 * t).sin()
            + 0.15 * (2.0 * std::f64::consts::PI * 392.0 * t).sin();
        let lush = 0.5 + 0.5 * (2.0 * std::f64::consts::PI * t / 8.0).sin();
        *slot = (kick + hat + chord * lush).clamp(-1.0, 1.0) as f32;
    }
    y
}

fn vmhwm_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    line.split_whitespace().nth(1)?.parse::<u64>().ok()
}

fn bench<T>(name: &str, warmup: usize, reps: usize, mut f: impl FnMut() -> T) -> T {
    for _ in 0..warmup {
        black_box(f());
    }
    let mut times = Vec::with_capacity(reps);
    let mut last = None;
    for _ in 0..reps {
        let t0 = Instant::now();
        last = Some(f());
        times.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    let min = times.iter().copied().fold(f64::INFINITY, f64::min);
    let mean = times.iter().sum::<f64>() / reps as f64;
    let peak = vmhwm_kb().unwrap_or(0);
    println!("row {name} min_ms={min:.3} mean_ms={mean:.3} reps={reps} peak_vmhwm_kb={peak}");
    last.expect("at least one rep")
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn fingerprint(beats: &[f32], bpm: Option<f64>, key: Option<&str>, downbeats: &[f32]) {
    let mut bytes = Vec::new();
    for &b in beats {
        bytes.extend_from_slice(&b.to_le_bytes());
    }
    for &d in downbeats {
        bytes.extend_from_slice(&d.to_le_bytes());
    }
    let hash = fnv1a64(&bytes);
    let first = downbeats.first().copied().unwrap_or(0.0);
    let last = downbeats.last().copied().unwrap_or(0.0);
    let sum: f64 = downbeats.iter().map(|&d| f64::from(d)).sum();
    let mean = if downbeats.is_empty() {
        0.0
    } else {
        sum / downbeats.len() as f64
    };
    println!(
        "fingerprint beats={} bpm={} key={} downbeats={} first={:.6} last={:.6} mean={:.6} hash={hash:016x}",
        beats.len(),
        bpm.map(|b| format!("{b:.6}")).unwrap_or_else(|| "none".to_string()),
        key.unwrap_or("none"),
        downbeats.len(),
        first,
        last,
        mean,
    );
}

fn targets(bpm: bool, key: bool, beat_grid: bool) -> AnalysisConfig {
    AnalysisConfig {
        targets: AnalysisTargets {
            bpm,
            key,
            beat_grid,
        },
        ..AnalysisConfig::default()
    }
}

fn main() {
    let mut secs = 240.0f64;
    let mut pipeline_only = false;
    let mut dump: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--secs" => {
                if let Some(v) = args.next() {
                    secs = v.parse().expect("--secs takes a number");
                }
            }
            "--dump" => dump = args.next(),
            "--pipeline-only" => pipeline_only = true,
            other => panic!("unknown argument {other}"),
        }
    }

    let y = corpus(secs);
    let analyzer = QmdspAnalyzer::new();
    let all = targets(true, true, true);

    let full = analyzer.analyze_pcm(&y, SR, &all).expect("analyze_full");
    let grid = full.beat_grid.as_ref().expect("beat grid");
    let full_beats = grid.beats.clone();
    let full_downbeats = grid.downbeats.clone();
    fingerprint(
        &full_beats,
        full.bpm.map(|b| b.bpm),
        full.key.as_ref().map(|k| k.musical.as_str()),
        &full_downbeats,
    );
    if let Some(path) = &dump {
        // Streamed: one line per beat, no whole-file intermediate String.
        use std::io::{BufWriter, Write};
        let mut out = BufWriter::new(std::fs::File::create(path).expect("create dump"));
        for &b in &full_beats {
            writeln!(out, "{b:e}").expect("write dump");
        }
        out.flush().expect("flush dump");
    }

    let _ = bench("analyze_full", WARMUP, REPS, || {
        analyzer.analyze_pcm(&y, SR, &all)
    });

    let bpm_only = targets(true, false, false);
    let _ = bench("analyze_bpm", WARMUP, REPS, || {
        analyzer.analyze_pcm(&y, SR, &bpm_only)
    });

    let grid_only = targets(false, false, true);
    let _ = bench("analyze_grid", WARMUP, REPS, || {
        analyzer.analyze_pcm(&y, SR, &grid_only)
    });

    let key_only = targets(false, true, false);
    let _ = bench("analyze_key", WARMUP, REPS, || {
        analyzer.analyze_pcm(&y, SR, &key_only)
    });

    let key_run = analyzer
        .analyze_pcm(&y, SR, &key_only)
        .expect("analyze_key");
    fingerprint(
        &[],
        None,
        key_run.key.as_ref().map(|k| k.musical.as_str()),
        &[],
    );

    if pipeline_only {
        return;
    }

    #[cfg(feature = "sub")]
    sub_rows(&y, &full_beats);
    #[cfg(not(feature = "sub"))]
    eprintln!(
        "note: built without the `sub` feature, so the sub-stage rows are not \
         measured; re-run with `--features sub` for all 11 rows"
    );
}

#[cfg(feature = "sub")]
fn sub_rows(y: &[f32], full_beats: &[f32]) {
    use analyzer_qmdsp::{downbeat, key, math, measurement, onset, overlap, tempo};

    bench("sub_init", WARMUP, REPS, || key::GetKeyMode::new(SR));

    let y64: Vec<f64> = y.iter().map(|&s| f64::from(s)).collect();
    // Framing straight from the library, so these rows cannot drift from
    // `detect_beats`.
    let (window, step) = measurement::frame_and_step(SR);
    println!("framing step={step} window={window}");
    let results = bench("sub_onset", WARMUP, REPS, || {
        let mut df = onset::DetectionFunction::new(window);
        let mut res = Vec::new();
        overlap::feed(&y64, window, step, |f| {
            res.push(df.process_time_domain(f));
        });
        res
    });
    // Same trim `detect_beats` applies, on the very sweep `sub_onset` just ran,
    // so the tempo rows below see the same detection function the beats path
    // feeds them.
    let df_slice = measurement::beat_tracking_slice(&results);

    let bp = bench("sub_beat_period", WARMUP, REPS, || {
        tempo::calculate_beat_period(df_slice)
    });

    bench("sub_beats_dp", WARMUP, REPS, || {
        tempo::calculate_beats(df_slice, &bp)
    });

    // The decimated stream, shaped exactly as `analyze_pcm` builds it for the
    // key and downbeat consumers: the ×8 pass sweeps the track plus the silence
    // `overlap::feed` would append, and the result carries a zero pre-roll in
    // front. `sub_chroma`, `sub_decimate8` and `sub_downbeat` all read this, so
    // they time the same input production hands those stages.
    let mode = key::GetKeyMode::new(SR);
    let block = mode.block_size();
    let frame_size = mode.frame_size();
    let preroll = frame_size / 2;
    let pad = mode.decimation_input_len(y64.len()) - block / 2 - y64.len();
    let mut buf = y64.clone();
    buf.resize(y64.len() + pad, 0.0);
    let mut dec8 = vec![0.0; preroll + buf.len() / 8];
    let mut d8 = key::decimator::Decimator::new(key::decimator::Decimation::By8, buf.len());
    d8.process(&buf, &mut dec8[preroll..]);

    bench("sub_decimate8", WARMUP, REPS, || {
        let mut out = vec![0.0; preroll + buf.len() / 8];
        let mut d = key::decimator::Decimator::new(key::decimator::Decimation::By8, buf.len());
        d.process(&buf, &mut out[preroll..]);
        out
    });

    let mut chroma = key::cqt::Chromagram::new(key::cqt::ChromagramConfig {
        fs: f64::from(SR) / 8.0,
        min: math::pitch_frequency(48, -12.0 / 36.0 * 100.0, 440.0),
        max: math::pitch_frequency(96, -12.0 / 36.0 * 100.0, 440.0),
        bpo: 36,
        cq_thresh: 0.0054,
    });
    // Fold each frame into a checksum instead of collecting owned copies:
    // `Chromagram::process` returns a borrow of its own buffer, so the chroma
    // math is what is timed, not a Vec per frame. The checksum is a sink that
    // stops the loop being optimised away — it is deliberately *not* a stable
    // fingerprint, because `chroma`'s internal state carries across reps (a warm
    // cache is what a timing row wants) and each rep therefore folds a different
    // sequence. Nothing parses this value.
    let sub_chroma_frame = frame_size;
    bench("sub_chroma", WARMUP, REPS, || {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for frame in dec8.chunks_exact(sub_chroma_frame) {
            for &v in chroma.process(frame) {
                hash ^= v.to_bits();
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        hash
    });

    // The downbeat sees the same slice `analyze_pcm` hands it.
    let downbeat_dec = &dec8[preroll..preroll + y64.len() / 8];
    bench("sub_downbeat", WARMUP, REPS, || {
        downbeat::detect_downbeats(downbeat_dec, SR, full_beats, 4)
    });
}
