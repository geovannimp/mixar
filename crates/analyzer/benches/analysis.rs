//! Criterion benchmarks for the analyzer facade's decode → preprocess → analyze path.
//!
//! Fixtures are the real audio files in `samples/` at the repo root (see that
//! directory's `README.md` for attribution). They are tracked in git, so a
//! normal checkout has them, and `cargo package` never ships them: the
//! directory sits outside the analyzer crate's package root. A checkout
//! without the tree skips the affected scenario with a message rather than
//! panicking part-way through a measurement run.
//!
//! Runs are **warm**: the file is read through the page cache by the first
//! iteration of every benchmark in this process.
//!
//! Every throughput denominator comes from a probe decode of the fixture (its
//! native sample rate and the frame count actually decoded), never from a
//! hardcoded duration, so a swapped fixture cannot leave the report describing
//! work that was not measured. The `_k` suffix in each id is the probed rate.
//! Probes are memoized per process by [`common::probe`]; they run before the
//! harness times anything, so they are setup work and not part of the measured
//! iterations.
//!
//! Timings are reported to the console; no analysis output values are asserted
//! here. Deterministic-output checks live in the crate's `#[cfg(test)]` suites.

mod common;

use std::time::Duration;

use analyzer::{analyze_file, analyze_pcm, integrated_lufs_mono, DecodedMono};
use common::{
    config_for, fixture, frames_for, probe, FAST_MS, MASTER, MASTER_SR, OPUS, PRECISE_MS,
};
use criterion::{criterion_group, Criterion, Throughput};

/// In-memory PCM scenario: `AnalysisDurationMode` for a mid-length track.
const PCM_SECS: f64 = 90.0;

/// Milliseconds the PCM scenario caps its decode at.
///
/// `PCM_SECS * 1000.0` is exactly representable and far below `i32::MAX`, so the
/// cast is lossless; the assert pins that so drift fails loudly rather than
/// silently truncating the cap.
fn pcm_cap_ms() -> i32 {
    let ms = PCM_SECS * 1000.0;
    assert!(
        ms.is_finite() && ms >= 0.0 && ms <= f64::from(i32::MAX),
        "PCM_SECS ({PCM_SECS}) must convert to a non-negative i32 millisecond cap"
    );
    ms as i32
}

fn criterion_bench() -> Criterion {
    Criterion::default()
        // Analysis of a multi-minute track is seconds per iteration, so the
        // default 3 s warm-up / 5 s measurement windows are unsafe to shrink.
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(6))
        .sample_size(10)
}

fn decode_mono_bench(c: &mut Criterion) {
    let (Some(master), Some(opus)) = (fixture(MASTER), fixture(OPUS)) else {
        return;
    };
    let mut group = c.benchmark_group("decode_mono");
    for (label, path, max_ms) in [
        ("complete/stereo", master.clone(), None),
        ("precise/stereo", master.clone(), Some(PRECISE_MS)),
        ("fast/stereo", master, Some(FAST_MS)),
        ("complete/opus", opus, None),
    ] {
        let config = config_for(max_ms);
        // Probe with the scenario's own config: the decoded frame count is the
        // work one iteration performs, and the fixture's rate names the id.
        let Some(probed) = probe(&path, &config) else {
            continue;
        };
        let rate_khz = f64::from(probed.sample_rate) / 1000.0;
        group.throughput(Throughput::Elements(probed.frames));
        group.bench_function(format!("{label}_{rate_khz:.1}k"), |b| {
            b.iter(|| analyzer::decode_mono(&path, &config));
        });
    }
    group.finish();
}

fn analyze_file_bench(c: &mut Criterion) {
    let Some(master) = fixture(MASTER) else {
        return;
    };
    let mut group = c.benchmark_group("analyze_file");
    for (label, max_ms) in [
        ("complete/stereo", None),
        ("precise/stereo", Some(PRECISE_MS)),
        ("fast/stereo", Some(FAST_MS)),
    ] {
        let config = config_for(max_ms);
        // The probe's frame count is the decode half of the measured work, so it
        // is the honest denominator for the combined analysis run.
        let Some(probed) = probe(&master, &config) else {
            continue;
        };
        let rate_khz = f64::from(probed.sample_rate) / 1000.0;
        group.throughput(Throughput::Elements(probed.frames));
        group.bench_function(format!("{label}_{rate_khz:.1}k"), |b| {
            b.iter(|| analyze_file(&master, &config));
        });
    }
    group.finish();
}

/// Decode the mono buffer the in-memory scenarios analyze: `PCM_SECS` seconds
/// of [`MASTER`], or `None` when the scenario cannot run.
///
/// `None` covers both skip cases: the fixture is missing from the repo-root
/// `samples/` tree, or it exists and will not decode. Either way nothing is
/// measured, so the scenario is skipped with a printed reason rather than
/// failing the run.
///
/// `analyze_pcm` applies no duration cap of its own — `preprocess::prepare`
/// ignores the config and the cap only bounds the decode — so the whole decoded
/// buffer is the measured work.
fn decode_pcm_fixture() -> Option<DecodedMono> {
    let path = fixture(MASTER)?;
    match analyzer::decode_mono(&path, &config_for(Some(pcm_cap_ms()))) {
        Ok(decoded) => Some(decoded),
        Err(error) => {
            println!(
                "skipping bench: {} could not be decoded: {error}",
                path.display()
            );
            None
        }
    }
}

fn analyze_pcm_bench(c: &mut Criterion) {
    let Some(decoded) = decode_pcm_fixture() else {
        return;
    };
    // The id and the throughput denominator both claim `PCM_SECS` seconds at
    // `MASTER_SR`; pin the fixture to that claim so a swap fails loudly instead
    // of quietly reporting a different amount of work. Unlike a missing fixture
    // (a skip, because nothing was measured), a fixture that decodes to the wrong
    // rate or length would make the report describe work that did not happen, so
    // that case is a panic.
    assert_eq!(
        decoded.sample_rate, MASTER_SR,
        "the PCM scenario's throughput is derived from MASTER_SR"
    );
    assert_eq!(
        decoded.samples.len(),
        frames_for(PCM_SECS) as usize,
        "the master must be at least {PCM_SECS} s long"
    );
    let mono = decoded.samples;
    let config = config_for(None);

    let mut group = c.benchmark_group("analyze_pcm");
    group.throughput(Throughput::Elements(mono.len() as u64));
    let rate_khz = f64::from(MASTER_SR) / 1000.0;
    group.bench_function(format!("mono_90s_{rate_khz:.1}k"), |b| {
        b.iter(|| analyze_pcm(&mono, MASTER_SR, &config));
    });
    group.bench_function(
        format!("integrated_lufs_mono/mono_90s_{rate_khz:.1}k"),
        |b| {
            b.iter(|| integrated_lufs_mono(&mono, MASTER_SR));
        },
    );
    group.finish();
}

criterion_group! {
    name = benches;
    config = criterion_bench();
    targets = decode_mono_bench, analyze_file_bench, analyze_pcm_bench
}

fn main() {
    benches();
}
