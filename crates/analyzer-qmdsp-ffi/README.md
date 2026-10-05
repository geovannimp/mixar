# analyzer-qmdsp-ffi — qm-dsp oracle

> **Oracle / non-product.** Only wired into `analyzer-probe` as `qmdsp-ffi`.
> The pure-Rust port lives in `crates/analyzer-qmdsp`; this crate wraps the real
> C++ library so the port can be differential-tested against it.

Runs Mixxx's analysis library [qm-dsp](https://github.com/c4dm/qm-dsp) through a
small C ABI shim (`src/qm_shim.cpp`), replicating Mixxx's defaults:

- **Beats** — complex-domain onset detection (`DF_COMPLEXSD`, ~11.61 ms step,
  window `next_pow2(sr/50)`) → `TempoTrackV2` (Davies comb-filterbank/Viterbi +
  Ellis DP beats), exactly like `AnalyzerQueenMaryBeats`.
- **Key** — `GetKeyMode` (36-bin chromagram/HPCP, 440 Hz tuning,
  `hpcpAverage=10`, `medianAverage=10`, overlap 1, decimation 8), like
  `AnalyzerQueenMaryKey`. The dominant key is the most frequently reported one.
- Window feeding mirrors Mixxx's `DownmixAndOverlapHelper` (centred first window,
  zero-padded finalize).

## Build

qm-dsp source is **not vendored** (it's GPL C++). Point `QMDSP_SOURCE_DIR` at a
checkout of Mixxx's `lib/qm-dsp`; the default is
`~/.cache/mixar/mixxx-src/lib/qm-dsp`, which you can create with a sparse clone:

```bash
git clone --filter=blob:none --no-checkout --depth 1 \
  https://github.com/mixxxdj/mixxx.git ~/.cache/mixar/mixxx-src
cd ~/.cache/mixar/mixxx-src
git sparse-checkout set lib/qm-dsp
git checkout
```

Needs a C++17 toolchain. `-msse -msse2 -mfpmath=sse` are applied only on x86;
other architectures build without them. If the source is missing, the crate
builds empty and `QmdspFfiAnalyzer::from_env()` returns `None`, so the workspace
still builds.

## Licensing

qm-dsp is **GPL-2.0-or-later**, which is compatible with Mixar's GPLv3. Because
we link it (rather than redistribute its source), anyone building the backend
must obtain qm-dsp themselves; that satisfies the source-sharing obligation.

## Parity notes

- Beat positions come back in samples and are divided by the sample rate.
- BPM uses `analyzer-core`'s port of Mixxx `BeatUtils` (constant-region ironing +
  `roundBpmWithinRange`), the same derivation Mixxx displays.
- qm-dsp has no per-frame confidence, so `BpmAnalysis.confidence` is a
  beat-interval regularity heuristic and `KeyAnalysis` confidence/clarity are 0.
