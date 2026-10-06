# analyzer-qmdsp — pure-Rust qm-dsp analysis

A pure-Rust port of the two analyses [Mixxx](https://mixxx.org) runs via
[qm-dsp](https://github.com/c4dm/qm-dsp): `AnalyzerQueenMaryBeats` (BPM/beat
grid) and `AnalyzerQueenMaryKey` (musical key). No FFI, no C++ toolchain.

This is Mixar's only analysis backend; the `analyzer` facade uses it by default.

## What is ported

Generic DSP comes from established crates; only qm-dsp-specific algorithms are
ported:

| concern | source |
|---|---|
| FFT | [`rustfft`](https://crates.io/crates/rustfft) (complex constant-Q kernel) + [`realfft`](https://crates.io/crates/realfft) (real STFT/chroma) instead of bundled kissfft |
| onset | `DetectionFunction` (`DF_COMPLEXSD`) + phase-vocoder front end |
| tempo | `TempoTrackV2` — comb-filter bank, Viterbi tempo contour, Ellis DP beat tracker |
| BPM | `analyzer-core`'s port of Mixxx `BeatUtils` (constant-region ironing + integer `roundBpmWithinRange`), not a median |
| key | `Decimator` (IIR ×8), `ConstantQ`/`Chromagram`, `GetKeyMode` (HPCP + Krumhansl profiles + median filter) |
| framing | Mixxx `DownmixAndOverlapHelper` (centred first window, padded finalize) |

The Hann/Hamming windows are reimplemented because qm-dsp's are *periodic*
(not the symmetric windows most libraries provide), so a generic window crate
would not match.

## Parameters (Mixxx defaults)

- **Beats:** step `sample_rate * 0.01161`, window `next_pow2(sample_rate / 50)`,
  `DF_COMPLEXSD`, trailing-non-positive trim, skip first 2 frames, then
  `TempoTrackV2` (`inputtempo = 120`, `alpha = 0.9`, `tightness = 4.0`).
  Beat positions are `frame * step + step/2` samples.
- **Key:** `GetKeyMode` with 440 Hz tuning, `hpcpAverage = 10`,
  `medianAverage = 10`, overlap 1, decimation 8; the dominant key is the most
  frequently reported window key.

## Accuracy

Against file tags on Mixar's 12-track Hercules pack (a C++ qm-dsp reference used
during development agreed with this port on 11/12 tracks, to within ~0.2 ms per
beat):

- **BPM** — exact 11/12, within ±1 BPM 11/12 (the one miss is a 3:2 metrical lock)
- **Key** — exact 5/12, exact-or-relative 10/12
- **Grid phase** — 27 ms mean vs the embedded Serato grid

## Licensing

A port of qm-dsp (GPL-2.0-or-later) is a derivative work; it stays under
Mixar's GPLv3.
