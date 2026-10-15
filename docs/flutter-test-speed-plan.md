# Flutter test suite speed plan

> Status: **Phases 1 + 2 applied 2026-10-15** (`moon.yml`, `test.yml`), all gates green.
> Phase 3 investigated and closed with no change (evidence below).
> Flutter 3.47.0 (pinned `mise.toml`), 70 suites / 496 tests.

**Goal:** cut `moon run gui-flutter:test` wall time on CI (`ubuntu-latest` = 4 vCPU, 16 GB,
**no Dart build cache**) without changing any test's semantics or outcome.

**Measured basis** (16-core dev machine unless noted; `--reporter=json`, warm
`build/test_cache` unless noted):

| Config | Wall |
|---|---|
| 70 suites, default `-j` (cores/2 = 8), 16 cores | 24.6s |
| 70 suites, `-j 16`, 16 cores | 23.3s |
| 70 suites, `-j 2`, **4 cores** (`taskset -c 0-3`) | 48.5s |
| 70 suites, `-j 4`, 4 cores | **38.1s** |
| 70 suites, `-j 4`, 4 cores, **cold** `build/test_cache` | 56.2s |
| 70 suites, `-j 1` (serialized measurement run) | 96.7s |

**Cost model** (serialized `-j 1`, the only contention-free view): 50.1s suite load
(16.6s *serialized* compiles — ~237ms flat per suite, one `TestCompiler` at a time —
plus 33.5s `flutter_tester` spawn / isolate init) + 43.8s bodies (of which 16.2s is the
first-test warm-up: the first `MaterialApp` pumpWidget in each file costs 189ms, every
later one 13–17ms) + 2.8s overhead. Individual bodies are cheap: median 23ms, 82% <100ms.

## Rejected — with evidence

1. **`flutter test --experimental-faster-testing`** — broken SDK, not repo code.
   `_generateIsolateSpawningTesterPackageConfig`
   (`flutter_tools/lib/src/test/runner.dart:217`) parses the project's
   `.dart_tool/package_config.json` against the **project root** instead of the
   `.dart_tool/` dir holding it, so the root package's `rootUri: "../"` resolves to
   `apps/` and every `package:gui_flutter/…` import fails to compile (child spawner
   compile, exit 1, 0 tests run). Identical code in every locally installed SDK
   (3.44, 3.44.9, 3.47, 3.47.0, 3.47.5, latest) — a pin bump does not fix it.
2. **Suite consolidation** — measured net **negative**. Merging 5 tiny pure-logic suites
   (`tempo_format`, `key_format`, `jog_ticks`, `track_progress`, `analyzing_track_ids`
   → 1 file) saved exactly what the model predicts in `-j 1` (load 50.1s → 49.1s, compile
   chain 16.6s → 15.9s) but the default-concurrency wall went **up in 4/4 runs**
   (27.4 / 29.3 / 30.3 / 31.2s vs baseline 24.6 / 24.9 / 23.8 / 26.4 / 27.3s), with
   per-suite loads slower for 46 of 65 common suites (mean 1.58s → 1.90s). Do not
   consolidate for wall-clock reasons.
3. **Trimming per-suite imports** — compile time does not track import weight
   (r = +0.07; 166–327ms, mean 237ms; `widget_test.dart` with 29 lib imports compiles in
   308ms, `deck_jog_test.dart` with 5 in 262ms). The persistent `build/test_cache` kernel
   already absorbs the app code, so per-suite compiles are a flat, irreducible cost.

## Phase 1 — concurrency flag (applied: −13.6s / −28% on 4-core CI, wash on 16 cores)

package:test defaults to `max(1, cores/2)` concurrent suites. The compile chain is
serialized either way, so the extra concurrency is nearly free and it directly shortens
the body phase (43.8s of bodies → 8-wide ≈ 5.5s vs 4-wide ≈ 11s).

**Applied** in `apps/gui-flutter/moon.yml`, task `test`:

```yaml
  test:
    command: "bash"
    args:
      - "-c"
      - 'exec mise exec -- flutter test -j "$(nproc)"'
```

**Measured A/B** (warm kernel, "All tests passed" in every run):

| Config | Before (`cores/2`) | After (`$(nproc)`) | Δ |
|---|---|---|---|
| 4 cores (`taskset -c 0-3`), direct `flutter test` | 48.3s / 48.0s | 34.7s | **−13.6s (−28%)** |
| 4 cores, via `moon run gui-flutter:test --force` | ~48s | 33.1s | **−15s** |
| 16 cores, direct `flutter test` | 24.0s (`-j 8`) | 23.4s (`-j 16`) | wash |

16-core sweep for reference (warm): `-j 4` 32.1s, `-j 6` 25.3s, `-j 8` 24.0s,
`-j 12` 25.2s, `-j 16` 23.4s — so `$(nproc)` is never materially worse.

`nproc` honours the CPU affinity mask (verified: reports 4 under `taskset -c 0-3`), so
4-core CI gets `-j 4` and 16-core dev gets `-j 16` with no hardcoded number. `-j 8`
oversubscribes and is slightly worse than `-j 4` on 4 cores (40.6s vs 38.1s), so
`$(nproc)` is the right ceiling. `inputs` / `options.runInCI` unchanged.

Cheaper alternative if a shell wrapper is unwanted: leave `command: "mise"` and append
`"-j", "4"` — saves 10.4s on CI but forces 4-way concurrency on developer machines
(−25s locally vs `-j 16`). Not recommended.

## Phase 2 — cache the Flutter test kernel in CI (applied; expect −18.1s cold)

CI caches Rust (`Swatinem/rust-cache`) but no Dart artifact, so every run starts with an
empty `build/test_cache` and pays a cold kernel build: 56.2s vs 38.1s warm on 4 cores
(−18.1s measured).

**Applied** in `.github/workflows/test.yml`, after "Install npm dependencies":

```yaml
      - name: Cache Flutter test kernel
        uses: actions/cache@v6
        with:
          path: apps/gui-flutter/build/test_cache
          key: ${{ runner.os }}-flutter-test-kernel-${{ hashFiles('apps/gui-flutter/pubspec.lock') }}
          restore-keys: |
            ${{ runner.os }}-flutter-test-kernel-
```

Keyed on the dependency snapshot (`actions/cache@v6` to match the repo's other cache
steps). The cached dill is *incrementally* extended by each run, so a stale entry from a
`restore-keys` hit stays valid — exactly like local dev. Expected first-run effect on CI:
`Run tests` ≈ 56s → ≈ 38s (combined with Phase 1, ≈ 66s → ≈ 33s on the CI profile).

## Phase 3 — the 48 slowest tests (investigated 2026-10-15: no change recommended)

48 tests exceed 300ms, holding 28.5s of the 43.8s aggregated body. Split by cause:

| Cause | Count | Aggregated | Fixable? |
|---|---|---|---|
| First test in its file (per-file warm-up: first `MaterialApp` mount 189ms, first big-tree pump 300–530ms) | 36 | 22.1s | No — structural, one per file, one per isolate |
| Mounts a real tree / decodes real data (12 tests) | 12 | 6.4s | No — the cost *is* the work |

The 12 non-warm-up tests were each read and measured: `widget_test` ×5 (full `AppShell`
pumps + taps + settles), `mixer_panes` `MixerPage`, `settings_l10n_smoke` (two locale
mounts), `fader_slider`/`rotary_knob` (single drag in a minimal tree), `pad_grid`,
`level_meter` (2 meters, 36 `DecoratedBox`es), `app_typography`. Each already pumps the
minimum tree its assertions cover; the only remaining lever would be asserting less
(coverage regression) or pumping smaller stand-in trees (semantics regression).

Two further levers were measured and rejected:

- **Sharding in CI** (`--total-shards=4 --shard-index=k`): max shard 33.6s on 4 cores vs
  33.1s unsharded — a shard keeps ~18 suites' compile+spawn fixed costs, so a 4-way matrix
  buys ~0-5% for 4× the CI cache and job orchestration.
- **Hoisting the theme build** out of per-test helpers (it *looked* promising at
  26–57ms/call): a controlled probe showed the difference is one-time code-path warm-up
  per isolate, not a per-pump cost (fresh-theme pump 253ms vs cached 19ms on the *first*
  pump of a test body; both 3–4ms afterwards).

**Conclusion:** the per-test bodies are at their legitimate floor. CI wall is now bound by
per-suite fixed costs (70 × 237ms serialized compile + 70 × 480ms spawn), which only a
runner change (the broken `--experimental-faster-testing`) or a suite-consolidation
refactor (measured net negative) could remove. Phase 3 closed with no code change.

## Verification — run 2026-10-15, all green

| Gate | Result |
|---|---|
| `moon run gui-flutter:test --force` (×2, 16 cores) | 496 passed both; 23.9s / 25.0s |
| `taskset -c 0-3 moon run gui-flutter:test --force` (CI profile) | 496 passed; 33.1s (was ~48s) |
| 4-core A/B direct (`-j 2` → `-j $(nproc)`) | 48.3s/48.0s → 34.7s |
| Single file (`flutter test test/tempo_format_test.dart`) | 8 passed |
| `flutter test --test-randomize-ordering-seed=random` | 496 passed |
| `npm run test` (`rust:test`, `gui-flutter:test`, `website:test`) | 3/3 completed (cached) |
| `flutter analyze` | not re-run — no Dart source changed |

Rollback: Phase 1 = restore `args: ["exec", "--", "flutter", "test"]`;
Phase 2 = delete the cache step; Phase 3 = revert the individual test edits.

## Appendix — consolidation audit (reference only; not recommended)

Ready if a future need (e.g. sharded serial runs) makes it worthwhile:

- 26 of 70 suites are pure logic (`test()` only, no `testWidgets`) — merging them is
  zero framework-state risk. Best clusters: `library_events` + `artwork_cache` +
  `deck_library_track_lookup` + `analyzing_track_ids` (26 tests);
  `absolute_engine_cmds` + `engine_playheads` (+`engine_ui`, 49 tests);
  `waveform_math` + `waveform_overlay` (36 tests); `settings_defaults` +
  `settings_path_reload` + `settings_storage_panel` (16 tests).
- Duplicate top-level helpers to hoist into `test/support/` first: `_SeededEngineUi`
  (4 files), `_track` (4 files).
- Global-state overrides that gate a merge: `FlutterError.onError` (6 files — all restore
  via `addTearDown`), `debugOverrideDesktopWindow` (5 files, all restored).
- Every merge must pass `--test-randomize-ordering-seed=random` twice before it counts.
