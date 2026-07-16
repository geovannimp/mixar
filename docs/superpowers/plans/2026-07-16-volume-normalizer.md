# Volume Normalizer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Offline loudness (ReplayGain tags or BS.1770 LUFS) stored per track, applied as a separate `auto_gain_db` layer under user `gain_trim_db` when the volume normalizer is enabled.

**Architecture:** Measure/import loudness during analysis into `track_analysis.loudness_lufs`. Deck DSP uses `db_to_linear(auto_gain_db + gain_trim_db)`. On library load (and settings change), Tauri sets `auto_gain_db = clamp(target − loudness, ±12)` or `0` when disabled/missing.

**Tech Stack:** Rust (`analyzer-core`, `analyzer`, `library`, `engine-dsp`, `engine-core`), `ebur128` crate, lofty ReplayGain tags, Tauri + React settings.

**Spec:** `docs/superpowers/specs/2026-07-16-volume-normalizer-design.md`  
**Issue:** [#67](https://github.com/geovannimp/rust-dj-engine/issues/67)

## Global Constraints

- Gain model: `effective_db = auto_gain_db + gain_trim_db` (knob = user offset only).
- Prefer ReplayGain **track** tags; else compute integrated LUFS (BS.1770).
- Default target **−18 LUFS**; default normalizer **enabled**.
- Missing loudness → `auto_gain_db = 0` (no surprise boost).
- Auto clamp **±12 dB**.
- ReplayGain → LUFS: `loudness_lufs = -18.0 - track_gain_db` (RG2 reference); then `auto = target - loudness` (when target is −18, auto ≈ tagged gain).
- Out of scope: file baking, real-time AGC, master limiter, VDJ knob-includes-auto.

---

## File map

| File | Responsibility |
|------|----------------|
| `analyzer-core/src/loudness.rs` | **New** — `auto_gain_db`, ReplayGain→LUFS, (optional) helpers |
| `analyzer-core/src/result.rs` | `loudness_lufs: Option<f64>` on `TrackAnalysis` |
| `analyzer-core/src/lib.rs` | `mod loudness`; re-exports |
| `analyzer/src/loudness.rs` | **New** — ebur128 measure on mono PCM |
| `analyzer/src/lib.rs` | Wire loudness into `analyze_pcm` / `analyze_file` |
| `analyzer/Cargo.toml` | Add `ebur128` |
| `library/src/tags.rs` | Read `ItemKey::ReplayGainTrackGain` |
| `library/src/entity/track_analysis.rs` | `loudness_lufs` column |
| `library/src/analysis.rs` | Persist/upsert loudness |
| `engine-dsp/src/deck.rs` | `auto_gain_db` + combined trim |
| `engine-core/src/engine.rs` | `set_deck_auto_gain_db` |
| `gui-app/src-tauri/src/lib.rs` | Settings + apply on load/save |
| `gui-app/src/types.ts` / `busSettings.ts` | Settings fields |
| `gui-app/src/components/settings/SettingsAudioPanel.tsx` | Toggle + target |

---

### Task 1: Pure auto-gain math (TDD)

**Files:**
- Create: `analyzer-core/src/loudness.rs`
- Modify: `analyzer-core/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub const REPLAYGAIN_REFERENCE_LUFS: f64 = -18.0;`
  - `pub const AUTO_GAIN_CLAMP_DB: f32 = 12.0;`
  - `pub fn loudness_lufs_from_replaygain_track_gain_db(track_gain_db: f64) -> f64`
  - `pub fn auto_gain_db(target_lufs: f32, loudness_lufs: f64) -> f32` — `clamp(target - loudness as f32, ±12)`

- [ ] **Step 1: Write failing tests** in `loudness.rs`:

```rust
#[test]
fn replaygain_plus_3_means_loudness_minus_21() {
    // +3 dB gain needed → track is 3 dB under −18 reference
    let l = loudness_lufs_from_replaygain_track_gain_db(3.0);
    assert!((l - (-21.0)).abs() < 1e-9);
}

#[test]
fn auto_gain_matches_difference_and_clamps() {
    assert!((auto_gain_db(-18.0, -18.0) - 0.0).abs() < 1e-5);
    assert!((auto_gain_db(-18.0, -24.0) - 6.0).abs() < 1e-5);
    assert!((auto_gain_db(-18.0, 0.0) - (-12.0)).abs() < 1e-5); // clamp
    assert!((auto_gain_db(-18.0, -40.0) - 12.0).abs() < 1e-5); // clamp
}
```

- [ ] **Step 2: Run** `cargo test -p analyzer-core loudness -- --nocapture` — expect FAIL

- [ ] **Step 3: Implement** formulas; export from `lib.rs`

- [ ] **Step 4: Run** — expect PASS

- [ ] **Step 5: Commit** `Add loudness auto-gain math helpers in analyzer-core.`

---

### Task 2: Deck `auto_gain_db` (TDD)

**Files:**
- Modify: `engine-dsp/src/deck.rs`

**Interfaces:**
- Produces: `Deck::set_auto_gain_db(&mut self, db: f32)`, `auto_gain_db(&self) -> f32` (default `0.0`, clamp with same ±24 trim clamp as `gain_trim_db` **or** leave uncapped here and only clamp at apply — prefer clamp with `clamp_gain_db` for safety)
- Process: `let trim = db_to_linear(self.auto_gain_db + self.gain_trim_db);`

- [ ] **Step 1: Failing tests** (reuse tone helpers):

```rust
#[test]
fn auto_gain_defaults_zero_and_adds_to_trim() {
    let mut deck = Deck::new(0, 48000, 512, "medium");
    // load constant tone, play, set volume 1, gain_trim 0, auto +6
    // peak with auto+6 ≈ 2× peak with auto 0 (within tolerance)
}
```

- [ ] **Step 2: Run** focused test — FAIL

- [ ] **Step 3: Implement field + process change**

- [ ] **Step 4: PASS + commit** `Apply auto_gain_db with gain trim on decks.`

---

### Task 3: Engine API

**Files:**
- Modify: `engine-core/src/engine.rs`

**Interfaces:**
- Produces: `Engine::set_deck_auto_gain_db(&mut self, deck_id: usize, gain_db: f32) -> Result<()>`

- [ ] **Step 1: Add method** next to `set_deck_gain_trim_db`

- [ ] **Step 2: Optional smoke** in an existing integration test if cheap; else skip

- [ ] **Step 3: Commit** `Expose set_deck_auto_gain_db on Engine.`

---

### Task 4: Measure LUFS in analyzer (TDD)

**Files:**
- Create: `analyzer/src/loudness.rs`
- Modify: `analyzer/src/lib.rs`, `analyzer/Cargo.toml`
- Modify: `analyzer-core/src/result.rs` — add `pub loudness_lufs: Option<f64>` to `TrackAnalysis` (default `None` in constructors/tests)

**Interfaces:**
- Produces: `pub fn integrated_lufs_mono(samples: &[f32], sample_rate: u32) -> Result<f64, AnalyzerError>`
- `analyze_pcm` / `analyze_file` set `track.loudness_lufs = Some(...)`

- [ ] **Step 1: Add dependency** `ebur128 = "0.1"` (or current stable) to `analyzer/Cargo.toml`

- [ ] **Step 2: Failing test** — synthetic sine or silence: loudness is finite; louder buffer → higher LUFS than quieter (relative)

- [ ] **Step 3: Implement** with `ebur128::EbuR128` mode I (integrated), channel 1 mono

- [ ] **Step 4: Wire into analyze path** after prepare/decode

- [ ] **Step 5: Commit** `Measure integrated LUFS during offline analysis.`

---

### Task 5: Prefer ReplayGain tags + persist

**Files:**
- Modify: `library/src/tags.rs` — parse `ReplayGainTrackGain` (string like `"+3.20 dB"`)
- Modify: `library` analyze path (`analyze_file_source` / merge) — if tag gain present, set `loudness_lufs` via `loudness_lufs_from_replaygain_track_gain_db` and **skip** recompute (or overwrite analyzer value)
- Modify: `library/src/entity/track_analysis.rs` — `pub loudness_lufs: Option<f64>`
- Modify: `library/src/analysis.rs` — upsert column

**Preferred order in analyze:**
1. Run analyzer (gets computed LUFS)
2. If file has ReplayGain track gain, **replace** `loudness_lufs` with tag-derived value
3. Upsert

- [ ] **Step 1: Tag parse helper + unit test** for `"+3.20 dB"` / `"−1.5 dB"`

- [ ] **Step 2: Entity + upsert**

- [ ] **Step 3: Integration** — existing `analyze_track_persists_track_analysis` extended to assert `loudness_lufs.is_some()` on a real fixture if available; else unit-level

- [ ] **Step 4: Commit** `Persist loudness_lufs from tags or analysis.`

---

### Task 6: Settings schema + UI

**Files:**
- Modify: `gui-app/src-tauri/src/lib.rs` — `AppSettings` fields with serde defaults
- Modify: `gui-app/src/types.ts`, `gui-app/src/lib/busSettings.ts` (`normalizeAppSettings`)
- Modify: `gui-app/src/components/settings/SettingsAudioPanel.tsx` — toggle + target control
- Modify: `LibraryPanel` / any hardcoded default `AppSettings` literals

**Fields:**
```rust
#[serde(default = "default_volume_normalizer_enabled")]
volume_normalizer_enabled: bool, // true
#[serde(default = "default_target_lufs")]
target_lufs: f32, // -18.0
```

- [ ] **Step 1: Backend settings** + defaults in `settings_from_state` / `apply_settings`

- [ ] **Step 2: TS types + normalize**

- [ ] **Step 3: UI** under Engine or new “Normalization” group: Switch + number/slider (−24…−9 LUFS)

- [ ] **Step 4: Commit** `Add volume normalizer settings.`

---

### Task 7: Apply on load + settings save

**Files:**
- Modify: `gui-app/src-tauri/src/lib.rs`
- Possibly: store `loudness_lufs` on `DeckInfo` when loading library track

**Logic:**
```rust
fn apply_deck_auto_gain(state: &mut AppState, deck_id: usize) -> Result<(), String> {
    let enabled = /* from settings_from_state or cached fields on AppState */;
    let target = state…target_lufs;
    let loudness = state.decks[deck_id].loudness_lufs; // Option<f64>
    let auto = match (enabled, loudness) {
        (true, Some(l)) => analyzer_core::loudness::auto_gain_db(target, l),
        _ => 0.0,
    };
    state.decks[deck_id].auto_gain_db = auto;
    if let Some(engine) = state.engine.as_mut() {
        engine.set_deck_auto_gain_db(deck_id, auto)?;
    }
    Ok(())
}
```

- On `load_library_track_to_deck`: fetch analysis loudness from library DB; set `DeckInfo.loudness_lufs`; call `apply_deck_auto_gain`.
- On `load_track` (raw path): `loudness_lufs = None` → auto 0 unless you add opportunistic analyze (out of scope).
- On `save_settings`: after apply, for each deck with a track, `apply_deck_auto_gain`.
- Expose `auto_gain_db` on `DeckStatus` optional for debugging (nice-to-have).

**Library fetch:** add `store`/`Library` helper `track_loudness_lufs(track_id) -> Result<Option<f64>>` reading `track_analysis`.

- [ ] **Step 1: DeckInfo + fetch helper**

- [ ] **Step 2: Wire load_library_track_to_deck**

- [ ] **Step 3: Wire save_settings recompute**

- [ ] **Step 4: Commit** `Apply auto gain on library load and settings save.`

---

### Task 8: Verification

- [ ] Run:
  - `cargo test -p analyzer-core`
  - `cargo test -p analyzer`
  - `cargo test -p engine-dsp auto_gain`
  - `cargo test -p library analyze_track`
- [ ] Manual: analyze two tracks with different loudness; enable normalizer; load both at trim 0 — levels closer; disable — disparity returns; trim still offsets.
- [ ] Comment on #67 with branch name (optional).
- [ ] Commit only if docs need a one-line update.

---

## Spec coverage (self-review)

| Spec item | Task |
|-----------|------|
| Separate auto + trim layers | 2 |
| Tags then LUFS | 4, 5 |
| −18 default / enabled true | 6 |
| Missing → 0 | 7 |
| ±12 clamp | 1 |
| Persist loudness_lufs | 5 |
| Settings recompute | 7 |
| No file bake / AGC / limiter | — |

ReplayGain conversion locked: `loudness = -18 - gain_db`. Method names: `auto_gain_db`, `set_deck_auto_gain_db`, settings `volume_normalizer_enabled` / `target_lufs`.
