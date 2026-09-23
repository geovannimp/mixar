# NI `.stem.mp4` Cache & Playback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the four-file Opus/FLAC stem cache with one NI-compatible `.stem.mp4`, open user Stem files in place, and use a Stem file (native or cache) as the playback source when present.

**Architecture:** `codec` gains pure-Rust Stem demux/mux + STEM atom. `analyzer-stems` separates PCM then muxes five consistent streams (NI order). `library` stores one cache path and ensures only during analyze. Prepare resolves native Stem or cache → `StemBundle` on `PreparedTrackPlayback`; deck load attaches stems immediately and **never** starts ensure.

**Tech Stack:** Symphonia (demux/decode), pure-Rust MP4 mux (`mp4io` preferred — multi-track Opus/AAC), existing `ruopus` / `flacenc` for encode, lofty or raw `udta` for STEM JSON atom, sea-orm-sync schema, FRB/host unchanged transport shape where possible.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-09-23-ni-stem-mp4-cache-design.md`
- No ffmpeg
- Stem order (file + engine + pads): **drums, bass, other, vocals** (mixdown = stream 0)
- Consistent streams only (all five same codec + rate)
- `stems_format`: `opus` (default) | `flac` | `aac` — ship opus+flac mux first; `aac` ensure returns a clear error until AAC encode lands
- Ensure **only** from analyze/library worker — remove deck-load ensure/hot-swap
- Native `.stem.mp4`: never Demucs, never Mixar cache row
- Worktree: `/home/geovanni/Projects/mixar`; Cargo: `cargo --manifest-path crates/Cargo.toml …`
- Commit signing: if 1Password fails, leave staged — no `--no-gpg-sign`
- Prefer `git add -f` for paths under `docs/superpowers/` (gitignored)

## File map

| File | Role |
|------|------|
| `crates/codec/src/stem_atom.rs` | STEM JSON types + defaults (stemgen-compatible) |
| `crates/codec/src/stem_mp4.rs` | `is_stem_path`, demux → PCM×5, mux consistent streams + STEM atom |
| `crates/codec/src/lib.rs` | Re-exports; keep single-track `AudioDecoder` |
| `crates/codec/Cargo.toml` | Add mux crate (`mp4io`), serde/serde_json if needed |
| `crates/analyzer-stems/src/infer.rs` | `SOURCE_ORDER` → identity NI order |
| `crates/analyzer-stems/src/split.rs` | `STEM_NAMES` NI; write via mux not 4 files |
| `crates/analyzer-stems/src/format.rs` | Keep encode helpers used by mux, or thin wrappers |
| `crates/library/src/entity/track_stem.rs` | Single `path` column |
| `crates/library/src/stems.rs` | Ensure → `.stem.mp4`; skip native; clear old dirs |
| `crates/library/src/lib.rs` | `PreparedTrackPlayback.stems`; prepare resolution |
| `crates/host-flutter/src/api/engine.rs` | Load attaches prepared stems; delete deck ensure spawn |
| `apps/gui-flutter/lib/mixer/pad_modes.dart` | `kStemNames` → NI order (index alignment, not polish) |
| Settings normalize | Accept `aac`; reject at ensure until encoder exists |
| Docs | Point pad/storage designs at this spec |

---

### Task 1: NI stem order + STEM atom types

**Files:**
- Create: `crates/codec/src/stem_atom.rs`
- Modify: `crates/codec/src/lib.rs`
- Modify: `crates/analyzer-stems/src/infer.rs` (`SOURCE_ORDER`)
- Modify: `crates/analyzer-stems/src/split.rs` (`STEM_NAMES`)
- Modify: `apps/gui-flutter/lib/mixer/pad_modes.dart` (`kStemNames`)
- Test: unit tests in `stem_atom.rs` + existing `infer` order test

**Interfaces:**
- Produces:
  - `pub const STEM_NAMES: [&str; 4] = ["drums", "bass", "other", "vocals"];` in `analyzer-stems` (and same strings in Flutter)
  - `codec::StemAtom` with `version: i32`, `stems: [StemSlot; 4]`, `mastering_dsp: MasteringDsp` (disabled defaults)
  - `StemAtom::default_ni()` matching stemgen JSON colors/names
  - `SOURCE_ORDER: [0, 1, 2, 3]` (ONNX already drums/bass/other/vocals)

- [ ] **Step 1: Write failing atom round-trip test**

```rust
#[test]
fn stem_atom_default_matches_stemgen() {
    let json = serde_json::to_string(&StemAtom::default_ni()).unwrap();
    assert!(json.contains("\"name\":\"Drums\""));
    assert!(json.contains("#009E73"));
    assert!(json.contains("\"version\":1"));
    let back: StemAtom = serde_json::from_str(&json).unwrap();
    assert_eq!(back, StemAtom::default_ni());
}
```

- [ ] **Step 2: Run test — expect fail (type missing)**

Run: `cargo test --manifest-path crates/Cargo.toml -p codec stem_atom -- --nocapture`

- [ ] **Step 3: Implement `stem_atom.rs` + wire `mod` / re-export; set `SOURCE_ORDER = [0,1,2,3]`; update `STEM_NAMES` and Flutter `kStemNames` to `drums, bass, other, vocals` (display labels can stay short: `drums`/`bass`/`other`/`vocals`)**

- [ ] **Step 4: Fix `source_order_maps_onnx_to_mixar` → assert identity; run analyzer-stems + codec tests**

Run: `cargo test --manifest-path crates/Cargo.toml -p codec stem_atom`
Run: `cargo test --manifest-path crates/Cargo.toml -p analyzer-stems source_order`

- [ ] **Step 5: Commit**

```bash
git add crates/codec/src/stem_atom.rs crates/codec/src/lib.rs \
  crates/analyzer-stems/src/infer.rs crates/analyzer-stems/src/split.rs \
  apps/gui-flutter/lib/mixer/pad_modes.dart
git commit -m "$(cat <<'EOF'
feat(stems): NI stem order and STEM atom types

EOF
)"
```

---

### Task 2: Detect + demux `.stem.mp4` (Symphonia)

**Files:**
- Create: `crates/codec/src/stem_mp4.rs`
- Modify: `crates/codec/src/lib.rs`, `crates/codec/Cargo.toml`
- Test: `crates/codec` unit tests (synthetic or checked-in tiny fixture when available)

**Interfaces:**
- Produces:
  - `pub fn is_stem_path(path: &Path) -> bool` — true if file name ends with `.stem.mp4` (ASCII case-insensitive)
  - `pub struct StemPcmBundle { pub sample_rate: u32, pub mixdown: Vec<f32>, pub stems: [Vec<f32>; 4], pub atom: Option<StemAtom> }`
  - `pub fn decode_stem_file(path: &Path) -> Result<StemPcmBundle>`
    - Probe with Symphonia; require ≥5 audio tracks
    - Decode track 0 → mixdown interleaved stereo f32
    - Decode tracks 1..4 → stems in file order (already NI)
    - Try parse STEM atom from metadata if present; else `atom: None` (still OK if 5 streams)
  - Error if fewer than 5 decodable audio tracks

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn is_stem_path_detects_extension() {
    assert!(is_stem_path(Path::new("/x/Track.stem.mp4")));
    assert!(is_stem_path(Path::new("/x/Track.STEM.MP4")));
    assert!(!is_stem_path(Path::new("/x/Track.mp4")));
    assert!(!is_stem_path(Path::new("/x/vocals.opus")));
}
```

For demux: if no fixture yet, skip heavy test with `#[ignore]` and add a mux→demux round-trip in Task 3; still implement `decode_stem_file` API now.

- [ ] **Step 2: Implement demux by iterating Symphonia `format.tracks()`, decoding each audio track fully (reuse buffer-copy patterns from `AudioDecoder::load_entire_file`). Do not change single-track `AudioDecoder` behavior for normal files.**

- [ ] **Step 3: Run**

Run: `cargo test --manifest-path crates/Cargo.toml -p codec is_stem_path`

- [ ] **Step 4: Commit**

```bash
git add crates/codec/src/stem_mp4.rs crates/codec/src/lib.rs crates/codec/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(codec): demux NI .stem.mp4 into mixdown + four stems

EOF
)"
```

---

### Task 3: Mux consistent `.stem.mp4` (opus + flac) + STEM atom

**Files:**
- Modify: `crates/codec/src/stem_mp4.rs`, `crates/codec/Cargo.toml`
- Optionally reuse encode helpers from `analyzer-stems` **or** move shared encode into `codec` to avoid a cyclic dependency (`analyzer-stems` already depends on codec? check — if not, keep encode in analyzer-stems and only mux packets in codec). Prefer: `codec` accepts **already-encoded** frames OR raw PCM and encodes opus/flac internally using deps already used by analyzer-stems (`ruopus`, `flacenc`) so library/analyzer call one API.

**Interfaces:**
- Produces:
  - `pub enum StemMuxFormat { Opus, Flac }` — no Aac yet
  - `pub fn encode_stem_mp4(path: &Path, format: StemMuxFormat, sample_rate: u32, mixdown: &[f32], stems: [&[f32]; 4], atom: &StemAtom) -> Result<()>`
    - Require interleaved stereo; same frame count across all five (trim/pad policy: truncate to shortest)
    - Write temp file then rename onto `path`
    - Five tracks, same codec/rate; STEM atom written (lofty freeform / custom `udta` — match TagLib `STEM` complex property shape stemgen uses: JSON string)
  - Round-trip test: encode → `decode_stem_file` → 5 streams, NI order, atom present

**Mux crate:** add `mp4io` (multi-track Opus/AAC writer). If Opus-in-MP4 via `mp4io` fails in practice, fall back to documenting blocker and ship **FLAC** path first while keeping opus API — do not introduce ffmpeg.

- [ ] **Step 1: Failing round-trip test**

```rust
#[test]
fn stem_mp4_opus_round_trip() {
    let rate = 48_000;
    let frames = rate / 10;
    let tone = |amp: f32| -> Vec<f32> {
        (0..frames * 2).map(|i| amp * ((i / 2) as f32 * 0.01).sin()).collect()
    };
    let mix = tone(0.1);
    let stems = [tone(0.2), tone(0.3), tone(0.4), tone(0.5)];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.stem.mp4");
    encode_stem_mp4(
        &path,
        StemMuxFormat::Opus,
        rate,
        &mix,
        [&stems[0], &stems[1], &stems[2], &stems[3]],
        &StemAtom::default_ni(),
    )
    .unwrap();
    let bundle = decode_stem_file(&path).unwrap();
    assert_eq!(bundle.stems.len(), 4);
    assert!(bundle.mixdown.len() > 1000);
    assert_eq!(bundle.atom.unwrap().stems[0].name, "Drums");
}
```

- [ ] **Step 2: Run — expect fail**

Run: `cargo test --manifest-path crates/Cargo.toml -p codec stem_mp4_opus_round_trip -- --nocapture`

- [ ] **Step 3: Implement encode+mux+STEM write; add flac variant test (may use model rate 44100)**

- [ ] **Step 4: Tests pass**

Run: `cargo test --manifest-path crates/Cargo.toml -p codec stem_mp4`

- [ ] **Step 5: Commit**

```bash
git add crates/codec/src/stem_mp4.rs crates/codec/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(codec): mux consistent-stream .stem.mp4 with STEM atom

EOF
)"
```

---

### Task 4: Library cache schema + ensure writes `.stem.mp4`

**Files:**
- Modify: `crates/library/src/entity/track_stem.rs`
- Modify: `crates/library/src/stems.rs`
- Modify: `crates/analyzer-stems/src/split.rs` (return mux path / stop writing 4 files)
- Modify: settings normalize paths (`host-flutter` / `library` bus) to allow `aac` string but ensure errors on `aac`
- Test: `crates/library/src/stems.rs` unit tests

**Interfaces:**
- Consumes: `codec::encode_stem_mp4`, `codec::is_stem_path`, `analyzer_stems::separate_interleaved` / split API
- Produces:
  - `TrackStemsInfo { path: PathBuf, backend, source_fingerprint, format, sample_rate, generated_at }`
  - Cache file: `{stems_root}/{fnv64(track_id)}.stem.mp4`
  - `ensure_*`: if source path `is_stem_path` → no-op success (no row)
  - `normalize_stems_format`: `opus` | `flac` | `aac`; unknown → `opus`
  - On `aac`: return `LibraryError` with message that AAC encode is not implemented yet
  - `clear_all_track_stems`: delete files + rows; remove leftover per-track directories
  - `all_files_exist` → `path.is_file()`
  - Split/ensure pipeline: Demucs PCM → mixdown = source PCM (resampled to stem rate) → mux five streams → publish via temp+rename

- [ ] **Step 1: Update entity to single `path`; adjust upsert OnConflict columns; sea-orm-sync will migrate on open**

- [ ] **Step 2: Rewrite `generate_stem_files` / `TrackStemsInfo` tests for one path; add test that `is_stem_path` source skips generation (mock or unit on helper)**

- [ ] **Step 3: Change `split_interleaved_stereo` to write one `.stem.mp4` into `output_dir` (e.g. `track.stem.mp4`) **or** have library call separate+`encode_stem_mp4` directly and thin/remove multifile split — prefer library owns output path `{key}.stem.mp4` and analyzer returns `[Vec<f32>;4]` only (cleaner). If so, add `pub use infer::separate_interleaved` as the ensure entry and deprecate multifile `StemSplitResult.paths`.**

Recommended API after this task:

```rust
// analyzer-stems
pub fn separate_interleaved(...) -> Result<([Vec<f32>; 4], String)>; // NI order

// library ensure
let (stems, ep) = analyzer_stems::separate_interleaved(...)?;
codec::encode_stem_mp4(&tmp_path, format, rate, &mixdown_pcm, [...], &StemAtom::default_ni())?;
```

- [ ] **Step 4: Run library stem tests**

Run: `cargo test --manifest-path crates/Cargo.toml -p library stems::`

- [ ] **Step 5: Commit**

```bash
git add crates/library/src/entity/track_stem.rs crates/library/src/stems.rs \
  crates/analyzer-stems/src/split.rs crates/analyzer-stems/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(library): cache stems as single .stem.mp4

EOF
)"
```

---

### Task 5: Prepare resolves StemBundle; deck load attaches; remove load-time ensure

**Files:**
- Modify: `crates/library/src/lib.rs` (`PreparedTrackPlayback`, `prepare_source_for_playback`)
- Modify: `crates/host-flutter/src/api/engine.rs` (load path, delete/stop using `spawn_stems_ensure_attach_task` on deck load)
- Modify: `crates/engine-core` / control load if it attaches stems — wire prepared stems
- Keep analyze worker `ensure_track_stems` as the only generator

**Interfaces:**
- Produces:

```rust
pub struct PreparedTrackPlayback {
    pub track_id: TrackId,
    pub source: AudioSource,
    pub audio: Arc<LoadedAudio>,           // mixdown or original
    pub stems: Option<[Arc<LoadedAudio>; 4]>, // NI order when present
    pub loudness_lufs: Option<f64>,
}
```

- Prepare algorithm:
  1. If file source and `is_stem_path(path)` → `decode_stem_file` → mixdown + stems; error if demux fails (no Demucs)
  2. Else if `buses.stems_enabled()` and `get_track_stems` valid → `decode_stem_file(info.path)` → mixdown + stems; on corrupt cache delete/invalidate row and fall back to original decode
  3. Else → existing original decode; `stems: None`
- Host `load_prepared`: after loading main buffer, if `prepared.stems` is `Some`, `attach_deck_stems` immediately
- **Delete** calls to `spawn_stems_ensure_attach` from deck load / post-load paths (grep and remove). Analyze/worker remains.

- [ ] **Step 1: Unit-test prepare resolution with temp dirs (native / cache hit / miss) under `library` tests — use tiny muxed fixture from Task 3 helpers**

- [ ] **Step 2: Implement prepare + host attach; remove ensure-on-load**

- [ ] **Step 3: Compile host + library**

Run: `cargo test --manifest-path crates/Cargo.toml -p library prepare`
Run: `cargo check --manifest-path crates/Cargo.toml -p host-flutter`

- [ ] **Step 4: Commit**

```bash
git add crates/library/src/lib.rs crates/host-flutter/src/api/engine.rs
git commit -m "$(cat <<'EOF'
feat(stems): prepare StemBundle and drop deck-load ensure

EOF
)"
```

---

### Task 6: Settings `aac` option + docs touch-up

**Files:**
- Modify: `apps/gui-flutter/lib/settings/settings_library_panel.dart` (dropdown includes aac)
- Modify: normalize helpers in host settings / library bus (already accept string)
- Modify: `docs/stems-pad-mode-design.md`, `docs/stems-storage-models-design.md` — one-line “cache format superseded by NI `.stem.mp4` spec”
- Test: Dart settings dirty test still defaults opus; optional test selecting aac

- [ ] **Step 1: Add `aac` to format dropdown; ensure still errors clearly if chosen before encoder exists**

- [ ] **Step 2: `moon run gui-flutter:analyze` or `flutter analyze` in `apps/gui-flutter` if moon heavy**

- [ ] **Step 3: Commit**

```bash
git add apps/gui-flutter/lib/settings/settings_library_panel.dart \
  docs/stems-pad-mode-design.md docs/stems-storage-models-design.md
git commit -m "$(cat <<'EOF'
chore(stems): settings aac option and docs pointer to stem.mp4 spec

EOF
)"
```

---

### Task 7: Engine index assert + smoke

**Files:**
- Modify: `crates/engine-dsp/src/deck.rs` tests — document NI index 0 = drums
- Optional: one host-level comment near attach

- [ ] **Step 1: Add/adjust test name or comment asserting attach order drums→bass→other→vocals (index 0 mute affects first stem buffer)**

- [ ] **Step 2: Run**

Run: `cargo test --manifest-path crates/Cargo.toml -p engine-dsp stems_`

- [ ] **Step 3: Commit if code changed**

```bash
git commit -m "$(cat <<'EOF'
test(engine-dsp): document NI stem index order

EOF
)"
```

---

## Spec coverage (self-review)

| Spec requirement | Task |
|------------------|------|
| Single `.stem.mp4` cache | 4 |
| Native Stem in place, no Demucs | 4, 5 |
| Early ensure only (analyze) | 5 (remove load ensure); worker already ensures |
| Consistent streams | 3 |
| opus default / flac / aac later | 3, 4, 6 |
| Pure Rust, no ffmpeg | 2, 3 |
| NI order file + engine | 1, 3, 5, 7 |
| STEM atom disabled mastering | 1, 3 |
| Prepare uses Stem as replacement | 5 |
| Corrupt cache fallback / native fail loud | 5 |
| Clear old 4-file dirs | 4 |
| Round-trip / prepare / engine tests | 3, 5, 7 |

## Placeholder / consistency check

- No TBD steps; mux crate named (`mp4io`) with explicit fallback to flac-first if Opus-in-MP4 blocks.
- `PreparedTrackPlayback.stems` naming consistent across Tasks 5–7.
- `STEM_NAMES` / pad names / file streams share drums→bass→other→vocals.
