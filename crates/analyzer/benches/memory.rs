//! Peak-RSS scenarios for the analyzer facade.
//!
//! Criterion reports time only, but this run also has to flag peak-memory
//! regressions. Each scenario below decodes/analyzes the same real audio a fixed
//! number of times and prints `VmHWM` (peak resident set size) before and after,
//! so a change can be compared by the high-water mark it leaves behind rather
//! than by an aggregate statistic.
//!
//! Run with: `cargo bench -p analyzer --bench memory` (the report goes to
//! stdout; no criterion arguments are parsed here).
//!
//! Page cache is warm, matching the timing benchmarks in `analysis.rs`.

mod common;

use analyzer::{analyze_file, decode_mono};

/// Repetitions per scenario; the peak is a high-water mark, so repeats only
/// confirm it is reached by the first pass.
const REPEATS: usize = 3;

/// Peak resident set size in kibibytes, read from `VmHWM`.
#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn scenario(name: &str, mut run: impl FnMut() -> anyhow::Result<()>) {
    // A fixture that exists but will not decode is a skip, not an abort: the
    // scenarios already measured stay in the report.
    if let Err(error) = run() {
        println!("skipping bench: {name} could not run: {error}");
        return;
    }
    let before = peak_rss_kib();
    for _ in 0..REPEATS {
        if let Err(error) = run() {
            println!("skipping bench: {name} could not run: {error}");
            return;
        }
    }
    let after = peak_rss_kib();
    println!(
        "{name:<44} VmHWM before {before:>9} KiB   after {after:>9} KiB   delta {:>9} KiB",
        after as i64 - before as i64
    );
}

#[cfg(target_os = "linux")]
fn run() {
    let Some(master) = common::fixture(common::MASTER) else {
        return;
    };

    scenario("decode_mono/complete/stereo_44k", || {
        decode_mono(&master, &common::config_for(None))?;
        Ok(())
    });
    scenario("decode_mono/precise/stereo_44k", || {
        decode_mono(&master, &common::config_for(Some(common::PRECISE_MS)))?;
        Ok(())
    });
    scenario("decode_mono/fast/stereo_44k", || {
        decode_mono(&master, &common::config_for(Some(common::FAST_MS)))?;
        Ok(())
    });
    scenario("analyze_file/complete/stereo_44k", || {
        analyze_file(&master, &common::config_for(None))?;
        Ok(())
    });
    scenario("analyze_file/precise/stereo_44k", || {
        analyze_file(&master, &common::config_for(Some(common::PRECISE_MS)))?;
        Ok(())
    });
    scenario("analyze_file/fast/stereo_44k", || {
        analyze_file(&master, &common::config_for(Some(common::FAST_MS)))?;
        Ok(())
    });
}

fn main() {
    // Peak-RSS reporting reads `/proc/self/status`, which only exists on Linux;
    // everywhere else the run is a skip rather than a panic.
    #[cfg(not(target_os = "linux"))]
    println!("skipping bench: peak-RSS reporting reads /proc/self/status and needs Linux");
    #[cfg(target_os = "linux")]
    run();
}
