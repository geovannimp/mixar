# stems-live-separation — run record

> **Historical:** this record covers the earlier HTDemucs live path. The
> shipped Realtime Stems filter is StemgenRT on the CPU EP (XNNPACK measured
> within noise but is not selected) and is opt-in; see the stems overview and
> the Android section at the end for current behavior.

Branch `geovannimp/rust-optimization`, base `d85d3a99`, feature commit
`51eeb256`, docs `62c87989`. Crates: `analyzer-stems`, `audio-core`,
`engine-dsp`, `engine-core`, `host-flutter`.

Machine: 16-core x86-64, RTX 3060 Ti, Linux. Release mode. Model
`htdemucs_mixxx_v1-db37d131.onnx` (sha256 `db37d131…752d`). Audio: first 120 s
of `samples/Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at
57pct).wav` (44.1 kHz stereo). Runs are serial; WebGPU EP unless marked CPU.

Instrument: `crates/analyzer-stems/examples/bench_split.rs` (added this run).
`--dump` writes raw f32 stems plus a `sha256` file for byte parity;
`--live [--preload]` measures the producer instead of the offline split.

## Baseline (B0), before any code change

Commit `d85d3a99`, 2026-10-20, same host/model/audio.

| scenario | command | result |
|---|---|---|
| offline 120 s | `bench_split --seconds 120` | 22 834 ms, 5.255× RT, 21 chunks, chunk p50 940 ms |
| offline 30 s (cold) | `--seconds 30` | 9 851 ms, 3.045× RT (includes the 4–5 s session load) |
| offline 30 s CPU | `MIXAR_STEMS_ORT_EP=cpu --seconds 30` | 23 635 ms, 1.269× RT, chunk p50 3 271 ms |
| stem hashes 30 s | `--dump` | `499a7d70…`, `bf52feb2…`, `e2436947…`, `c989a623…` |
| live path | — | none; pads ready only after a full generate + mux (~57 s for a 5 min track at 5.3×) |

Baseline suite: `cargo test -p analyzer-stems` at `d85d3a99` — see Gates.

## Scoreboard vs B0

| scenario | B0 | final | relative |
|---|---|---|---|
| offline 120 s, cold | 22 834 ms / 5.255× | 22 372 ms / 5.364× | −2.0% |
| offline 120 s, warm | not measured | 18 933 ms / 6.338× | — |
| offline 90 s synthetic A/B, warm | 15 297 ms median / 5.89× | 14 964 ms median / 6.02× | −2.2% (noise) |
| chunk cadence | 940 ms p50 | 898 ms p50 | −4.5% (noise) |
| live first window, session preloaded | n/a | **861 ms** | new |
| live first window, cold session | n/a | 4 475 ms | new |
| live 120 s sustained | n/a | 18 711 ms / **6.413×** | new |
| live window cadence | n/a | 890 ms p50 | new |
| disk written per live track | full 5-stream `.stem.mp4` | **0 bytes** | −cache size |

Noise: the 90 s offline A/B alternates base and head commits in separate
worktrees. The head is 2.2% faster, inside the ±3% host band; the offline code
path is unchanged. The headline is the new live path.

## Parity and quality

- Offline output is byte-identical to B0: the 30 s dumps match all four
  `sha256` lines above after the final change.
- Live output is byte-identical to offline, frame by frame, for the initial
  epoch and after a mid-track seek on the offline chunk grid:
  `tests/live_split_matches_offline.rs` (2 ignored real-model tests) passes.
- Live stems are f32; the cached path is 160 kbps Opus per stream. Live is
  therefore equal (same model math) or better (no lossy encode) in quality.

## Kept changes

| commit | change | evidence |
|---|---|---|
| `51eeb256` | SPSC `LiveStemRing` (2^20 frames, frame-major); live producer; grid-aligned epochs + context chunk; deck live branch + 5 ms crossfade; engine attach/readiness event; host start + session preload + lifecycle stops; docs; benchmark; ignored parity tests | scoreboard, parity tests, clippy/fmt/tests |
| `62c87989` | docs wording: finalizer mirrors offline; post-seek verification | no behavior |

Offline also acquires the ORT session per chunk instead of per track so a live
producer can interleave. Output is unchanged (hashes above).

## Removed/replaced code

- No stem-cache code removed; the cache remains authoritative when present.
- `Deck::stems` changed from `Option<[Arc<LoadedAudio>; 4]>` to
  `DeckStems::{Buffers, Live}`; `mix_at_position` takes `&mut self` for the
  blend/coverage state.
- Feature diff: 22 files, +1617 / −52. Of that, `bench_split` is +364 (the
  instrument), the live producer/ring/parity tests are +713, and the docs
  commit is +12.

## Tried and rejected

| change | why | numbers |
|---|---|---|
| FP16 ONNX (`onnxconverter-common` / `onnxruntime.transformers`) | 3× faster per chunk, but the converted graph is invalid: Level3 load fails on fp16 `Range`/`Cast`; Level1 loads but emits NaN on real input | fp16 WebGPU chunk warm 308 ms vs fp32 908 ms; output NaN |
| dynamic batch (header-only `dim_param`) | WebGPU rejects batch>1 with a Softmax bind-group validation error; CPU batch scales linearly | CPU ORT 1/2/3 chunks: 2 955 / 6 350 / 9 271 ms |
| full-track RAM stems | 4 × 6 min f32 ≈ 2 GB per track, no seek support | arithmetic |
| per-chunk resample | resetting resampler state at every boundary injects edge transients | quality risk; whole-track resample kept |
| cache without mixdown (4 streams) | saves ~20% disk but still duplicates; superseded by live | not pursued |
| separate live/generate locks | per-chunk session acquisition is simpler and lets them interleave | offline unchanged in A/B |

## Decisions made alone

1. **That HTDemucs version was opportunistic**: it started only when the model
   artifact was already cached (no download) and a non-CPU EP was compiled.
   CPU-only builds skipped it so separation could not compete with audio
   rendering.
2. **That version started live for every uncached deck load** (no setting).
   Cached stems stayed authoritative; an explicit Generate still wrote a
   persistent cache.
3. **Ring = 2^20 frames (23.8 s, 32 MiB/deck)** with a two-stride lead. The
   producer derives its lead from the ring capacity, so a smaller ring cannot
   wrap onto the reader (`pacing_never_crosses_capacity`).
4. **Session preload at engine start** when the model is cached: first live
   window 4.5 s → 0.86 s for a 3.5 s one-time background load.
5. **Grid-aligned epochs** plus one context chunk after a seek. Costs ~1 s of
   extra separation after a seek; preserves bit-identity with the cached grid.
6. **5 ms crossfade**, main-mix fallback outside coverage. The four stems sum
   back to the mix, so the transition is nearly transparent.

## Review findings and fixes

- **Ring capacity vs lead (P0)**: `MIN_CAPACITY_FRAMES` allowed a lead plus
  window larger than the ring. Fixed with `max_ahead_frames(capacity)` and a
  unit test.
- **Post-seek parity (P1)**: a re-epoch at an arbitrary frame used a different
  chunk grid. Fixed by anchoring to `frame - frame % CHUNK_STRIDE` and
  computing the context chunk; the ignored seek test asserts equality.
- **Producer lifecycle (P0/P1)**: the previous separator is stopped before
  every successful load; `attach_stems` closes a live ring; unload and restart
  drop the host handle. No overlapping rings or orphan threads.
- **Per-sample cost (P1)**: frame-major ring (one or two cache lines per frame),
  hoisted `floor`, per-block coverage snapshot, `blend == 1` early-out.

## Unverified items

- Flutter UI was not run on hardware; `stems_ready` reaches it through the
  existing tick `Updated` event. Engine, ring, producer, and host paths are
  covered by tests.
- Two decks simultaneously live was not benchmarked (GPU serializes; 32 MiB per
  deck).
- macOS/Windows execution providers were not exercised.
- CPU live is intentionally gated off; CPU offline was not re-measured (path
  unchanged).
- Non-44.1 kHz live tracks hold a full 44.1 kHz resampled copy (~85 MB/4 min,
  ~212 MB/10 min). Most tracks are 44.1/48 kHz.

## Gates

- `cargo fmt --check`: clean.
- `cargo clippy -D warnings` for `audio-core`, `analyzer-stems`, `engine-dsp`,
  `engine-core`, `host_flutter` (all targets): clean.
- Tests: 22 audio-core, 28 analyzer-stems unit (+1 ignored smoke), 153
  engine-dsp, engine-core suites, 52 host-flutter; all pass.
- Real-model ignored tests: 2/2 live parity pass (initial epoch + post-seek).
- `pnpm --dir apps/website build`: 46 pages built.
- Baseline suite at `d85d3a99`: `cargo test -p analyzer-stems` → 28 passed,
  0 failed, 1 ignored (the model-download smoke).

## Reproduce

```bash
cd crates
cargo build --release -p analyzer-stems --example bench_split

# offline, 120 s, cold + warm in one process
LD_LIBRARY_PATH=target/release:target/release/examples \
  ./target/release/examples/bench_split \
  --audio "../samples/Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav" \
  --seconds 120 --models-root /tmp/mixar-stems-bench/models --repeat 2

# live, preloaded session
LD_LIBRARY_PATH=target/release:target/release/examples \
  ./target/release/examples/bench_split ... --live --preload

# parity (needs the model on disk)
MIXAR_STEMS_MODEL_DIR=/tmp/mixar-stems-bench/models MIXAR_STEMS_ORT_EP=webgpu \
  LD_LIBRARY_PATH=target/release \
  cargo test --release -p analyzer-stems --test live_split_matches_offline -- --ignored
```

## Android devices: StemgenRT execution providers (2026-11-06)

ONNX Runtime 1.31 from `com.microsoft.onnxruntime:onnxruntime-android` loaded
through `ort`'s `load-dynamic`. Instrument: `examples/live_ep_bench.rs`
(release; mirrors the live session's thread settings), 300 hops of
`stemgenrt.onnx` (sha256 `0a49388d…`), 10-hop warm-up, zeroed input. Budget per
hop is 128 / 44 100 = 2.90 ms.

| device | NNAPI | XNNPACK | CPU |
|---|---|---|---|
| unknown Galaxy (`RX2W6001FBF`) | 219.61 ms · 0.01× | 10.97 ms · 0.26× | 11.30 ms · 0.26× |
| Galaxy S23 (`SM-S911B`) | 64.8–71.4 ms · 0.04× | 3.2–3.9 ms · 0.74–0.91× | 3.2–3.7 ms · 0.78–0.90× |

Steady state on the S23 after repeated runs: CPU ≈ 3.6 ms (0.80×), XNNPACK
≈ 3.9 ms (0.74×). CPU thread sweep there (300 hops): 1 → 4.48 ms, 2 → 4.23,
3 → 4.05, 4 → 4.20, 6 → 4.23, 8 → 8.98; the live path's half-cores-clamped-to-4
is the right preset.

Probing every EP type against the Microsoft AAR (`live_ep_bench` prints each
one): `nnapi`, `xnnpack` and `cpu` are `available=true`; `acl`, `qnn`, `tvm`
and `azure` are `available=false` — that binary does not contain them. The
bench harness probes at runtime, so dropping in a custom ONNX Runtime build
makes those rows run without code changes. QNN needs the Qualcomm AI Engine
Direct SDK, TVM needs a built TVM runtime, and Azure needs an Azure ML endpoint;
ACL is the only one buildable from open sources alone.

NNAPI partitions almost nothing of this graph and pays per-hop copy cost, so it
is ~18–20× slower than CPU on both devices, and QNN ranges from 4× to 24×
slower; the live producer therefore uses CPU only. Android stem generation
(offline) is CPU-only too, by product decision: the same accelerators are the
only alternatives and none measured faster. A forced EP still works for
experiments via `MIXAR_STEMS_ORT_EP` or the `debug.mixar.stems_ep` system
property. Even at 0.8–0.98× the producer cannot sustain playback: the pacing
window is ~11.9 s, so a fully pre-buffered start covers roughly a minute of
playback before the deck falls back to the dry mix. The producer logs
`StemgenRT live: producer is slower than realtime ep=… avg_hop_ms=… realtime=…`
when that happens.

Reproduce:

```bash
cd crates
cargo build --release -p analyzer-stems --target aarch64-linux-android --example live_ep_bench
adb push target/aarch64-linux-android/release/examples/live_ep_bench /data/local/tmp/
adb push libonnxruntime.so stemgenrt.onnx /data/local/tmp/
adb shell ORT_DYLIB_PATH=/data/local/tmp/libonnxruntime.so \
  /data/local/tmp/live_ep_bench /data/local/tmp/stemgenrt.onnx 300
# optional thread override: ... /data/local/tmp/stemgenrt.onnx 300 4
```

## QNN (Qualcomm HTP) on the S23 (2026-11-07)

Built ONNX Runtime v1.31.0 for Android arm64 with
`--use_qnn=static_lib --qnn_home <QAIRT 2.51.0.260929> --use_xnnpack`, loaded
through `ort`'s `load-dynamic`. NNAPI had to be dropped from that build: its
provider archive and QNN's both define `NodeAttrHelper`, so the shared library
fails to link with duplicate symbols.

| EP | avg hop | realtime factor |
|---|---|---|
| QNN, default options | 71.46 ms | 0.04× |
| QNN, `MIXAR_QNN_PERF=burst` | 13.34 ms | 0.22× |
| QNN, fp16 + high performance | 42.43 ms | 0.07× |
| XNNPACK (same lib) | 3.17 ms | 0.91× |
| CPU (same lib) | 2.96–3.00 ms | 0.96–0.98× |

QNN/HTP is 4–24× slower than CPU here, depending on performance mode: the
streaming graph partitions poorly and per-hop dispatch dominates, so HTP is not
in the live cascade. Offline conv-heavy models (HTDemucs/SCNet) were not
measured on HTP. Runtime wiring: `LD_LIBRARY_PATH` and `ADSP_LIBRARY_PATH` must
contain the QNN libraries and `libQnnHtpV73Skel.so`; the app declares
`libcdsprpc.so` via `<uses-native-library>` and sets `ADSP_LIBRARY_PATH` to its
native library dir.

Reproduce:

```bash
# QAIRT 2.51 unpacked at /path/qairt/2.51.0.260929, ORT v1.31.0 checkout
./build.sh --android --config Release --android_abi arm64-v8a --android_api 29 \
  --build_shared_lib --parallel --skip_tests --cmake_generator Ninja \
  --android_ndk_path "$ANDROID_NDK" \
  --use_qnn=static_lib --qnn_home /path/qairt/2.51.0.260929 --use_xnnpack
adb push libonnxruntime.so libQnnHtp.so libQnnHtpPrepare.so libQnnSystem.so \
  libQnnHtpV73Stub.so libQnnHtpV73Skel.so stemgenrt.onnx live_ep_bench /data/local/tmp/
adb shell "cd /data/local/tmp && LD_LIBRARY_PATH=/data/local/tmp \
  ADSP_LIBRARY_PATH=/data/local/tmp ORT_DYLIB_PATH=/data/local/tmp/libonnxruntime.so \
  ./live_ep_bench ./stemgenrt.onnx 300"
```
