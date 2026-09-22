# Burn HTDemucs Stems Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `stem-splitter-core`/ORT with a Mixar-owned Burn HTDemucs (4-stem) so Linux GPU stems run via wgpu/Vulkan without a user CUDA install.

**Architecture:** Keep `analyzer-stems` public API. New modules: Mixar safetensors manifest+download, Burn HTDemucs model+weights, STFT/CaC DSP, chunked overlap-add infer. Workspace `resampler` for 44100 Hz prep. Runtime: try wgpu, fall back to ndarray. Cache backend id becomes `htdemucs_burn_v1`.

**Tech Stack:** Burn 0.20 (ndarray + wgpu features), safetensors, realfft, reqwest (blocking), sha2, pollster, existing `resampler`/`ruopus`/`flacenc`.

## Global Constraints

- Public API surface of `analyzer-stems` stays: `StemSplitRequest`, `StemSplitResult`, `ensure_model`, `split_interleaved_stereo`, `STEM_NAMES`, `StemAudioFormat`.
- `DEFAULT_MODEL = "htdemucs_burn_v1"`.
- Resample Demucs input with `crates/resampler` `RubatoResampler` only — no linear shortcut, no Burn-local resampler.
- No `stem-splitter-core`, `ort`, or `libloading` CUDA probe after Task 1.
- CI must pass with ndarray only (no GPU required).
- License: Mixar GPL-3.0 — do not copy demucs-rs source; reimplement from public HTDemucs architecture + HF weights.
- Weights URL: `https://huggingface.co/set-soft/audio_separation/resolve/main/Demucs/htdemucs.safetensors`.

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `crates/analyzer-stems/Cargo.toml` | Deps + `burn-ndarray` / `burn-wgpu` features |
| `crates/analyzer-stems/src/lib.rs` | Re-exports; drop `ort_ep` |
| `crates/analyzer-stems/src/manifest.rs` | JSON manifest, path safety, sha256, download |
| `crates/analyzer-stems/src/resample_pcm.rs` | Interleaved stereo → target rate via `RubatoResampler` |
| `crates/analyzer-stems/src/dsp/mod.rs` | STFT/iSTFT, CaC helpers |
| `crates/analyzer-stems/src/model/mod.rs` | HTDemucs Burn modules |
| `crates/analyzer-stems/src/weights/mod.rs` | safetensors → Burn `Param` |
| `crates/analyzer-stems/src/backend.rs` | Device pick + process-global session |
| `crates/analyzer-stems/src/infer.rs` | Chunk loop + progress + stem buffers |
| `crates/analyzer-stems/src/split.rs` | Wire ensure/split to Burn path |
| `crates/analyzer-stems/src/format.rs` | Unchanged Opus/FLAC (already uses resampler) |
| Delete `ort_ep.rs` | — |
| `docs/stems-pad-mode-design.md` | Point at Burn, not ORT |

---

### Task 1: Manifest download without stem-splitter-core

**Files:**
- Create: `crates/analyzer-stems/src/manifest.rs`
- Create: `crates/analyzer-stems/src/resample_pcm.rs`
- Modify: `crates/analyzer-stems/Cargo.toml`
- Modify: `crates/analyzer-stems/src/lib.rs`
- Modify: `crates/analyzer-stems/src/split.rs` (cut SSC imports; keep encode + tests that move)
- Delete: `crates/analyzer-stems/src/ort_ep.rs`
- Test: unit tests in `manifest.rs` and `resample_pcm.rs`

**Interfaces:**
- Produces:
  - `pub const DEFAULT_MODEL: &str = "htdemucs_burn_v1";`
  - `pub struct ModelManifest { name, sample_rate, artifacts: Vec<Artifact>, … }`
  - `pub struct ModelHandle { pub manifest: ModelManifest, pub local_path: PathBuf }`
  - `pub fn ensure_from_manifest(models_root: &Path, manifest: &ModelManifest) -> Result<ModelHandle>`
  - `pub fn builtin_manifest(model_name: &str) -> Result<ModelManifest>` — embeds URL/sha for `htdemucs_burn_v1`
  - `pub fn resample_interleaved_stereo(pcm: &[f32], from_hz: u32, to_hz: u32) -> Result<Vec<f32>>`
- Consumes: `resampler::RubatoResampler`, `reqwest` blocking, `sha2`

- [ ] **Step 1: Update Cargo.toml**

Remove `stem-splitter-core`, `libloading`. Add:

```toml
[features]
default = ["burn-ndarray", "burn-wgpu"]
burn-ndarray = []
burn-wgpu = []

[dependencies]
anyhow.workspace = true
flacenc = "0.5"
resampler = { path = "../resampler" }
ruopus = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls"] }
sha2 = "0.10"
hex = "0.4"
# Burn lands in Task 3; keep Cargo compiling without it until then — optional stub:
# (add burn in Task 3)
```

For Task 1 only, `split_interleaved_stereo` may return `Err(anyhow!("Burn HTDemucs not wired yet"))` after ensure+resample so the crate still builds.

- [ ] **Step 2: Write failing manifest path-safety test**

Move / rewrite tests from current `split.rs` into `manifest.rs`:

```rust
#[test]
fn artifact_path_rejects_traversal() {
    let root = Path::new("/tmp/mixar-models");
    let m = ModelManifest {
        name: "../escape".into(),
        sample_rate: 44_100,
        artifacts: vec![],
    };
    assert!(artifact_path(root, &m, "m.safetensors", "abcdef01").is_err());
}
```

- [ ] **Step 3: Implement `manifest.rs`**

- `builtin_manifest("htdemucs_burn_v1")` returns fixed artifact:
  - file: `htdemucs.safetensors`
  - url: HF URL above
  - sha256: fetch once while implementing and pin the real hash (document in comment)
- `ensure_from_manifest`: create dir, skip download if sha matches, else download + verify
- `resolve_model(models_root, name)` → `builtin_manifest` + `ensure_from_manifest`
- Path safety helpers identical to current `safe_path_component` / `safe_sha_prefix`

- [ ] **Step 4: Implement `resample_pcm.rs` using workspace resampler**

Mirror `format.rs` `resample_to_opus_rate` loop, generalized:

```rust
pub fn resample_interleaved_stereo(pcm: &[f32], from_hz: u32, to_hz: u32) -> Result<Vec<f32>> {
    if from_hz == to_hz {
        return Ok(pcm.to_vec());
    }
    let chunk = 512usize;
    let mut resampler = RubatoResampler::new(from_hz, to_hz, 2, chunk, "high")?;
    // same padded process loop as format.rs
}
```

Test: 4800 frames @ 48k → ~4410 frames @ 44.1k (allow ±2 frames).

- [ ] **Step 5: Wire lib + stub split; delete ort_ep**

- `lib.rs`: `mod manifest; mod resample_pcm;` remove `ort_ep`
- `split.rs`: use `manifest::resolve_model`; call `resample_interleaved_stereo`; then `bail!("Burn HTDemucs not wired yet")` until Task 4
- `ensure_model`: resolve + verify file exists (no ORT preload)

- [ ] **Step 6: Run tests**

```bash
cargo test --manifest-path crates/Cargo.toml -p analyzer-stems --lib
```

Expected: PASS for manifest + resample; no stem-splitter in tree (`cargo tree -p analyzer-stems | rg stem-splitter` empty).

- [ ] **Step 7: Commit**

```bash
git add crates/analyzer-stems crates/Cargo.lock docs/stems-burn-design.md
git commit -m "$(cat <<'EOF'
refactor(stems): drop stem-splitter-core for Mixar manifest download

Own safetensors manifest/path safety and rubato PCM resample ahead of
Burn HTDemucs inference.
EOF
)"
```

---

### Task 2: DSP — STFT / CaC (no Burn yet)

**Files:**
- Create: `crates/analyzer-stems/src/dsp/mod.rs`
- Create: `crates/analyzer-stems/src/dsp/stft.rs`
- Create: `crates/analyzer-stems/src/dsp/cac.rs`
- Modify: `crates/analyzer-stems/Cargo.toml` — add `realfft = "3.5"`, `num-complex = "0.4"`
- Modify: `crates/analyzer-stems/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct Stft { … }` with `fn forward(&mut self, wave: &[f32]) -> Result<Vec<Complex32>>` and `fn inverse(&mut self, spec: &[Complex32], out_len: usize) -> Result<Vec<f32>>`
  - Constants: `N_FFT = 4096`, `HOP_LENGTH = 1024`
  - `pub fn stft_to_cac_planar(left_spec, right_spec, n_fft) -> Vec<f32>` layout matching HTDemucs `[4, F, T]` channel-as-complex packing
  - `pub fn cac_planar_to_complex(slice: &[f32], bins, frames) -> Vec<Complex32>`

- [ ] **Step 1: Write STFT round-trip test**

```rust
#[test]
fn stft_istft_recovers_sine_roughly() {
    let sr = 44_100;
    let n = 8192;
    let wave: Vec<f32> = (0..n)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
        .collect();
    let mut stft = Stft::new(N_FFT, HOP_LENGTH);
    let spec = stft.forward(&wave).unwrap();
    let back = stft.inverse(&spec, n).unwrap();
    let err: f32 = wave.iter().zip(back.iter()).map(|(a, b)| (a - b).abs()).sum::<f32>() / n as f32;
    assert!(err < 0.05, "mean abs err {err}");
}
```

- [ ] **Step 2: Implement STFT/iSTFT with realfft + Hann window**

- [ ] **Step 3: CaC pack/unpack unit test (shape + invertibility on synthetic)**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(stems): add STFT and CaC DSP for HTDemucs"
```

---

### Task 3: Burn HTDemucs model + safetensors load (ndarray)

**Files:**
- Create: `crates/analyzer-stems/src/model/` (`mod.rs`, `htdemucs.rs`, `conv.rs`, `transformer.rs` as needed)
- Create: `crates/analyzer-stems/src/weights/` (`mod.rs`, `tensor_store.rs`, `load.rs`)
- Modify: `Cargo.toml` — add Burn:

```toml
burn = { version = "0.20", default-features = false, features = ["std"] }
# feature gates:
# burn-ndarray → burn/ndarray + burn/simd
# burn-wgpu → burn/wgpu
safetensors = "0.5"
half = "2"
```

Wire features so `default = ["burn-ndarray", "burn-wgpu"]` enables the matching Burn features.

**Interfaces:**
- Produces:
  - `pub struct HTDemucs<B: Backend> { … }`
  - `impl HTDemucs<B> { pub fn forward(&self, freq: Tensor<B,4>, time: Tensor<B,3>) -> (Tensor<B,4>, Tensor<B,3>); }`
  - `pub fn load_htdemucs_from_safetensors<B: Backend>(bytes: &[u8], device: &B::Device) -> Result<HTDemucs<B>>`
- Architecture constants (must match training / HF weights):
  - `CHANNELS=48`, `GROWTH=2`, `DEPTH=4`, `KERNEL_SIZE=8`, `STRIDE=4`, `T_LAYERS=5`, `T_HEADS=8`, `SAMPLE_RATE=44100`, `TRAINING_LENGTH=343980`

- [ ] **Step 1: Tiny safetensors round-trip test for `TensorStore`**

Build in-memory safetensors with one f32 tensor; `take` returns correct shape/data.

- [ ] **Step 2: Implement TensorStore + weight name mapping**

Map published HTDemucs safetensors keys (signature prefix `955717e8.…`) into Burn `Param`s. Prefer implementing loaders module-by-module (Conv1d/2d, GroupNorm, Linear, encoder/decoder, transformer) with unit tests using synthetic tensors where possible.

- [ ] **Step 3: Implement HTDemucs forward graph**

Hybrid encoder/decoder + cross-domain transformer matching public HTDemucs v4 structure. Keep files under ~400–600 lines each; split when larger.

- [ ] **Step 4: Load real weights smoke (ignored)**

```rust
#[test]
#[ignore]
fn loads_htdemucs_safetensors_from_models_root() {
    // env MIXAR_MODELS_ROOT or download via ensure_from_manifest into tempdir
}
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(stems): Burn HTDemucs model and safetensors loader"
```

---

### Task 4: Infer loop + wire `split_interleaved_stereo` (ndarray)

**Files:**
- Create: `crates/analyzer-stems/src/backend.rs`
- Create: `crates/analyzer-stems/src/infer.rs`
- Modify: `crates/analyzer-stems/src/split.rs`
- Modify: `crates/library/src/stems.rs` tests that hardcode `htdemucs_ort_v1` → `htdemucs_burn_v1`

**Interfaces:**
- Produces:
  - `pub fn separate_interleaved(pcm: &[f32], sample_rate: u32, on_progress: Option<&dyn Fn(usize,usize)>) -> Result<[Vec<f32>; 4]>` — interleaved L/R per stem in STEM_NAMES order
  - Process-global session behind `OnceLock` / mutex for ndarray (and later wgpu)
- Chunking: `TRAINING_LENGTH`, stride `TRAINING_LENGTH * 3 / 4`, triangular window, normalize by weight

- [ ] **Step 1: Write ndarray infer test with short sine + real weights (ignore if no network)**

Or: inject a tiny fake model behind a `cfg(test)` trait for CI without weights; keep `#[ignore]` full-weight test.

- [ ] **Step 2: Implement `infer.rs`**

1. `resample_interleaved_stereo` → 44100  
2. Chunk / pad / STFT / CaC  
3. `HTDemucs::forward`  
4. iSTFT + time branch → stems  
5. Overlap-add  
6. Progress `(done, total)`

- [ ] **Step 3: Implement `backend.rs` ndarray session**

```rust
pub enum ComputeBackend { NdArray, Wgpu }
pub fn select_backend() -> ComputeBackend; // Task 5 adds wgpu attempt
```

For Task 4: always NdArray.

- [ ] **Step 4: Wire `split_interleaved_stereo`**

Remove stub error. Call ensure → infer → `encode_stem_file` × 4.  
`StemSplitResult.backend = format!("{DEFAULT_MODEL}/ndarray")` (or `/wgpu` later).

- [ ] **Step 5: Update library tests expecting old backend id**

- [ ] **Step 6: Run**

```bash
cargo test --manifest-path crates/Cargo.toml -p analyzer-stems --lib
cargo test --manifest-path crates/Cargo.toml -p library --features analysis --lib stems
```

- [ ] **Step 7: Commit**

```bash
git commit -m "feat(stems): Burn ndarray HTDemucs infer and Opus/FLAC encode path"
```

---

### Task 5: wgpu backend + warmup

**Files:**
- Modify: `crates/analyzer-stems/src/backend.rs`
- Modify: `crates/analyzer-stems/Cargo.toml` (pollster)
- Modify: `docs/stems-pad-mode-design.md`, `docs/stems-storage-models-design.md`

**Interfaces:**
- `select_backend()`: if `burn-wgpu` feature and adapter init succeeds → Wgpu, else NdArray
- `MIXAR_STEMS_FORCE_CPU=1` forces ndarray
- `MIXAR_STEMS_SKIP_WARMUP=1` skips dummy forward

- [ ] **Step 1: wgpu device init with graceful failure → ndarray**

- [ ] **Step 2: `pollster::block_on` for async tensor readback on wgpu path**

- [ ] **Step 3: Warmup after first load**

- [ ] **Step 4: Docs — inference row = Burn HTDemucs; remove ORT/CUDA user notes**

- [ ] **Step 5: Manual verify on 3060 Ti**

Run Mixar analyze with stems on; confirm log shows `htdemucs_burn_v1/wgpu` and four stem files in Storage.

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(stems): enable Burn wgpu HTDemucs with CPU fallback"
```

---

### Task 6: Cleanup + CI sanity

**Files:**
- Grep workspace for `stem-splitter`, `ort_ep`, `STEMMER_`, `htdemucs_ort`
- Modify: `crates/analyzer-stems/tests/split_pcm_smoke.rs` for Burn + `#[ignore]`

- [x] **Step 1: Remove dead references; update smoke test**

- [x] **Step 2: `cargo tree -p host_flutter | rg 'stem-splitter|onnxruntime|ort '` must be empty**

- [x] **Step 3: Commit + open PR against main**

```bash
git commit -m "chore(stems): finish Burn migration cleanup"
```

---

## Spec coverage (self-review)

| Spec item | Task |
|-----------|------|
| Drop stem-splitter / ort_ep | 1 |
| Mixar models_root + safetensors manifest | 1 |
| Workspace resampler for 44100 | 1 (+ format already) |
| STFT/CaC | 2 |
| Burn HTDemucs + weights | 3 |
| Chunk overlap infer + encode | 4 |
| DEFAULT_MODEL / cache invalidation | 4 |
| wgpu + ndarray fallback | 5 |
| Docs | 5–6 |
| CI ndarray | 4, 6 |

## Notes for implementers

- demucs-rs is a **behavior reference** (constants, chunk math, weight key layout), not a source to paste.
- Prefer shipping Task 4 ndarray end-to-end before polishing every transformer micro-op — but weights must load and forward without NaNs on a short sine.
- If Burn 0.20 API drifts, pin the exact crate versions that compile on Linux CI and document in Cargo.toml comments.
