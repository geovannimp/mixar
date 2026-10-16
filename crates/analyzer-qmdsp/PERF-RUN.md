# qmdsp-analysis-optimization — run record

Branch `improve/2026-10-15-qmdsp-tempo`, base commit `ea8041cb`, final commit
`60b0017a`. Crate `crates/analyzer-qmdsp` (Mixar's only analysis backend: the
qm-dsp beats/key port).

Measured with `examples/qmdsp_timing` (added in this run). 240 s corpus: 128 BPM
percussion + a C major chord, deterministic, no RNG. Release mode, host
`-C target-cpu=x86-64-v3` (AVX2+FMA, 16 cores). `min_ms` over 5 reps after
1 warmup is the statistic; `mean_ms` is printed for drift detection. Every A/B
was run serially, never two measurements at once.

`B0` is the instrumented baseline (recorded before any behavioural change);
`B_pristine` is the pristine-base baseline that proves the instrument neutral.
Both are byte-identical on the `fingerprint` lines:
`beats=511 bpm=128.000000 key=C downbeats=128 hash=bac7cf82f007574f`.

## Final scoreboard vs B0

Two full runs of the final commit; both shown because the machine drifts.

| row | B0 min_ms | final #1 | rel | final #2 | rel | B0 peak kB | final peak kB | rel |
|---|---|---|---|---|---|---|---|---|
| `analyze_full` | 621.337 | 208.872 | **−66.4%** | 217.699 | −65.0% | 236256 | 142856 | **−39.5%** |
| `analyze_bpm` | 458.609 | 147.757 | **−67.8%** | 151.288 | −67.0% | 236256 | 142868 | −39.5% |
| `analyze_grid` | 570.573 | 197.385 | **−65.4%** | 203.475 | −64.3% | 236408 | 143028 | −39.5% |
| `analyze_key` | 89.978 | 85.410 | −5.1% | 86.334 | −4.0% | 236408 | 143036 | −39.5% |
| `sub_init` | 4.456 | 4.542 | +1.9% | 4.531 | +1.7% | 236408 | 143036 | −39.5% |
| `sub_onset` | 405.033 | 89.480 | **−77.9%** | 97.303 | −76.0% | 236408 | 143260 | −39.4% |
| `sub_beat_period` | 16.167 | 13.620 | **−15.8%** | 13.420 | −17.0% | 236408 | 143260 | −39.4% |
| `sub_beats_dp` | 1.189 | 1.250 | +5.1% | 1.258 | +5.8% | 236408 | 143260 | −39.4% |
| `sub_chroma` | 7.199 | 7.651 | +6.3% | 7.415 | +3.0% | 236408 | 143260 | −39.4% |
| `sub_decimate8` | 73.781 | 29.148 | **−60.5%** | 28.826 | −60.9% | 324376 | 159516 | **−50.8%** |
| `sub_downbeat` | 107.304 | 21.388 | −80.1%† | 21.136 | −80.3%† | 329432 | 158868 | −51.8%† |

† `sub_downbeat`'s **meaning changed**: from C3 it receives the pre-decimated
stream, so the ×8 pass moved outside the timed closure. Do not read this as a
like-for-like −80%; `analyze_grid` is the honest row for the shared-decimation
saving.

### Noise calibration

`sub_init` and `sub_chroma` have byte-identical code in `ea8041cb` and in the
final commit (the changes that would have touched them, C2 and C6, were
reverted). They drift +1.7% to +6.3% between B0 and the final runs, which is the
machine's noise band on this host. The noise band is ±5–6%; the rows whose code
did not change move within it, and only rows far outside it are real wins.
`sub_beats_dp` (+5.1%) sits at the edge and is 0.06 ms of absolute change.

### Parity

Every kept commit carries `fingerprint` byte-identical to B0
(`beats=511 bpm=128.000000 key=C downbeats=128 first=0.934603 last=239.055237
mean=119.992744 hash=bac7cf82f007574f`), and the dumped beat grids are
byte-identical to the base (`diff beats_final.txt beats_base.txt` → clean,
511 beats, `max_abs_diff_s=0.000000000`). `click_track_yields_the_click_tempo`
(BPM within 5 of 120) passes on the final commit.

## Kept changes

| commit | change | evidence |
|---|---|---|
| `06b85155` | timing harness + `internals` re-export; `Cargo.toml` default-on `sub` feature | pristine baseline 626.082/465.632/580.422/90.478 vs B0 621.337/458.609/570.573/89.978 (within 1.7%); `fingerprint` byte-identical across `B_pristine`, `B0_pipeline` and `B0` |
| `a4c6e178` | onset: unit-phasor rotation replaces per-bin `atan2`+`princarg`+`cos`/`sin` | `sub_onset` 411.571/411.039 → 90.581/88.715 (−78%); oracle `complex_sd_matches_the_reference_definition` green on the old code first |
| `c6ab3433` | streaming `Decimator` + `Decimation` enum; B4/A4 tables and the factor-1 copy branch deleted | `sub_decimate8` 75.948 → 69.171/28.741; `analyze_grid` 258.772 → 196.083/198.117; peak 236720 → 143716 KiB (−39%) |
| `5c8d2a42` | one factor-8 decimation shared by key and downbeat | bit-exactness proven by throwaway test: 7 windows × 4096 samples, 0 `to_bits()` mismatches; `analyze_full` 240.979 → 207.449/206.601 (−14%) |
| `287b8d52` | bound the decimation sweep to `output_len` writes (fixes the panic C4 introduced) | teeth check: flipping `i % self.factor == 0` fails all 4 decimator tests |
| `cb6556a5` | hoist tempo block-loop scratch; flatten `tmat`/`delta`/`psi`; delete `math::max_val`/`max_ind` | `sub_beat_period` 16.181 → 13.053/13.070/13.075 (−19%); peak 143528 → ~142800 |
| `578a3819` | memoise the downbeat Hann; `spectral_diff` without per-beat allocations | 5-round `sub_downbeat` A/B: min 20.677 → 20.257 (−2.0%), mean of clean rounds −0.5%; `analyze_grid` statistically identical (min −0.04%) |
| `60b0017a` | delete dead `QmdspAnalyzer::from_env` | no callers (`grep -rn from_env --include=*.rs .`: only `analyzer-stems`' unrelated `force_ep_from_env`) |

## Removed code (net line count)

`git diff ea8041cb..HEAD --stat` → 9 files, **692 insertions, 272 deletions**.
Of that, `examples/qmdsp_timing.rs` is +286 (the instrument) and `Cargo.toml`
+7 (the `sub` feature). Net library change: **+419 / −272**.

Deleted outright:
`math::princ_arg`, `math::mod_floor`, `math::max_val`, `math::max_ind`,
`math::tests::princ_arg_wraps_into_range` (−17 lines net), decimator `B4`/`A4`
tables (16), decimator `B1`/`A1` tables and the `factor == 1` copy branch plus
`factor_one_is_a_copy` (10), `downbeat::decimate` (14),
`GetKeyMode`'s `decimator`/`dec_buffer` fields and `block_size`/`hop_size` (6),
`QmdspAnalyzer::from_env` (5). Per-change shortstats are in each commit.

## Tried and rejected, with measured results

| change | why | numbers |
|---|---|---|
| fused window+fftshift in `onset.rs` + `cqt.rs` (`d317254e`, reverted `7fbe725e`) | target row `sub_onset` within noise; `analyze_key` +2.8% in both C2 runs | 3 C1 vs 2 C2 samples; `min_ms` spread within one unchanged binary (±3.5%) wider than any commit difference |
| sharing the ×8 decimation before C4 (`c2f0e81e`, reverted `f18bbfce`) | standalone `analyze_key` 44% slower: the whole-track 84.8 MiB scratch | parent 97.165 → 139.783/143.854; re-landed on C4 where the scratch is gone |
| hoisting the Krumhansl denominators (`c44ba437`, reverted `9f7ce4c7`) | consistent regression on its own row | 3 interleaved rounds: 80.447→83.528 (+3.8%), 80.747→83.440 (+3.3%), 91.739→92.933 (+1.3%); key name stayed `C` |
| lag-interleaved autocorrelation in `get_rcf` (inside `cb6556a5`'s development) | 19% regression on its target row despite being bit-identical | throwaway proof `mismatches = 0 / 512`; then `sub_beat_period` 13.05 → 15.433/15.436 (two runs within 0.003 ms of each other) |

Also not pursued (from the plan's Rejected list, unchanged): parallel IIR
sections for the decimator; `overlap::feed` zero-copy; `adaptive_threshold`
scratch reuse; f32→f64 inside `feed` to drop the whole-track `y`; rustfft `avx`
(already default); a hand-rolled real FFT; removing `Chromagram`'s conjugate
mirror; rayon parallelism (barred by `docs/audio-analyzer-spec.md` §6.2);
`RealFft` exposing its `Complex` spectrum; dependency bumps.

## Decisions made alone

1. **C4 before C3.** Sharing the decimation first exposed C4's 84.8 MiB scratch
   and made `analyze_key` 44% slower. C4 is bit-identical and changes no
   signature, so it lands standalone; C3 then lands on top of it and keeps the
   saving the per-block path had. Both changes have their own measured
   evidence.
2. **`Cargo.toml` gained a default-on `sub` feature** so the harness's sub-stage
   rows (which reach the crate's private modules through `internals`) can be
   compiled out. This is what lets the pristine-baseline build work against
   `ea8041cb`'s `src` via `--no-default-features --pipeline-only`; it costs no
   dev-dependency and does not touch `Cargo.lock`.
3. **`prev_prev_*` initialise to `(1, 0)`, not `(0, 0)`.** qm-dsp's
   `phase_history_old` starts at angle 0, and `e^{i*0} = (1, 0)`. The oracle
   caught the zero phasor (`frame 0: got 0, reference 256`).
4. **The oracle applies the Hann twice.** `process_time_domain` already windows
   its input, which the oracle pre-windowed; without the second multiply the
   oracle fails on unchanged code (`frame 1: got 923.407, reference 1384.849`).
5. **C7 keeps a locally-spelled Hann** rather than reusing `window::hann`:
   `0.5 - 0.5*cos(..)` and `0.5 * (1.0 - cos(..))` are not bit-identical, and
   the downbeat's spectral difference is too parity-sensitive to trade a
   last ulp for a line.
6. **Reverts stay as commit pairs rather than being squashed away**, so the
   record of what was tried and what it cost survives in `git log`.

## Unverified items

- **CI was not run.** Repo `AGENTS.md` asks for CI after pushing; the run's
  constraint forbids pushing, so no CI signal exists for this branch.
- **Per-commit gates were crate-scoped** (`cargo nextest run -p analyzer-qmdsp`).
  The workspace suite ran once, on the final commit: `cargo nextest run
  --workspace` → 766 passed, 2 skipped; `cargo test --doc` → ok; `cargo clippy
  --all-targets --all-features -- -D warnings` → clean; `cargo fmt --check` →
  ok. `analyzer-stems` built, so the plan's fallback exclusion was never needed.
- **C2's and the first C3's commit bodies are pre-measurement drafts.** They
  carry the change and its gates; the measurements that condemned them live in
  the revert commits (`7fbe725e`, `f18bbfce`).
- The `%`-of-noise calibration rests on two runs of the final commit. Individual
  row deltas inside ±6% on this host are not reproducible from a single run.

## Next ledger items

1. **The whole-track `y: Vec<f64>` (84.8 MiB, ~40 ms of `analyze_full`)** is
   the largest un-attributed cost left. The sub-rows sum to ~167 ms against
   `analyze_full`'s ~207 ms; the gap is the `samples.iter().map(|&s|
   f64::from(s)).collect()` in `lib.rs`. Streaming it touches the shared
   decimation and the beats path at once, so it is its own measured change.
2. **Cache the CQT kernel per `(fs, min, max, bpo, cq_thresh)`** behind an
   `Arc` + `OnceLock`, keyed by `to_bits()`, capped at ~8 entries (the shape
   `crates/codec` already uses). `sub_init` is 4.4 ms/track today, repeated per
   track at a given sample rate.
3. **Pack the onset per-bin history into one array-of-structs** (5 arrays → 1):
   the loop is scalar `sqrt`/`div`-bound, so this may land below noise.
4. **Flatten `rcfmat` to one contiguous `Vec<f64>`** and narrow `psi` to `u8`
   (all `arg` values live in `20..q-20`, `q == 128`): −2.5 MB peak, ~0.3 ms.
5. **`backlink: Vec<i64>` → `Vec<i32>`** in `calculate_beats`: −2.9 MB peak,
   ~0.2–0.4 ms.
6. **Correctness items the visibility lens raised, deliberately out of scope
   for a perf run,** in priority order:
   - `lib.rs`/`get_key_mode.rs`: silent or too-short audio reports a definite
     key (`"Cm"`) with `confidence: 0.0`. Divergence from qm-dsp by design, so
     it needs an explicit decision.
   - `tempo.rs`: `calculate_beats` divides by a beat period that can be zero
     (`rcfmat.len() < 2`), producing NaN that a comparison silently discards.
   - `decimator.rs`: `process`'s length contract is `debug_assert`-only, so a
     release build silently ignores a too-long `src` and leaves a stale `dst`
     tail.
