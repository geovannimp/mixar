# Stems Pad Mode Implementation Plan

> **For agentic workers:** Execute task-by-task. Steps use checkbox syntax.

**Goal:** Settings-gated offline HTDemucs stems → app-support cache → non-blocking deck attach → Stems pad mute/isolate.

**Architecture:** New `analyzer-stems` wraps stem-splitter-core windowed Demucs on Mixar PCM; `library` persists + ensures; engine mixes four synced buffers; Flutter settings + pads. charon-audio is reference-only (stub inference).

**Tech Stack:** Rust workspace, stem-splitter-core, sea-orm-sync, engine bus/FRB, Flutter pads.

## Global Constraints

- Do not depend on `charon-audio` for inference.
- `engine-dsp` remains pure (no I/O / no stem-splitter).
- Never block deck load on stem generation.
- Default `stems_enabled = false`.
- Four stems: vocals, drums, bass, other.
- Cargo via `cargo --manifest-path crates/Cargo.toml`.

## File map

| Path | Role |
|------|------|
| `crates/analyzer-stems/` | PCM split + write WAVs |
| `crates/library/src/entity/track_stem.rs` | Schema |
| `crates/library/src/stems.rs` | ensure / get / has |
| `crates/engine-api` PadMode + cmds/evts | Stems mode + stem state |
| `crates/engine-dsp/src/deck.rs` | Multi-buffer mix |
| `crates/engine-core` | attach stems, pad handlers |
| `crates/host-flutter` + Flutter | Settings + UI |

---

### Task 1: `analyzer-stems` crate

**Files:**
- Create: `crates/analyzer-stems/Cargo.toml`
- Create: `crates/analyzer-stems/src/lib.rs`
- Create: `crates/analyzer-stems/src/split.rs`
- Create: `crates/analyzer-stems/tests/split_pcm_smoke.rs` (optional if model heavy — prefer unit test of planar window helper)
- Modify: `crates/Cargo.toml` (workspace member)

**Produces:**
```rust
pub const STEM_NAMES: [&str; 4] = ["vocals", "drums", "bass", "other"];
pub struct StemSplitRequest<'a> {
    pub interleaved_stereo: &'a [f32],
    pub sample_rate: u32,
    pub output_dir: &'a Path,
    pub model_name: &'a str, // default "htdemucs_ort_v1"
}
pub struct StemSplitResult {
    pub paths: [PathBuf; 4], // vocals, drums, bass, other
    pub sample_rate: u32,
    pub backend: String,
}
pub fn ensure_model(model_name: &str) -> Result<()>;
pub fn split_interleaved_stereo(req: StemSplitRequest<'_>) -> Result<StemSplitResult>;
```

- [ ] Add workspace member and crate depending on `stem-splitter-core`, `anyhow`, `hound` (or stem-splitter writers), `thiserror`.
- [ ] Implement `split_interleaved_stereo`: `ensure_model` + `engine::preload` + window/hop loop mirroring stem-splitter `splitter.rs` but feeding Mixar interleaved PCM (resample to 44100 if needed via existing `rubato` / simple path).
- [ ] Write WAV stems with fixed names under `output_dir`.
- [ ] Unit-test window fill helper with a tiny buffer (no model download in CI unit test). Mark integration test `#[ignore]` if it needs the ~200MB model.
- [ ] Commit.

---

### Task 2: Library `track_stem` + ensure

**Files:**
- Create: `crates/library/src/entity/track_stem.rs`
- Create: `crates/library/src/stems.rs`
- Modify: entity mod, `db.rs` registry, `lib.rs`, `worker.rs` / analyze path
- Modify: library-api bus kinds if needed for `StemsReady` / progress

**Produces:**
- `LibraryManager::has_track_stems`, `get_track_stems`, `ensure_track_stems(library, id, stems_root, enabled)`
- When `enabled` and missing: decode via existing cache → `analyzer_stems::split_interleaved_stereo` → upsert row
- Stem root: caller passes `{app_support}/stems`

- [ ] Entity + sync.
- [ ] Wire analyze_track (end) and a public ensure used by host on load.
- [ ] Tests with `stems_enabled` false skip; with mock or ignored real split.
- [ ] Commit.

---

### Task 3: Engine stem playback + PadMode::Stems

**Files:**
- Modify: `crates/engine-api` PadMode, DeckSnapshot, CmdBody/EvtBody as needed
- Modify: `crates/engine-dsp/src/deck.rs` — optional `[Arc<LoadedAudio>; 4]`, gains, mix in `play_interpolated` / stretch paths
- Modify: `crates/engine-core` — `attach_deck_stems`, `stems_pad_press`, control routing
- Modify: `crates/controller` pad_mode validation
- Tests: `engine-dsp` mute/isolate sum; `engine-core` bus pad mode

**Produces:**
- `PadMode::Stems`
- Deck fields: `stems: Option<[Arc<LoadedAudio>; 4]>`, `stem_mute: [bool; 4]`, `stem_isolate: Option<u8>`
- When `stems` is Some, audible samples come from weighted sum of stems at shared playhead; else original

- [ ] Implement DSP mix + pad toggles.
- [ ] Snapshot exposes mute/isolate/ready for UI.
- [ ] Commit.

---

### Task 4: Host + Flutter settings & pads

**Files:**
- Modify: `host-flutter` settings (`stems_enabled`), engine PadMode FRB, library ensure on prepare
- Modify: `pad_modes.dart`, `deck_pads_panel.dart`, new `stems_pads.dart`, settings library panel
- Regenerate FRB: `moon run gui-flutter:generate` (or project script)
- Tests: Dart pad_modes + settings default

- [ ] Wire non-blocking ensure after load when setting on.
- [ ] Pads disabled until snapshot says ready.
- [ ] Commit.

---

### Task 5: Docs + PR

- [ ] Touch `docs/deck-spec.md` §5.10 status line if needed (minimal).
- [ ] Push branch and open PR referencing #46 and design doc.
- [ ] Note charon findings + realtime follow-up in PR body.

## Spec coverage

| Spec item | Task |
|-----------|------|
| Settings gate | 4 (+ 2 respects flag) |
| Analyze + load enqueue | 2, 4 |
| Non-blocking load | 4 |
| Pads when ready | 3, 4 |
| 4-stem mute/isolate | 3, 4 |
| PCM in / Mixar cache | 1, 2 |
| Realtime docs only | design doc (done) |
| No charon dep | 1 |

## Execution note

Owner requested continuum to PR without per-section review. Prefer inline execution over subagent handoff unless blocked.
