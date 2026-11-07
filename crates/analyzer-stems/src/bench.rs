//! Realtime Stems capability probe.
//!
//! Runs the StemgenRT graph on the CPU execution provider and reports how much
//! real time one 128-frame hop costs. The live producer is untouched: this only
//! loads a private session and measures forward passes, so it can run while the
//! engine is playing (never on the audio callback).
//!
//! The numbers feed the settings gate: live stems stay CPU-only, and a machine
//! is only offered the feature when this probe reaches realtime with margin.

use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::stemgen;

/// Frames per StemgenRT hop (the live ring publishes one hop at a time).
pub const HOP_FRAMES: usize = stemgen::HOP;

/// Audio budget for one hop, in milliseconds.
///
/// The producer has this much wall time to keep up with playback, so
/// `realtime = HOP_BUDGET_MS / avg_hop_ms`.
pub const HOP_BUDGET_MS: f64 = HOP_FRAMES as f64 / stemgen::SAMPLE_RATE as f64 * 1000.0;

/// Hops run (and discarded) before timing: the graph builds its recurrent
/// state during pre-roll, and a cold session would otherwise dominate.
pub const WARMUP_HOPS: u32 = 10;

/// Timed hops in the default probe.
pub const DEFAULT_BENCH_HOPS: u32 = 100;

/// Measured cost of one StemgenRT hop on the CPU EP.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HopBenchmark {
    /// Mean wall time of one timed hop.
    pub avg_hop_ms: f64,
    /// How many times faster than realtime the graph ran (`budget / avg`).
    pub realtime: f64,
    /// Number of timed hops (warm-up excluded).
    pub hops: u32,
}

impl HopBenchmark {
    /// Build a result from the measured wall time of `hops` forward passes.
    ///
    /// A zero-hop or zero-duration run reports `0.0`, which never passes the
    /// realtime gate; callers still bail before that on a misconfiguration.
    pub fn from_elapsed(elapsed: Duration, hops: u32) -> Self {
        let avg_hop_ms = if hops == 0 {
            0.0
        } else {
            elapsed.as_secs_f64() * 1000.0 / f64::from(hops)
        };
        Self {
            avg_hop_ms,
            realtime: realtime_multiple(avg_hop_ms),
            hops,
        }
    }
}

/// `HOP_BUDGET_MS / avg_hop_ms`, or `0.0` for a non-positive/invalid average.
pub fn realtime_multiple(avg_hop_ms: f64) -> f64 {
    if avg_hop_ms.is_finite() && avg_hop_ms > 0.0 {
        HOP_BUDGET_MS / avg_hop_ms
    } else {
        0.0
    }
}

/// Benchmark a StemgenRT artifact already on disk.
///
/// `weights` must exist; this never downloads and never starts the live
/// producer. `on_progress(completed, total)` fires after every hop, warm-up
/// included, so a caller can surface a progress bar.
pub fn benchmark_stemgen(
    weights: &Path,
    hops: u32,
    warmup: u32,
    on_progress: Option<&dyn Fn(u32, u32)>,
) -> Result<HopBenchmark> {
    stemgen::benchmark(weights, hops, warmup, on_progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hop_budget_is_one_128_frame_block_at_44k1() {
        assert!((HOP_BUDGET_MS - 2.902_494_331_065_759).abs() < 1e-12);
        assert!((realtime_multiple(HOP_BUDGET_MS) - 1.0).abs() < 1e-12);
        // Twice the budget is half realtime.
        assert!((realtime_multiple(2.0 * HOP_BUDGET_MS) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn benchmark_derives_realtime_from_the_average_hop() {
        let benchmark = HopBenchmark::from_elapsed(Duration::from_millis(300), 100);
        assert!((benchmark.avg_hop_ms - 3.0).abs() < 1e-9);
        assert!((benchmark.realtime - HOP_BUDGET_MS / 3.0).abs() < 1e-9);
        assert_eq!(benchmark.hops, 100);
    }

    #[test]
    fn zero_hops_and_invalid_durations_are_not_realtime() {
        let empty = HopBenchmark::from_elapsed(Duration::from_millis(10), 0);
        assert_eq!(empty.avg_hop_ms, 0.0);
        assert_eq!(empty.realtime, 0.0);

        assert_eq!(realtime_multiple(0.0), 0.0);
        assert_eq!(realtime_multiple(-1.0), 0.0);
        assert_eq!(realtime_multiple(f64::NAN), 0.0);
        assert_eq!(realtime_multiple(f64::INFINITY), 0.0);
    }
}
