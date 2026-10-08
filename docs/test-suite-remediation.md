# Test-suite & CI remediation — completed

Source: audit of CI run `37398514009` (7m29s: `Run tests` 5m51s, of which 5m23s was
compiling/linking 283 crates and only **15s** was running 566 tests) plus local
benchmarks on a 16-core machine.

## Measured outcome

| Metric | Before | After |
|---|---|---|
| Rust test CPU time (all tests) | 33.70s | **12.16s** (−64%) |
| Rust wall, `cargo nextest -j16` | 2.045s | **0.806s** (−61%) |
| Rust wall, warm `cargo test --workspace` | 6.31s | **4.80s** |
| Rust tests | 566 (3 failing locally) | 563 passed, 0 failed, 1 ignored |
| `npm run test` (rust + flutter + website) | red locally | **green, 19.1s** |
| Slowest single test | 1.33s (full commercial track decode) | 0.49s |
| Tests stuck at the 200ms `Engine::start()` floor | 92 | **0** |

The `0.22s`/`0.24s`/`0.32s` clusters in the timing histogram (92 tests) are gone
entirely; 423 tests now complete in ~0.01s.

## Phase 1 — CI configuration

- [x] P1.1 `CARGO_INCREMENTAL=0` in test/build/lint workflows (the 1.6 GB
      `target/debug/incremental` tree no longer bloats the shared rust-cache entry).
      Kept out of `[profile.dev]` so local incremental builds stay fast.
- [x] P1.2 Investigated the rust-cache race. **No speculative workflow change
      made** — the exact cause needs the A/B below, and a wrong change makes cold
      builds worse. Evidence gathered from this PR's own runs:
      - one run had a partial restore (`full match: false`, 805 MB of a 16 GB
        target, different env hash) and its save was rejected with *"another job
        may be creating this cache"*;
      - another restored **nothing** (`Cache Rust` 0s);
      - when the cache did hit, `Run tests` took **103s**; with no cache, **359s**
        (same commit, same runner) — i.e. the ~5 minutes of variance is entirely
        cache hit rate, not the tests.
      Recommended experiment for the team: bump rust-cache `prefix-key` to
      `v1-rust` (so the three jobs stop reusing each other's feature-set-specific
      entries) and give `build.yml` (the `--all-targets --all-features` superset)
      `save-if` so one job owns the write. Measure cold vs warm on both sides.
- [x] P1.3/P1.4 Mapping validation, two parts:
      - `rust:test-mappings` had never run in CI (`moon run :test` only matches
        tasks named `test`, and no workflow runs `moon ci`).
      - The fix is `crates/controller/tests/map_check.rs`, which runs under the
        normal `test` task and covers the shipped bundles plus the empty-tree and
        typo'd-`script` cases. A separate `cargo run --bin map-check` CI step was
        dropped after measuring ~40s of extra linking for identical coverage.
      - moon rejects `..` in `inputs` and `mappings/` is outside this project
        root, so the bundles cannot be a cached input for the moon task; the task
        is kept for manual use.

## Phase 2 — Engine correctness

- [x] P2.1 **Null backend now actually renders.** Proven before the fix: seek to
      50ms, `Play`, wait 1000ms → `position_ms` still `Some(50)`;
      `NullStream::start` rendered exactly once and the producer parked on the
      pre-filled ring, so ~60 engine-core tests were asserting on their own seeks.
      `NullStream` now runs a per-stream paced driver thread (one callback per
      buffer duration, like a sound card), with `Drop` stopping it.
      `test_null_stream_audio_processing` now asserts frames are rendered and that
      `stop()` parks the driver.
- [x] P2.2 Removed the unconditional 200ms `PRODUCER_WARMUP_MS` sleep. The ring is
      pre-filled synchronously and the producer is parked on `filled >= target_fill`
      regardless, so the sleep was pure startup latency (and app startup latency).
- [x] P2.3 `library` worker `RECV_TIMEOUT` 100ms → 25ms: bounds the join every
      `LibraryTransport` drop pays, at negligible idle-poll cost.

## Phase 3 — Wrong / useless / dead tests

- [x] P3.1 `bus_sync_speed.rs`: now asserts the real 1.2× sync ratio (0.1 norm) and
      a derived 1.02× follow (0.46 norm) using `default_tempo_range = 0.25` instead
      of a saturated `≈ 0.0`. Deleting `apply_tempo_sync` now fails these tests.
- [x] P3.2 The three `backend: "auto"` tests no longer open a stream; they assert
      backend selection + device enumeration, which is deterministic.
- [x] P3.3 Deleted `controller/tests/smoke.rs` (no assertions) and
      `engine-api/tests/postcard_roundtrip.rs` (empty; postcard was retired).
- [x] P3.4 Deleted `engine-core/tests/bus_pause_long_wav.rs` — duplicate of
      `bus_pause_preserves_position.rs` that decoded a full commercial track
      (1.33s, the slowest test in the suite).
- [x] P3.5 Removed 7 tautological `assert_eq!(*event.kind(), …)` after a
      kind-filtered receive.
- [x] P3.6 `analyzer-stems`: replaced the `SOURCE_ORDER` self-assert with a real
      `STEM_NAMES` ↔ ONNX source-order consistency check.
- [x] P3.7 `codec`: repointed the silently-skipping opus test at the CI fixture and
      added a real decoder/metadata round-trip; deleted the struct-literal test.
- [x] P3.8 Deleted `test_ring_buffer_integration` (it tested `rtrb`, not Mixar).
- [x] P3.9 `bus_jog_position`: bounded position assertion with the fixture length
      instead of `position_ms >= 0`.
- [x] P3.10 `engine_trust`: replaced vacuously-true `all()` negatives with
      `assert!(ev.is_empty(), …)` and a selective-offer test. A positive
      `MappingAttached` is not reachable without a MIDI backend (proven by probe).
- [x] P3.11 `bus_sampler_trigger`: asserts the pad-mode routing of the press
      (`HotCue` when sampler mode was never set), pins the empty-slot error text.
- [x] P3.12 `bus_performance`: pinned the fixture length instead of comparing two
      fields of the same event; asserted the derived 500ms beat jump; made the
      unload cue-point assertion strict.
- [x] P3.13 `library` recorder: `played_duration >= 0` → `is_some()`, and dropped
      its 5ms sleep (the timestamp is second-granular, so an exact value is
      non-deterministic).
- [x] P3.14 `engine-dsp`: `test_dsp_engine_processing` now loads a tone and asserts
      a non-silent master bus / silent cue bus.
- [x] P3.15 `report_errors`: real `tracing` log capture; the production message was
      de-duplicated into `report_script_binding_failure` so the test exercises it.

## Phase 4 — Flakiness / hygiene

- [x] P4.1 Added `crates/engine-core/tests/common/mod.rs` with the one correct
      `recv_evt_kind`, and migrated 15 files off their drifted copies. In 10 of
      them `.expect("event")` panicked on the first `Ok(None)` (a 50ms timeout),
      making the 1–5s deadline dead code — a real flake source under CI load.
      Fixed the 3 remaining inline copies with the same bug.
- [x] P4.2 `session_analyze_bus`: 3s tone → 1s, 30s deadline → 10s.
- [x] P4.3 `integration_tests`: fixed `temp_dir()/test_config.toml` → `tempfile::tempdir()`.
- [x] P4.4 `analyzer-stems/ep.rs`: `with_force_ep` now restores the previous env
      value; the locale test takes `ENV_LOCK`, asserts `setlocale` succeeded, and
      restores the previous locale.
- [x] P4.5 `session_input`: replaced the 20ms sleep with
      `age_cc_coalesce_for_test` (deterministic backdating of the coalesce window).
- [x] P4.6 `engine_buses`: the `remaining.max(1ms)` / `.expect` timeout bug is gone
      (now uses the shared helper).
- [x] P4.7 Deleted the resampler strict-prefix duplicate + merged two ctor tests;
      deleted a duplicated `audio-core` channel-mapping test.
- [x] P4.8 `host-flutter` `fs_browser_browse` now tests the FRB DTO mapping and
      error passthrough instead of repeating the `fs-browser` unit test;
      `smoke_null_backend` tightened to the exact null device.
- [x] P4.9 Folded into P3.13.
- [x] `bus_save_hot_cue`: the drain loop's 20ms quiet window was below the 33ms
      control tick, so it could break mid-flight and latch a stale snapshot; widened
      to 150ms and matched on the trigger's effect rather than `Kind` alone.
- [x] New `controller/tests/map_check.rs`: shipped bundles validate, an empty root
      is an error (previously "0 bundle(s) ok" + exit 0), and a typo'd
      `script = "fn"` binding is rejected.

## Phase 5 — Verification (all run, all green)

| Check | Result |
|---|---|
| `cargo fmt -- --check` (via `moon run rust:format-check`) | pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| `cargo test --workspace` | 563 passed, 0 failed, 1 ignored |
| `flutter analyze --no-fatal-infos` | pass (222 pre-existing infos) |
| `flutter test` | 338 passed |
| `npm run test` (`moon run :test`) | 3/3 tasks pass, 19.1s |
| `moon run rust:test-mappings` | `ok ddj-400` |
| `map-check --all` on a bundle with a typo'd script fn | `ERR … not found in script.rhai`, exit 1 |

## Not done / deferred (with reasons)

- **CI cache key strategy** — see P1.2. Deliberately left as-is; needs a CI A/B.
- **Reducing the 46 giant test binaries** (~100–210 MB each; `host_flutter`'s six
  alone are ~1.1 GB) by merging integration-test files. This is the remaining
  structural lever on the 5m23s compile/link cost, but it is a larger refactor with
  real blast radius, so it is proposed rather than attempted here.
- **`flutter test`'s ~20s** is dominated by per-file kernel compilation (52 files ×
  ~1.9s ≈ 100s CPU), not test bodies. No low-risk change found.
