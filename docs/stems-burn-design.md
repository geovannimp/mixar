# Stems inference — Burn HTDemucs (replace stem-splitter-core)

Date: 2026-09-21  
Status: proposed  
Replaces: ONNX / `stem-splitter-core` path in `analyzer-stems`

## Goal

Replace `stem-splitter-core` + ONNX Runtime with a **Mixar-owned Burn implementation** of HTDemucs v4 (4-stem). GPU via **wgpu** (Vulkan on Linux) so users need only a normal graphics driver — no CUDA toolkit install. Keep the existing `analyzer-stems` public API and Opus/FLAC cache pipeline.

## Non-goals (v1)

- 6-stem / fine-tuned Demucs variants
- WebAssembly stems
- ONNX import or runtime fallback
- Realtime / deck-path separation (offline ensure only, as today)
- Matching stem-splitter-core ONNX output bit-for-bit

## Why rewrite (not vendor demucs-rs)

Product choice: own the code in `analyzer-stems` for licensing boundaries, manifest/cache integration, and sync API fit. Use public HTDemucs architecture + Hugging Face safetensors weights as reference, not as a copied dependency.

## Current vs target

| Layer | Today | Target |
|-------|--------|--------|
| Inference | `stem-splitter-core` → ORT CUDA/CPU | Burn `HTDemucs<B>` |
| Weights | `htdemucs_ort_v1` ONNX (~155 MB) | `htdemucs.safetensors` (~84 MB) |
| Model cache | `{app_support}/models/` (Mixar manifest) | Same root; new manifest + `.safetensors` |
| Stem cache key | `backend = htdemucs_ort_v1` | `backend = htdemucs_burn_v1` |
| Progress | Window hop loop | Chunk overlap loop (see below) |
| GPU EP | ORT CUDA + `ort_ep` hacks | Burn wgpu (CubeCL → Vulkan) |

Old ONNX caches miss on backend change and regenerate (existing behavior).

## Architecture

Stay in **`crates/analyzer-stems`** (no new workspace member). Module layout:

```text
analyzer-stems/src/
  lib.rs           # public re-exports (unchanged surface)
  split.rs         # ensure_model, split_interleaved_stereo (rewired)
  format.rs        # Opus/FLAC encode (unchanged)
  manifest.rs      # Mixar JSON manifest, sha256 verify, download
  backend.rs       # Burn backend selection + shared Demucs session
  dsp/             # STFT/iSTFT, CaC (realfft); rate conversion via workspace `resampler`
  model/           # HTDemucs Burn modules + forward
  weights/         # safetensors → Param load
  infer.rs         # chunk loop, progress, stem buffers
```

Remove: `ort_ep.rs`, `stem-splitter-core` dependency.

### Public API (unchanged)

Keep:

- `StemSplitRequest`, `StemSplitResult`, `STEM_NAMES`
- `ensure_model`, `resolve_model`, `split_interleaved_stereo`
- `StemAudioFormat`, `encode_stem_file`
- `DEFAULT_MODEL` → **`htdemucs_burn_v1`**

`StemSplitResult.backend` reports model id + compute backend, e.g. `htdemucs_burn_v1/wgpu` or `.../ndarray`.

### Threading / lock

Keep library `demucs_lock()` — one global Burn session (model + device) per process, same as today’s ORT session.

## Model weights

- **Artifact:** `htdemucs.safetensors` (standard 4-stem HTDemucs).
- **Source:** Hugging Face mirror used by demucs-rs ecosystem:  
  `https://huggingface.co/set-soft/audio_separation/resolve/main/Demucs/htdemucs.safetensors`
- **Mixar manifest:** JSON beside existing pattern (`name`, `sample_rate`, `artifacts[]` with `file`, `sha256`, `size_bytes`, `url`). Drop ONNX-specific fields; add `format: "safetensors"`.
- **Path layout:** reuse `artifact_path(models_root, …)` safety (no traversal).
- **Verify:** SHA-256 before load (reuse test patterns from `split.rs`).

## Inference pipeline

HTDemucs constants (match training):

- Sample rate: **44100 Hz** — resample input with workspace `crates/resampler` (`RubatoResampler`), same as Opus encode; delete the linear shortcut in `split.rs`
- `N_FFT = 4096`, `HOP = 1024`
- Segment length: **343980** samples (~7.8 s)
- Long audio: **75% stride overlap** + triangular window + normalize weights (not the current fixed hop window loop)

Per chunk:

1. Resample stereo interleaved → 44100 if needed  
2. Pad segment to 343980  
3. STFT L/R → CaC → tensors `[1,4,F,T]` freq + `[1,2,T]` time  
4. `HTDemucs.forward`  
5. Bulk readback freq/time outputs → iSTFT + time branch → 4 stems (L/R per stem)  
6. Overlap-add into output buffers  
7. `encode_stem_file` × 4 (Opus/FLAC unchanged)

Progress callback: `(chunks_done, chunks_total)` mapped to existing `on_window_progress`.

## Burn backends

Cargo features on `analyzer-stems`:

| Feature | Default | Use |
|---------|---------|-----|
| `burn-ndarray` | on | CI, headless, no GPU |
| `burn-wgpu` | on | Desktop GPU (Vulkan) |

Runtime order (env override optional later):

1. Try **wgpu** if feature enabled and adapter available  
2. Fall back to **ndarray** (multi-threaded CPU)

Drop `ort`, `libloading` CUDA probe, `STEMMER_*` env handling.

Implementation note: wgpu forward uses async tensor readback; **`pollster::block_on`** inside sync `split_interleaved_stereo` (same pattern as demucs-rs CLI). Ndarray path stays fully sync.

Warmup: one dummy forward after load on wgpu to compile shaders / autotune (optional env `MIXAR_STEMS_SKIP_WARMUP=1` for tests).

## Cache / DB

- `library::stems::expected_backend()` follows `DEFAULT_MODEL` → invalidates old `htdemucs_ort_v1` rows automatically.
- Storage settings “clear models” continues to wipe `{app_support}/models/`.

## Testing

| Test | Purpose |
|------|---------|
| `manifest` path safety + sha reuse | No regression from split.rs tests |
| `weights` tiny synthetic safetensors | Loader maps names → Burn params |
| `dsp` STFT roundtrip / CaC shape | No model |
| `infer` ndarray + fixture weights | End-to-end short sine → 4 buffers (not golden audio) |
| `#[ignore]` smoke | Full HF weights + real WAV when `MIXAR_STEMS_SMOKE=1` |

CI runs ndarray only (no GPU required).

## Phased delivery

1. **Scaffold** — deps, manifest download, remove stem-splitter-core; ndarray stub forward (fail clearly until model wired)  
2. **Model + weights** — HTDemucs struct + safetensors load  
3. **DSP + infer** — chunk loop, progress, encode; ndarray backend green in CI  
4. **wgpu** — device init, warmup, desktop GPU path  
5. **Docs** — update `docs/stems-pad-mode-design.md`, drop ORT/CUDA notes  

## Risks

- **Effort:** Full HTDemucs in Burn is ~2–4k LOC; largest Mixar analyzer change to date.  
- **Quality:** Must validate by ear on a few tracks vs current ONNX caches.  
- **Perf:** wgpu faster than CPU but likely slower than CUDA ORT on NVIDIA; acceptable trade for zero CUDA install.  
- **Compile time / binary size:** Burn + wgpu increases `host-flutter` link weight; monitor release size.

## Verification (ship criteria)

- Analyze + stems ensure produces four Opus/FLAC files on Linux with wgpu (3060 Ti class GPU).  
- CI green with ndarray backend only.  
- No `stem-splitter-core` / `ort` in workspace dependency tree.  
- Missing model downloads once; checksum failure surfaces as library error toast.  
