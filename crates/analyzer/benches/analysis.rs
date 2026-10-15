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
//!
//! Timings are reported to the console; no analysis output values are asserted
//! here. Deterministic-output checks live in the crate's `#[cfg(test)]` suites.

use std::path::{Path, PathBuf};
use std::time::Duration;

use analyzer::decode_mono;
use analyzer::{analyze_file, analyze_pcm, integrated_lufs_mono, AnalysisConfig, DecodedMono};
use criterion::{criterion_group, Criterion, Throughput};

/// Real 387.1 s stereo 24-bit 44.1 kHz master (see `samples/README.md`).
const MASTER: &str = "Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav";
/// Compressed master, exercising the Opus decoder path (181.2 s at 48 kHz).
const OPUS: &str = "Palawan by SKIRK  Vlog Music [xXRDR-ycIeo].opus";

/// Native rate of [`MASTER`], asserted against the probe so the PCM scenario's
/// throughput denominator cannot drift away from the fixture.
const MASTER_SR: u32 = 44_100;
/// `AnalysisDurationMode::Precise` on a real track.
const PRECISE_MS: i32 = 193_000;
/// `AnalysisDurationMode::Fast`.
const FAST_MS: i32 = 30_000;
/// In-memory PCM scenario: `AnalysisDurationMode` for a mid-length track.
const PCM_SECS: f64 = 90.0;

/// A fixture inside the repo-root `samples/` tree, or `None` with a printed
/// reason.
///
/// The benches are a local measurement instrument, so a missing fixture is a
/// skip, not a panic in the middle of a run.
fn fixture(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples")
        .join(name);
    if path.is_file() {
        Some(path)
    } else {
        println!("skipping bench: fixture {} not found", path.display());
        None
    }
}

fn config_for(max_duration_ms: Option<i32>) -> AnalysisConfig {
    AnalysisConfig {
        max_duration_ms,
        ..AnalysisConfig::default()
    }
}

fn frames_for(secs: f64) -> u64 {
    (secs * f64::from(MASTER_SR)).round() as u64
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
        let probed = decode_mono(&path, &config)
            .unwrap_or_else(|error| panic!("fixture {} must decode: {error}", path.display()));
        let rate_khz = probed.sample_rate / 1000;
        group.throughput(Throughput::Elements(probed.samples.len() as u64));
        group.bench_function(format!("{label}_{rate_khz}k"), |b| {
            b.iter(|| decode_mono(&path, &config));
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
        let probed = decode_mono(&master, &config)
            .unwrap_or_else(|error| panic!("fixture must decode: {error}"));
        let rate_khz = probed.sample_rate / 1000;
        group.throughput(Throughput::Elements(probed.samples.len() as u64));
        group.bench_function(format!("{label}_{rate_khz}k"), |b| {
            b.iter(|| analyze_file(&master, &config));
        });
    }
    group.finish();
}

/// Decode the mono buffer the in-memory scenarios analyze: `PCM_SECS` seconds
/// of [`MASTER`].
///
/// `analyze_pcm` applies no duration cap of its own — `preprocess::prepare`
/// ignores the config and the cap only bounds the decode — so the whole decoded
/// buffer is the measured work.
fn decode_pcm_fixture() -> Option<DecodedMono> {
    let path = fixture(MASTER)?;
    match decode_mono(&path, &config_for(Some((PCM_SECS * 1000.0) as i32))) {
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
    // of quietly reporting a different amount of work.
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
    group.bench_function("mono_90s_44k", |b| {
        b.iter(|| analyze_pcm(&mono, MASTER_SR, &config));
    });
    group.bench_function("integrated_lufs_mono/mono_90s_44k", |b| {
        b.iter(|| integrated_lufs_mono(&mono, MASTER_SR));
    });
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
