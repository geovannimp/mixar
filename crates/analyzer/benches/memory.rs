//! Peak-RSS scenarios for the analyzer facade.
//!
//! Criterion reports time only, but this run also has to flag peak-memory
//! regressions. Each scenario below decodes/analyzes the same real audio a fixed
//! number of times and prints `VmHWM` (peak resident set size) before and after,
//! so a change can be compared by the high-water mark it leaves behind rather
//! than by an aggregate statistic.
//!
//! Run with: `cargo bench -p analyzer --bench memory -- --nocapture` (the report
//! goes to stdout; the `--nocapture` flag keeps `cargo` from swallowing it).
//!
//! Page cache is warm, matching the timing benchmarks in `analysis.rs`.

use std::path::{Path, PathBuf};

use analyzer::decode_mono;
use analyzer::{analyze_file, AnalysisConfig};

/// Real 386.7 s stereo 44.1 kHz master (see `samples/README.md`).
const MASTER: &str = "Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav";
/// `AnalysisDurationMode::Precise` on a real track.
const PRECISE_MS: i32 = 193_000;
/// `AnalysisDurationMode::Fast`.
const FAST_MS: i32 = 30_000;
/// Repetitions per scenario; the peak is a high-water mark, so repeats only
/// confirm it is reached by the first pass.
const REPEATS: usize = 3;

fn sample(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples")
        .join(name)
}

fn config_for(max_duration_ms: Option<i32>) -> AnalysisConfig {
    AnalysisConfig {
        max_duration_ms,
        ..AnalysisConfig::default()
    }
}

/// Peak resident set size in kibibytes, read from `VmHWM`.
fn peak_rss_kib() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status")
        .expect("this benchmark only supports Linux, which always has /proc/self/status");
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            return rest
                .split_whitespace()
                .next()
                .and_then(|value| value.parse().ok())
                .expect("VmHWM is `<value> kB`");
        }
    }
    panic!("VmHWM missing from /proc/self/status");
}

fn scenario(name: &str, mut run: impl FnMut()) {
    // Warm the code paths and the page cache so the reported peak is the steady
    // state of the operation, not of process start-up.
    run();
    let before = peak_rss_kib();
    for _ in 0..REPEATS {
        run();
    }
    let after = peak_rss_kib();
    println!(
        "{name:<44} VmHWM before {before:>9} KiB   after {after:>9} KiB   delta {:>9} KiB",
        after as i64 - before as i64
    );
}

fn main() {
    let master = sample(MASTER);

    scenario("decode_mono/complete/stereo_44k", || {
        decode_mono(&master, &config_for(None)).unwrap();
    });
    scenario("decode_mono/precise/stereo_44k", || {
        decode_mono(&master, &config_for(Some(PRECISE_MS))).unwrap();
    });
    scenario("decode_mono/fast/stereo_44k", || {
        decode_mono(&master, &config_for(Some(FAST_MS))).unwrap();
    });
    scenario("analyze_file/complete/stereo_44k", || {
        analyze_file(&master, &config_for(None)).unwrap();
    });
    scenario("analyze_file/precise/stereo_44k", || {
        analyze_file(&master, &config_for(Some(PRECISE_MS))).unwrap();
    });
    scenario("analyze_file/fast/stereo_44k", || {
        analyze_file(&master, &config_for(Some(FAST_MS))).unwrap();
    });
}
