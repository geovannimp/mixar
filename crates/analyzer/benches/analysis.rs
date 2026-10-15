//! Criterion benchmarks for the analyzer facade's decode → preprocess → analyze path.
//!
//! Fixtures are the real audio files in `samples/` (see that directory's
//! `README.md` for attribution). Runs are **warm**: the file is read through the
//! page cache by the first iteration of every benchmark in this process.
//!
//! Timings are reported to the console; no analysis output values are asserted
//! here. Deterministic-output checks live in the crate's `#[cfg(test)]` suites.

use std::path::{Path, PathBuf};
use std::time::Duration;

use analyzer::decode_mono;
use analyzer::{analyze_file, analyze_pcm, integrated_lufs_mono, AnalysisConfig};
use criterion::{criterion_group, BenchmarkId, Criterion, Throughput};

/// Real 386.7 s stereo 44.1 kHz master (see `samples/README.md`).
const MASTER: &str = "Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav";
/// Compressed master, exercising the Opus decoder path.
const OPUS: &str = "Palawan by SKIRK  Vlog Music [xXRDR-ycIeo].opus";

const MASTER_SR: u32 = 44_100;
const MASTER_SECS: f64 = 386.7;
/// Half the master's length, i.e. `AnalysisDurationMode::Precise` on a real track.
const PRECISE_MS: i32 = 193_000;
/// `AnalysisDurationMode::Fast`.
const FAST_MS: i32 = 30_000;
/// In-memory PCM scenario: `AnalysisDurationMode` for a mid-length track.
const PCM_SECS: f64 = 90.0;

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
    let master = sample(MASTER);
    let mut group = c.benchmark_group("decode_mono");
    for (label, secs, max_ms) in [
        ("complete/stereo_44k", MASTER_SECS, None),
        ("precise/stereo_44k", 193.0, Some(PRECISE_MS)),
        ("fast/stereo_44k", 30.0, Some(FAST_MS)),
        ("complete/opus_48k", 120.0, None),
    ] {
        let path = if label.contains("opus") {
            sample(OPUS)
        } else {
            master.clone()
        };
        let config = config_for(max_ms);
        group.throughput(Throughput::Elements(frames_for(secs)));
        group.bench_function(BenchmarkId::new("decode_mono", label), |b| {
            b.iter(|| decode_mono(&path, &config));
        });
    }
    group.finish();
}

fn analyze_file_bench(c: &mut Criterion) {
    let master = sample(MASTER);
    let mut group = c.benchmark_group("analyze_file");
    for (label, secs, max_ms) in [
        ("complete/stereo_44k", MASTER_SECS, None),
        ("precise/stereo_44k", 193.0, Some(PRECISE_MS)),
        ("fast/stereo_44k", 30.0, Some(FAST_MS)),
    ] {
        let config = config_for(max_ms);
        group.throughput(Throughput::Elements(frames_for(secs)));
        group.bench_function(BenchmarkId::new("analyze_file", label), |b| {
            b.iter(|| analyze_file(&master, &config));
        });
    }
    group.finish();
}

fn analyze_pcm_bench(c: &mut Criterion) {
    let config = config_for(None);
    let decoded = decode_mono(
        &sample(MASTER),
        &config_for(Some((PCM_SECS * 1000.0) as i32)),
    )
    .unwrap();
    let mono = decoded.samples;

    let mut group = c.benchmark_group("analyze_pcm");
    group.throughput(Throughput::Elements(frames_for(PCM_SECS)));
    group.bench_function("analyze_pcm/mono_90s_44k", |b| {
        b.iter(|| analyze_pcm(&mono, decoded.sample_rate, &config));
    });
    group.bench_function("integrated_lufs_mono/mono_90s_44k", |b| {
        b.iter(|| integrated_lufs_mono(&mono, decoded.sample_rate));
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
