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

- **CI was run for this branch.** PR #543 (`geovannip/optimize-qmdsp-tempo`,
  base `ea8041cb`) was pushed and the suite went green: build, test,
  format + lint, cargo audit and npm audit all pass; `analyzer-stems` builds,
  so the plan's fallback exclusion was never needed. The two AI review bots
  (open-code-review, Kody) also ran and all four threads they raised are
  replied to and resolved — two of them were real bugs this run introduced,
  fixed in `f42eae39` with enforcing tests (see the corrections section above).
- **Per-commit gates were crate-scoped** (`cargo nextest run -p analyzer-qmdsp`).
  The workspace suite was re-run on the final pushed commit: `cargo nextest run
  --workspace` → 768 passed, 2 skipped (27 in `analyzer-qmdsp`, up from 25 with
  the two new parity tests); `cargo test --doc` → ok; `cargo clippy
  --all-targets --all-features -- -D warnings` → clean; `cargo fmt --check` →
  ok.
- **C2's and the first C3's commit bodies are pre-measurement drafts.** They
  carry the change and its gates; the measurements that condemned them live in
  the revert commits (`7fbe725e`, `f18bbfce`).
- The `%`-of-noise calibration rests on two runs of the final commit. Individual
  row deltas inside ±6% on this host are not reproducible from a single run —
  re-measuring `f42eae39` hours later put every row 20–40% above the earlier
  runs from machine load alone, with parity unaffected.

## Post-review corrections (found by this run's own parity checks)

Two bugs, both introduced by this run's own changes, both caught by tests
written against the pre-change code. Both fixed in `f42eae39`.

**1. The streaming decimation dropped tail state.** `anti_alias_decimating`
bounded its sweep to `(output_len - 1) * factor`, so for `input_len = 32768,
factor = 8` it stopped at input 32760 and never swept the last 7 samples. The
pre-C4 code swept every input and only *read back* `output_len` of them, so the
state was complete for the next call. Latent in the final code (the shared pass
is a single call), but it is the reason the shared key path diverged from the
per-block path it replaced. The sweep now covers every input and keeps the first
`output_len` samples.

**2. The shared ×8 stream was not what the old key path decimated.** The shared
sweep stopped at `y.len()`, while the per-block key path swept `overlap::feed`'s
whole key window sequence — centred zero pre-roll, track, then the silence
`finalize` appends to complete the last window. So the final chroma frame, and on
clips shorter than one block the only frame, saw hard zeros where the old path
saw the filter's ring-out decayed from the real audio; the key vote could differ
on any length that is not a multiple of the block size. Fixed by deriving the
pad from the same formula `overlap::feed` uses and sweeping `track ++ pad`, with
the centred pre-roll prepended in the decimated domain — equivalent, because zero
initial state maps to exactly zero output, so no large copy is needed.
`detect_key` reads consecutive `frame_size()` blocks instead of re-cutting them
with `feed`, which is also what removes `feed`'s per-frame copy and finalize
padding from this path.

Enforced by `shared_key_decimation_matches_the_old_per_block_path`
(`src/lib.rs`) — it rebuilds the pre-shared key path with one stateful
`Decimator` reused across every window `overlap::feed` emits and compares all
4096 samples of every frame `to_bits()`-exact across 13 track lengths spanning
sub-block, block-boundary, exact-multiple and non-multiple cases — and by
`state_survives_a_call_with_a_partial_group` (`decimator.rs`). Both were verified
to fail against the code they were written for: restoring the truncated sweep
fails both, and they pass on the fix.

Measured, the correction is also a **6% win** on the pipeline rows:
`analyze_full` 240.054 → 224.926, `analyze_bpm` 172.234 → 161.456,
`analyze_grid` 229.199 → 214.482, `analyze_key` 100.651 → 94.949,
`sub_beat_period` 14.180 → 13.472 (serial, 240 s corpus, `min_ms` over 5 reps
after 1 warmup, parent and fix measured back to back). The win comes from the
key path no longer copying every frame through `feed`'s buffer and no longer
running `feed`'s finalize padding on the decimated stream.

Fingerprint and dumped beat grids remain byte-identical to base throughout.

## Third review wave (instrumentation only, all 11 resolved)

Both flagged bugs were verified against base and both are non-issues:

- **`dec8.len() < 2` guard** is *exactly* equivalent to base's
  `samples.len() < 16`. Base's `decimate` required 16 raw samples and produced
  `floor(len/16) >= 1` downsampled samples — one, not two — so the thread's
  premise was wrong, and `dec8.len() >= 2` is precisely `y.len() >= 16`.
  `x8_stream_matches_the_old_two_stage_decimate` rebuilds the old two-stage
  `decimate` and asserts identical decisions and bit-identical streams across 13
  lengths spanning the 15/16 boundary. Applying the suggested `dec8.len() < 4`
  would have *rejected* lengths base accepted (tested: it fails).
- **`&buf[..y_len]`** is what makes the padding safe: `buf` is only ever
  extended, so the slice is exactly the raw track whether or not the finalize
  silence was appended. `debug_assert`s now pin the three related invariants.

Also fixed in this wave: the redundant `internals` re-export deleted;
`DECIMATION_FACTOR` derived from `Decimation::By8.factor()` (now a `const fn`);
`--dump` streamed through a `BufWriter`; `frame_and_step`'s `sample_rate > 0`
precondition documented and pinned; the onset zero-magnitude comment reordered
to lead with the common case; and a latent `clippy::needless_return` in the
example that only fired with `sub` off.

**One earlier justification of mine was wrong and is corrected:** I claimed
opt-in `sub` would break `cargo clippy --all-targets` and `cargo test` because
the example would not compile. It compiles; nextest and the repo's
`--all-features` clippy pass with `sub` opt-in. The measured case for
default-on is instead the +0.96% rlib cost and the fact that the documented
one-command invocation would report 4 rows instead of 11.

## Fourth review wave (12 threads; one real 50% finding)

**Fixed — the zero-magnitude phasor branch reintroduced `atan2`/`cos`/`sin`.**
The branch fires for every bin of every silent frame, so a silent corpus
measured 150.3-152.3 ms on `sub_onset` against 91.5 ms for real audio. Guarding
on `re.is_sign_negative()` — the only case where the answer is not `(1, 0)` —
halved it: 74.453 / 75.429 / 79.848 ms (serial, 240 s silent corpus, `min_ms`
over 5 reps, three runs each). `complex_sd_matches_the_reference_definition`
still passes bit-exactly, so parity is unchanged.

**Declined — decrementing counter instead of `i % factor` in the decimation
hot loop.** Measured, and the counter was *slower*: `sub_decimate8` 29.0 ->
35.6 ms, `analyze_key` 82.1 -> 88.8 ms. The loop is bound by the dependent
recurrence chain on `o[0..7]`; the modulo is not on that chain. Reverted, with
the measurement recorded at the loop.

**Declined — beat-grid-only downbeat path.** Same question as wave 3's bug B:
base's downbeat path never used `overlap::feed` at all, it ran the private
`decimate(y)` over exactly `y.len()` samples. Now pinned by two tests,
`x8_stream_matches_the_old_two_stage_decimate` (bit-exact vs base across 13
lengths) and `padded_and_unpadded_downbeat_paths_agree` (the default
configuration's padded stream and the grid-only stream must agree).

**Fixed — VmHWM was reported per row but is process-wide.** `peak_vmhwm_kb`
after `sub_decimate8` was 242 552 KB, so every later row inherited it. `bench`
now reports `vmhwm_delta_kb` alongside it. On the final run the delta is 4 KB for
`analyze_full`, 0 for the pipeline and other sub rows, and 10 368 KB for
`sub_decimate8` — correctly identifying the one row that allocates.

Also fixed: the corpus's zero-initialise-then-overwrite pass; `--secs 0`;
stale `internals` reference; `saturating_add` in `decimation_input_len`; and
three new tests (signed-zero phasor, silent-to-audio state carryover, padded vs
unpadded downbeat agreement).

**Process note:** the commit for this wave failed twice with
`1Password: failed to fill whole buffer` — `SSH_AUTH_SOCK` pointed at GNOME
Keyring (no identities) while the signing key lives in 1Password's agent at
`~/.1password/agent.sock`, and signing needs interactive approval the agent
would not grant unattended. Resolved by the user unlocking 1Password and
approving; the commit is signed. Anyone hitting the same failure should check
`ssh-add -l` against `~/.1password/agent.sock` rather than assuming a broken key.

## Fifth review wave (5 threads; two confirmed edge bugs)

**Fixed — `--secs > 0` still allowed an empty corpus.** Confirmed in release:
`--secs 1e-6` passed the float guard and panicked at `.expect("analyze_full")`
with `Analysis("empty audio buffer")`. The guard now asserts on the derived
sample count, so `1e-6` / `0` / `nan` all fail up front with an actionable
message, and `--secs 1` still runs.

**Fixed — `decimation_input_len`'s saturating guard still overflowed.**
Confirmed in a `-O` repro with Mixxx's block: `(preroll.saturating_add(MAX)) /
block * block + block` wraps to **0** in release (panics in debug). End-to-end
`saturating_mul` / `saturating_add` keeps it at `usize::MAX`. Pinned by
`decimation_input_len_saturates_instead_of_wrapping_to_zero` (normal 240 s
length bit-identical to the old arithmetic; `usize::MAX` stays at `MAX`).

**Fixed — `debug_assert` said power-of-two but checked evenness.** Now uses
`is_power_of_two()`.

**Declined — rename feature `sub`.** Default-on internal gate; comment already
documents it. Rename is churn.

**Declined — seed banded Viterbi `arg` at `lo`.** On an all-zero row,
`get_max_index` returns 0; `arg = 0` matches base. Seeding `lo` would diverge
silence/degenerate back-pointers from qm-dsp (`0` → `20`).

Parity: fingerprint and dumped beats bit-identical to `e801261c`
(`hash=bac7cf82f007574f`). Gates: 779 passed / 2 skipped (38 in analyzer-qmdsp).

## Sixth review wave (12 threads; harness/docs mop-up + two false bugs)

Benchmark reconfirmed after all prior waves and after this wave's edits:
fingerprint `beats=511 bpm=128.000000 key=C downbeats=128 hash=bac7cf82f007574f`
bit-identical to base; framing `step=512 window=1024`. Representative rows
(240 s corpus, release, `min_ms` / 5 reps): `analyze_full` ~203–204,
`analyze_bpm` ~139–142, `analyze_key` ~79, `sub_onset` ~88,
`sub_decimate8` ~29, `sub_downbeat` ~21.

**Fixed (harness):** `bench` no longer wraps the result in `Option`; fingerprint
`with_capacity`; `--secs` ceiling at 3600; `--dump` errors include the path;
chroma row derives `fs` from `Decimation::By8.factor()`.

**Fixed (production hygiene):** Hann cache reuses via `clear`/`extend`; stale
`phase_history_old` comments; `measurement` module doc clarifies `pub(super)`
constants; Viterbi `tmat[j*q+i]` comment.

**Declined — `beat_len == 0` division by zero.** `(0..0).map(|j| … / beat_len)`
never runs the closure; empty range, no division. `continue` would change
qm-dsp-matching behaviour on collapsed extents.

**Declined — `frame_and_step` overflow at extreme rates.** `sample_rate` is
`u32`; the claimed overflow needs ~1.85×10¹¹ Hz. Documented on the helper.

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
     tail. (Fixed in `f42eae39` by sweeping every input and clamping the write
     count; the residual item is only that the *asserts* stay debug-only.)
