//! Shared fixture plumbing for the analyzer benches.
//!
//! Both bench binaries measure the same real audio, so the fixture lookup, the
//! config builder, and the fixture constants live here rather than being
//! duplicated. See `analysis.rs` for why a missing fixture is a skip.
//!
//! Each bench binary compiles this module separately and uses a different
//! subset — `memory` needs the fixture lookup and configs, `analysis` also needs
//! the probe cache — so unused items in one binary are expected, not dead.

// A shared support module, included per bench target with different needs.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use analyzer::{decode_mono, AnalysisConfig, DecodedMono};

/// Real 387.1 s stereo 24-bit 44.1 kHz master (see `samples/README.md`).
pub const MASTER: &str = "Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav";
/// Compressed master, exercising the Opus decoder path (181.2 s at 48 kHz).
pub const OPUS: &str = "Palawan by SKIRK  Vlog Music [xXRDR-ycIeo].opus";

/// Native rate of [`MASTER`], asserted against the probe so a scenario's
/// throughput denominator cannot drift away from the fixture.
pub const MASTER_SR: u32 = 44_100;
/// `AnalysisDurationMode::Precise` on a real track.
pub const PRECISE_MS: i32 = 193_000;
/// `AnalysisDurationMode::Fast`.
pub const FAST_MS: i32 = 30_000;

/// A fixture inside the repo-root `samples/` tree, or `None` with a printed
/// reason.
///
/// The benches are a local measurement instrument, so a missing fixture is a
/// skip, not a panic in the middle of a run.
pub fn fixture(name: &str) -> Option<PathBuf> {
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

pub fn config_for(max_duration_ms: Option<i32>) -> AnalysisConfig {
    AnalysisConfig {
        max_duration_ms,
        ..AnalysisConfig::default()
    }
}

/// Frame count `decode_mono` decodes for `secs` at [`MASTER_SR`].
///
/// `.ceil()` mirrors the decoder's own cap formula (`decode.rs`: a duration cap
/// of `secs` becomes that many frames), so assertions built on this hold for any
/// duration, not only ones whose frame count is a whole number.
pub fn frames_for(secs: f64) -> u64 {
    (secs * f64::from(MASTER_SR)).ceil() as u64
}

/// Decode `path` under `config`, memoized for the process.
///
/// Every throughput denominator and benchmark id is derived from a probe decode
/// of the fixture. The same (fixture, config) pair can be probed by more than
/// one target — `decode_mono` and `analyze_file` share three — so the decode is
/// paid once per process instead of once per target. The probe runs before the
/// harness starts timing, so it is setup work, not measured work.
pub fn probe(path: &Path, config: &AnalysisConfig) -> DecodedMono {
    let key = (path.to_path_buf(), config.max_duration_ms);
    if let Some(hit) = PROBES
        .lock()
        .expect("probe cache is never poisoned")
        .get(&key)
    {
        return hit.clone();
    }
    let decoded = decode_mono(path, config)
        .unwrap_or_else(|error| panic!("fixture {} must decode: {error}", path.display()));
    PROBES
        .lock()
        .expect("probe cache is never poisoned")
        .insert(key, decoded.clone());
    decoded
}

/// Probes keyed by (fixture, duration cap): the properties they establish —
/// native sample rate and decoded frame count — depend on both.
type ProbeCache = Mutex<HashMap<(PathBuf, Option<i32>), DecodedMono>>;

static PROBES: LazyLock<ProbeCache> = LazyLock::new(|| Mutex::new(HashMap::new()));
