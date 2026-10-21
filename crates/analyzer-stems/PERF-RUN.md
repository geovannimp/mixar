# stems-live-separation — run record

> **Historical:** this record covers the earlier HTDemucs live path. The
> shipped Realtime Stems filter is StemgenRT on the CPU and is opt-in; see the
> stems overview for current behavior.

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
