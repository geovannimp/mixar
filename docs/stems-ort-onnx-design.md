# Stems inference — ORT + HTDemucs ONNX

**Date:** 2026-09-22  
**Status:** Approved (shipped path)  
**Supersedes:** Burn HTDemucs path in `analyzer-stems`  
**Related:** `docs/stems-storage-models-design.md`, `docs/stems-pad-mode-design.md`, `docs/ni-stem-mp4-cache-design.md`

## Goal

Run stem separation with **[pykeio/ort](https://github.com/pykeio/ort)** (ONNX Runtime) on a single-file HTDemucs ONNX. Prefer the most capable execution provider available, fall back through slower EPs to CPU. Mixar owns the model cache under app-support; separated PCM is muxed to NI `.stem.mp4` (see `docs/ni-stem-mp4-cache-design.md`).

## Why

Burn wgpu on a 3060 Ti was ~1× realtime — too slow for DJ workflow. StemSplit’s ORT-oriented ONNX exports are parity-verified and much faster on ORT ([export writeup](https://stemsplit.io/blog/htdemucs-ft-onnx-export), [htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx)). ORT also opens the door to more ONNX models later. `burn-onnx` → Burn was rejected: high import risk, same Burn runtime bottleneck.

## Non-goals (v1)

- Keep Burn / burn-onnx / safetensors HTDemucs graph
- FT bag (`htdemucs_ft` ×4), `mdx_extra_q`, settings model picker
- Hard-require CUDA / TensorRT / toolkit installs
- Reintroduce `stem-splitter-core`
- Bit-identical output vs Burn or vs PyTorch Demucs

## Execution-provider cascade

Register EPs in preference order; failed register / unsupported ops → next; always end on **CPU**. Log the EP that actually ran. Optional force: `MIXAR_STEMS_ORT_EP=<name>`.

**Linux:** TensorRT RTX → TensorRT → CUDA → WebGPU → (OpenVINO if enabled on Intel builds) → CPU  

**Windows:** TensorRT RTX → TensorRT → CUDA → DirectML → WebGPU → CPU  

**macOS:** CoreML → WebGPU → CPU (Apple Silicon only)  

Notes:

- macOS ships arm64-only: `ort-sys` prebuilts exist for `aarch64-apple-darwin` alone — there is no `x86_64-apple-darwin` dist, not even CPU-only — so the x86_64 slice of a universal build cannot link ORT. `ARCHS = arm64` is pinned in `apps/gui-flutter/macos/Runner/Configs/AppInfo.xcconfig` and in the `host_flutter` podspec (cargokit reads `$ARCHS`); both must stay in sync.
- WebGPU EP is experimental upstream — best-effort, never hard-fail the app.
- CUDA/TensorRT only when libs are present; no install gate for launching Mixar.
- CI / headless: CPU EP only (Cargo features).

See [ort execution providers](https://ort.pyke.io/perf/execution-providers).

## Architecture

Lives in **`crates/analyzer-stems`**: resolve/download ONNX → process-global `ort::Session` → chunked overlap-add infer → hand interleaved PCM to `codec` for `.stem.mp4` mux.

### Public API (stable shape)

`StemSplitRequest`, `StemSplitResult`, `STEM_NAMES`, `ensure_model`, `resolve_model`, `split_interleaved_stereo`.

- Default model id follows the Mixxx / StemSplit artifact Mixar pins (`htdemucs_mixxx_v1` / equivalent ORT artifact).
- `StemSplitResult.backend` → `{model}/{ep}` (e.g. `htdemucs_mixxx_v1/cuda`).

Library cache match uses model-id-before-`/` so EP changes do not force regen.

## Model weights

- **Artifact:** HTDemucs ONNX under Mixar `{app_support}/models/` (download + sha verify via manifest).
- **I/O:** stereo mix in → four stereo stems out in NI order: **drums, bass, other, vocals** (same as engine / `.stem.mp4` streams 1–4).
- Old Burn / SSC backend ids miss on model id and regenerate.

## Inference pipeline

1. Resample interleaved stereo → model rate (typically 44.1 kHz).
2. Chunk with overlap / triangular fade (Demucs apply_model math; segment length per model).
3. For each chunk: ORT run → accumulate stems → progress `(done, total)`.
4. Library / codec muxes mixdown + four stems into `{stems_root}/{fnv64(track_id)}.stem.mp4`.

## Settings / UI (v1)

No model picker. Storage clear models still wipes `models/`. Progress phases: see `docs/stems-analyze-progress-design.md`.

## Testing

| Test | Purpose |
|------|---------|
| EP list unit tests | Force env; platform preference order without GPU |
| Manifest path + sha reuse | Existing patterns |
| CI | `cargo test -p analyzer-stems` CPU-only |

## Ship criteria

1. Analyze + stems produces a `.stem.mp4`; log shows chosen EP.
2. No CUDA toolkit required to run (CPU or WebGPU path works).
3. Progress advances per chunk; no silent 0% hang without logs.
4. Clear models → re-download ONNX.
5. No `burn` crate in the stems dependency tree.

## Follow-ups (out of v1)

- Settings model picker (FT specialists, MDX, etc.)
- OpenVINO packaging for Intel Linux
- TensorRT RTX packaging validation on shipping GPUs
