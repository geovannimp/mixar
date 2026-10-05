# analyzer-qmdsp — pure-Rust qm-dsp analysis

A pure-Rust port of the two analyses [Mixxx](https://mixxx.org) runs via
[qm-dsp](https://github.com/c4dm/qm-dsp): `AnalyzerQueenMaryBeats` (BPM/beat
grid) and `AnalyzerQueenMaryKey` (musical key). No FFI, no C++ toolchain.

Like the other experimental backends it is only wired into `analyzer-probe`;
`analyzer-qmdsp-ffi` wraps the real C++ library and exists **only as a
differential-test oracle** for this port.

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

## Parity

`analyzer-probe` runs this crate as `qmdsp` and the C++ shim as `qmdsp-ffi`, so
they can be compared directly. On the Hercules test pack the two agree exactly
(BPM, key, beat count identical; beat times within ~0.2 ms) on 11/12 tracks;
the remaining track differs by one tempo-contour near-tie because `rustfft`
and kissfft round differently.

## Licensing

A port of qm-dsp (GPL-2.0-or-later) is a derivative work; it stays under
Mixar's GPLv3.
