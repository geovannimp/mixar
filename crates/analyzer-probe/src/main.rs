//! Multi-backend analysis probe (diagnostic spike, not product output).
//!
//! Runs every available backend on the same decoded PCM and prints the results:
//! `stratum`, `rosa`, `beatthis` (Beat This! ONNX, loaded only when its models
//! are present), and `qmdsp` (pure-Rust port of Mixxx's qm-dsp). `qmdsp-ffi`
//! (the C++ qm-dsp oracle) is added too when its source is available, so the
//! port can be differential-tested.
//!
//! ```text
//! # positional args (quote paths with spaces); directories are expanded
//! cargo run -p analyzer-probe --manifest-path crates/Cargo.toml -- "/path/to/tracks"
//!
//! # env var, ':' or ';' separated
//! ANALYZER_PROBE_TRACK="/path/a.wav:/path/b.mp3" \
//!   cargo run -p analyzer-probe --manifest-path crates/Cargo.toml
//!
//! # no track given -> falls back to the sample below
//! cargo run -p analyzer-probe --manifest-path crates/Cargo.toml
//! ```
//!
//! Environment:
//! - `ANALYZER_PROBE_MAX_MS` — milliseconds to analyze (`0`/`full` = whole track; default 90000).
//! - `ANALYZER_PROBE_BACKENDS` — comma-separated allow-list (e.g. `qmdsp,qmdsp-ffi`).
//! - `ANALYZER_PROBE_NO_SNAP=1` — disable the facade grid snap (see raw backend beats).
//! - `ANALYZER_PROBE_CSV=1` — machine-readable rows per backend.
//! - `ANALYZER_PROBE_DUMP_DIR=<dir>` — write `<track>.<backend>.beats` beat-time dumps.
//! - `BEATTHIS_MEL_MODEL`, `BEATTHIS_BEAT_MODEL`, `BEATTHIS_MODEL_DIR` — Beat This! models.
//! - `QMDSP_SOURCE_DIR` — qm-dsp checkout for the `qmdsp-ffi` oracle.

use std::path::{Path, PathBuf};
use std::time::Instant;

use analyzer::{analyze_file_with, AnalysisConfig, AudioAnalyzer, TrackAnalysis};
use analyzer_beatthis::BeatThisAnalyzer;
use analyzer_qmdsp::QmdspAnalyzer;
use analyzer_qmdsp_ffi::QmdspFfiAnalyzer;
use analyzer_rosa::RosaAnalyzer;
use analyzer_stratum::StratumAnalyzer;
use anyhow::Result;

/// Fallback when no track is supplied. Change freely.
const DEFAULT_TRACK: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../samples/Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav"
);
const DEFAULT_MAX_MS: i32 = 90_000;

struct Backend {
    label: &'static str,
    analyzer: Box<dyn AudioAnalyzer>,
}

/// Every backend available in this build. Beat This! is skipped if its models
/// are missing so the probe still runs stratum/rosa out of the box.
fn backends() -> Vec<Backend> {
    let mut list: Vec<Backend> = vec![
        Backend {
            label: "stratum",
            analyzer: Box::new(StratumAnalyzer::new()),
        },
        Backend {
            label: "rosa",
            analyzer: Box::new(RosaAnalyzer::new()),
        },
    ];
    if let Some(beatthis) = BeatThisAnalyzer::from_env() {
        list.push(Backend {
            label: "beatthis",
            analyzer: Box::new(beatthis),
        });
    }
    list.push(Backend {
        label: "qmdsp",
        analyzer: Box::new(QmdspAnalyzer::new()),
    });
    if let Some(qmdsp_ffi) = QmdspFfiAnalyzer::from_env() {
        list.push(Backend {
            label: "qmdsp-ffi",
            analyzer: Box::new(qmdsp_ffi),
        });
    }
    // Optional restriction, e.g. ANALYZER_PROBE_BACKENDS=rosa for a fast key-only run.
    if let Ok(filter) = std::env::var("ANALYZER_PROBE_BACKENDS") {
        let wanted: Vec<&str> = filter
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if !wanted.is_empty() {
            list.retain(|backend| wanted.contains(&backend.label));
        }
    }
    list
}

fn main() -> Result<()> {
    let tracks = resolve_tracks();
    if tracks.is_empty() {
        anyhow::bail!("no tracks to analyze");
    }
    let max_ms = resolve_max_ms();
    let mut config = AnalysisConfig {
        max_duration_ms: max_ms,
        ..Default::default()
    };
    if std::env::var("ANALYZER_PROBE_NO_SNAP").is_ok() {
        config.snap.enabled = false;
    }
    let csv = std::env::var("ANALYZER_PROBE_CSV").is_ok();
    let backends = backends();

    if csv {
        println!(
            "track,backend,bpm,bpm_conf,key,key_conf,key_clarity,first_beat,last_beat,beat_count,duration_ms,elapsed_ms"
        );
    } else {
        println!(
            "analysis probe ({} backends) — not product output",
            backends.len()
        );
        println!(
            "max_duration_ms={}\n",
            max_ms.map_or("full".to_string(), |ms| ms.to_string())
        );
    }

    for track in tracks {
        if !csv {
            println!("=== {} ===", track.display());
        }
        if !track.is_file() {
            eprintln!("skip (not a file): {}", track.display());
            continue;
        }

        let mut results: Vec<(&'static str, TrackAnalysis, u128)> = Vec::new();
        for backend in &backends {
            match run(backend.label, &track, &config, &backend.analyzer) {
                Ok((analysis, elapsed)) => results.push((backend.label, analysis, elapsed)),
                Err(err) => eprintln!("{} failed: {err:#}", backend.label),
            }
        }

        if csv {
            for (label, analysis, elapsed) in &results {
                print_csv_row(&track, label, analysis, *elapsed);
            }
        } else if !results.is_empty() {
            print_header();
            for (label, analysis, elapsed) in &results {
                print_row(label, analysis, *elapsed);
            }
            for (label, analysis, _) in &results {
                print_grid(label, analysis);
            }
        }

        for (label, analysis, _) in &results {
            maybe_dump(&track, label, analysis);
        }
        if !csv && !results.is_empty() {
            println!();
        }
    }
    Ok(())
}

/// Decode + analyze one track with one backend, returning the analysis and the
/// wall-clock time it took.
fn run<A: AudioAnalyzer>(
    label: &str,
    path: &Path,
    config: &AnalysisConfig,
    analyzer: &A,
) -> Result<(TrackAnalysis, u128)> {
    let started = Instant::now();
    let analysis = analyze_file_with(analyzer, path, config)
        .map_err(|err| anyhow::anyhow!("{label}: {err}"))?;
    Ok((analysis, started.elapsed().as_millis()))
}

/// One CSV row per backend so the output can be merged with file tags.
fn print_csv_row(track: &Path, backend: &str, analysis: &TrackAnalysis, elapsed_ms: u128) {
    let fmt = |value: Option<f64>| value.map(|v| format!("{v:.4}")).unwrap_or_default();
    let (first, last, count) = match &analysis.beat_grid {
        Some(grid) if !grid.beats.is_empty() => (
            Some(f64::from(grid.beats[0])),
            grid.beats.last().map(|b| f64::from(*b)),
            Some(grid.beats.len()),
        ),
        _ => (None, None, None),
    };
    println!(
        "{},{},{},{},{},{},{},{},{},{},{},{}",
        csv_escape(&track.display().to_string()),
        backend,
        fmt(analysis.bpm.as_ref().map(|b| b.bpm)),
        fmt(analysis.bpm.as_ref().map(|b| f64::from(b.confidence))),
        analysis
            .key
            .as_ref()
            .map(|k| k.musical.as_str())
            .unwrap_or(""),
        fmt(analysis.key.as_ref().map(|k| f64::from(k.confidence))),
        fmt(analysis.key.as_ref().map(|k| f64::from(k.clarity))),
        fmt(first),
        fmt(last),
        count.map(|c| c.to_string()).unwrap_or_default(),
        analysis.metadata.duration_analyzed_ms,
        elapsed_ms,
    );
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// If `ANALYZER_PROBE_DUMP_DIR` is set, write one beat time (seconds) per line
/// to `<dir>/<track-stem>.<backend>.beats` for offline grid comparison.
fn maybe_dump(track: &Path, label: &str, analysis: &TrackAnalysis) {
    let Ok(dir) = std::env::var("ANALYZER_PROBE_DUMP_DIR") else {
        return;
    };
    let Some(grid) = &analysis.beat_grid else {
        return;
    };
    let stem = track
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("track");
    let path = Path::new(&dir).join(format!("{stem}.{label}.beats"));
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body: String = grid
        .beats
        .iter()
        .map(|beat| format!("{beat:.6}\n"))
        .collect();
    match std::fs::write(&path, body) {
        Ok(()) => eprintln!("dumped {} beats to {}", grid.beats.len(), path.display()),
        Err(err) => eprintln!("failed to dump beats: {err}"),
    }
}

fn print_header() {
    println!(
        "{:<9} {:>9} {:>8}  {:<7} {:>8} {:>8}  {:>7} {:>6} {:>10} {:>10} {:>9}",
        "backend",
        "bpm",
        "conf",
        "key",
        "conf",
        "clarity",
        "beats",
        "bars",
        "downbeats",
        "stability",
        "ms"
    );
}

fn print_row(label: &str, analysis: &TrackAnalysis, elapsed_ms: u128) {
    let (bpm, bpm_conf) = match &analysis.bpm {
        Some(b) => (format!("{:.2}", b.bpm), format!("{:.2}", b.confidence)),
        None => ("-".into(), "-".into()),
    };
    let (key, key_conf, clarity) = match &analysis.key {
        Some(k) => (
            k.musical.clone(),
            format!("{:.2}", k.confidence),
            format!("{:.2}", k.clarity),
        ),
        None => ("-".into(), "-".into(), "-".into()),
    };
    let (beats, bars, downbeats, stability) = match &analysis.beat_grid {
        Some(g) => (
            g.beats.len().to_string(),
            g.bars.len().to_string(),
            g.downbeats.len().to_string(),
            format!("{:.2}", g.grid_stability),
        ),
        None => ("-".into(), "-".into(), "-".into(), "-".into()),
    };

    println!(
        "{label:<9} {bpm:>9} {bpm_conf:>8}  {key:<7} {key_conf:>8} {clarity:>8}  \
         {beats:>7} {bars:>6} {downbeats:>10} {stability:>10} {elapsed_ms:>9}"
    );
}

/// Print beat-grid phase details (first/last/times) for grid comparison.
fn print_grid(label: &str, analysis: &TrackAnalysis) {
    let Some(grid) = &analysis.beat_grid else {
        println!("{label:<9} grid: (none)");
        return;
    };
    if grid.beats.is_empty() {
        println!("{label:<9} grid: (no beats)");
        return;
    }
    let first = grid.beats.first().copied().unwrap_or(0.0);
    let last = grid.beats.last().copied().unwrap_or(0.0);
    let head: Vec<String> = grid
        .beats
        .iter()
        .take(6)
        .map(|beat| format!("{beat:.3}"))
        .collect();
    println!(
        "{label:<9} grid: first={first:.4}s last={last:.4}s count={} head=[{}]",
        grid.beats.len(),
        head.join(", ")
    );
}

/// Positional args win; then `ANALYZER_PROBE_TRACK`; then the fallback sample.
/// Directory arguments are expanded to their audio files (recursively).
fn resolve_tracks() -> Vec<PathBuf> {
    let args: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let raw = if !args.is_empty() {
        args
    } else if let Ok(value) = std::env::var("ANALYZER_PROBE_TRACK") {
        let paths: Vec<PathBuf> = value
            .split([':', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect();
        if paths.is_empty() {
            vec![PathBuf::from(DEFAULT_TRACK)]
        } else {
            paths
        }
    } else {
        vec![PathBuf::from(DEFAULT_TRACK)]
    };
    raw.into_iter().flat_map(expand_path).collect()
}

const AUDIO_EXTS: &[&str] = &[
    "mp3", "wav", "flac", "ogg", "opus", "m4a", "aac", "aiff", "aif", "wv", "mp4",
];

/// A directory expands to its audio files; anything else is taken as-is.
fn expand_path(path: PathBuf) -> Vec<PathBuf> {
    if path.is_dir() {
        let mut files = Vec::new();
        collect_audio_files(&path, &mut files);
        files.sort();
        files
    } else {
        vec![path]
    }
}

fn collect_audio_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_audio_files(&path, out);
        } else {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase);
            if ext.as_deref().is_some_and(|e| AUDIO_EXTS.contains(&e)) {
                out.push(path);
            }
        }
    }
}

/// `0`/`full` means analyze the whole track; anything else is a millisecond cap.
fn resolve_max_ms() -> Option<i32> {
    match std::env::var("ANALYZER_PROBE_MAX_MS") {
        Ok(value) if value.eq_ignore_ascii_case("full") || value == "0" => None,
        Ok(value) => value
            .parse::<i32>()
            .ok()
            .filter(|ms| *ms > 0)
            .or(Some(DEFAULT_MAX_MS)),
        Err(_) => Some(DEFAULT_MAX_MS),
    }
}
