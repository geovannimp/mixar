# Keyboard & Key Shift Pad Modes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add realtime key-shift pitch to the engine and expose it as Keyboard and Key Shift performance-pad modes across the engine bus, DDJ-400 mapping, and Flutter UI.

**Architecture:** A realtime pitch factor is added to `crates/stretch` (keylock time-stretch at `tempo/pitch` plus a stateful `StreamingSincResampler` output stage). `engine-dsp` owns per-deck `key_shift_semitones`; `engine-core` exposes `SetKeyShift` / `SetKeyboardScale` commands and Keyboard/KeyShift pad handlers with session-only state; the controller binds the DDJ-400 note banks; Flutter renders the two pad grids.

**Tech Stack:** Rust workspace (`cargo --manifest-path crates/Cargo.toml`), `timestretch` 0.14 (`StreamingSincResampler`), engine cmd/evt bus, `flutter_rust_bridge`, Flutter (Shad/Mixar shell).

**Spec:** `docs/keyboard-key-shift-pad-modes-design.md`

## Global Constraints

- Cargo: `cargo --manifest-path crates/Cargo.toml …` (or `cd crates`). `engine-core` default features include `backend-cpal`; tests use `backend: "null"`.
- `engine-dsp` stays pure: no I/O, no filesystem, no library access.
- Key shift and keyboard scale are **session-only**; never write `track_*` rows or library key corrections.
- `P == 1.0` must bypass the pitch resampler (bit-identical to today).
- Engine pad slots are zero-based `0..7`; UI/DDJ-400 pads are `1..8`.
- Pitch is `2^(semitones/12)`; clamp semitones to `-12..=+12` and reject non-finite (treat as 0).
- Engine + controller are the source of truth; any input path (UI, MIDI, tests) publishes the same cmd.
- Pre-commit hook runs rustfmt/clippy for `.rs` and `dart format`/`flutter analyze` for `.dart`; do not bypass.

## Review Focus

The spec implies these inputs/failure modes that no single task's happy-path test catches; each is pinned by a test in the owning task:

1. Key shift 0 preserves today's output exactly (bypass) — Task 2.
2. Non-finite / out-of-range semitones must clamp, never panic or overflow the output buffer — Task 2.
3. Key shift with stems attached still plays stems (mixed feed), not silence — Task 2.
4. Re-pressing the active Key Shift pad clears the offset; the shift-bank resets — Task 3.
5. Unload/load resets key shift and scale so a new track never inherits a stale shift — Task 2.

---

### Task 1: Realtime pitch factor in `stretch`

**Files:**
- Modify: `crates/stretch/src/lib.rs`
- Modify: `crates/stretch/src/engine.rs`
- Test: `crates/stretch/tests/pitch_factor.rs`

**Interfaces:**
- Consumes: `timestretch::core::resample::{SincInterpTable, StreamingSincResampler}` (`new_stream_default`, `new`, `process_into`, `set_step_anchor`, `reset`, `group_delay_samples`, `current_half_span`).
- Produces:
  - `pub fn semitones_to_pitch(semitones: f32) -> f64` in `stretch`.
  - `trait TimeStretcher { …; fn set_pitch_factor(&mut self, factor: f64) {} }` (default no-op).
  - `TimestretchStretcher::set_pitch_factor(&mut self, factor: f64)` and pitch-aware `pull_interleaved`.

- [ ] **Step 1: Write the failing tests**

`crates/stretch/tests/pitch_factor.rs`:

```rust
use stretch::semitones_to_pitch;

#[test]
fn semitone_math_is_exact() {
    assert!((semitones_to_pitch(0.0) - 1.0).abs() < 1e-12);
    assert!((semitones_to_pitch(12.0) - 2.0).abs() < 1e-12);
    assert!((semitones_to_pitch(-12.0) - 0.5).abs() < 1e-12);
    assert!((semitones_to_pitch(1.0) - 1.059_463_094_359_295_4).abs() < 1e-9);
}
```

The frequency tests (`produced_freq`, below in Step 4) need the resampler wired up, so they are added with the implementation.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo --manifest-path crates/Cargo.toml test -p stretch --test pitch_factor semitone_math_is_exact`
Expected: FAIL — unresolved import `semitones_to_pitch`.

- [ ] **Step 3: Implement `semitones_to_pitch` and the trait method**

In `crates/stretch/src/lib.rs`:

```rust
/// Semitone offset → pitch multiplier (`2^(semitones/12)`).
pub fn semitones_to_pitch(semitones: f32) -> f64 {
    f64::from(semitones).mul_add(1.0 / 12.0, 0.0).exp2()
}
```

Add to `trait TimeStretcher` after `set_tempo_rate`:

```rust
/// Set pitch multiplier (`1.0` = original; `2.0` = +1 octave). Tempo is held.
fn set_pitch_factor(&mut self, _factor: f64) {}
```

- [ ] **Step 4: Implement pitch resampling in `TimestretchStretcher`**

Add fields: `pitch: f64`, `resampler_l: StreamingSincResampler`, `resampler_r: StreamingSincResampler`, `carry: Vec<Sample>`, `carry_head: usize`. Construct the resamplers from one shared `SincInterpTable::new_stream_default()`.

`set_pitch_factor` clamps to `[0.25, 4.0]`, maps non-finite to `1.0`, and on change resets both resamplers, sets both step anchors, and clears `carry`.

`pull_interleaved` when `(pitch - 1.0).abs() < 1e-6` keeps the current body unchanged. Otherwise:

- `engine_tempo = tempo_rate / pitch` (the stretcher already receives `tempo_rate`; keep `set_tempo_rate` storing the *requested* tempo and divide only when calling `self.controller.set_tempo_rate`).
- Maintain `carry` as interleaved `f32` (drain from `carry_head`; compact when `carry_head > 4096`).
- Loop until `carry_frames() >= out_frames`:
  - `need = out_frames - carry_frames()`.
  - `engine_frames = ((need as f64) * pitch).ceil() as usize + self.resampler_l.current_half_span() + 8`.
  - `top_up(engine_frames, feed)`; `processor.process(&mut scratch[..engine_frames*2])`.
  - Deinterleave into `scratch_l`/`scratch_r`; `resampler_{l,r}.process_into(&scratch_x[..engine_frames], pitch, &mut out_x)`; append interleaved to `carry`.
  - Guard: if `top_up` fed 0 and `carry` did not grow, break to avoid a spin.
- Copy `out_frames*2` from `carry` into `output`, zero-fill any remainder, advance `carry_head`.

`start_delay()` adds `resampler_l.group_delay_samples()`; `reset()` clears `carry`, resets both resamplers, and re-anchors steps. `queued_source_frames()` keeps returning `source.occupied_frames()`.

Add the frequency tests to the same test file (extend the imports to
`use stretch::{semitones_to_pitch, TimeStretcher, TimestretchStretcher};`):

```rust
fn produced_freq(semitones: f32) -> f64 {
    let mut s = TimestretchStretcher::new(SR, 512).unwrap();
    s.set_tempo_rate(1.0);
    s.set_pitch_factor(semitones_to_pitch(semitones));
    let total = SR as usize * 2;
    let mut buf = vec![0.0f32; total * 2];
    let mut phase = 0.0f64;
    let step = 440.0 / SR as f64;
    let n = s
        .pull_interleaved(total, &mut buf, &mut |need, out| {
            for i in 0..need {
                let v = (phase * std::f64::consts::TAU).sin() as f32 * 0.5;
                out[i * 2] = v;
                out[i * 2 + 1] = v;
                phase += step;
            }
            need
        })
        .out_frames;
    let tail = &buf[(n - SR as usize / 4) * 2..n * 2];
    // Zero-crossing frequency estimate on the left channel.
    let mut crossings = 0usize;
    for w in tail.chunks_exact(2).collect::<Vec<_>>().windows(2) {
        if w[0][0] <= 0.0 && w[1][0] > 0.0 {
            crossings += 1;
        }
    }
    crossings as f64 * (SR as f64) / (SR as f64 / 4.0)
}

#[test]
fn pitch_up_octave_doubles_frequency() {
    let f = produced_freq(12.0);
    assert!((f - 880.0).abs() / 880.0 < 0.03, "got {f}");
}

#[test]
fn pitch_down_octave_halves_frequency() {
    let f = produced_freq(-12.0);
    assert!((f - 220.0).abs() / 220.0 < 0.03, "got {f}");
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo --manifest-path crates/Cargo.toml test -p stretch`
Expected: PASS (all tests including the existing ones).

- [ ] **Step 6: Commit**

```bash
git add crates/stretch/src/lib.rs crates/stretch/src/engine.rs crates/stretch/tests/pitch_factor.rs
git commit -m "feat(stretch): realtime pitch factor for key shift (#298)"
```

---

### Task 2: `engine-api` surface + `engine-dsp` deck key shift

**Files:**
- Modify: `crates/engine-api/src/payload.rs`
- Modify: `crates/engine-api/src/kind.rs`
- Modify: `crates/engine-dsp/src/deck.rs`
- Modify: `crates/engine-core/src/sync.rs` (`DeckControlState`)
- Modify: `crates/engine-core/src/engine.rs` (state/cmds/snapshot/clear)
- Modify: `crates/engine-core/src/control.rs` (dispatch + `deck_snapshot_to_evt`)
- Test: `crates/engine-dsp/src/deck.rs` (`#[cfg(test)] mod tests`)
- Test: `crates/engine-core/tests/bus_key_shift.rs`

**Interfaces:**
- Consumes: `stretch::{semitones_to_pitch, TimeStretcher::set_pitch_factor}` (Task 1).
- Produces:
  - `engine_api::KeyboardScale { Major, Minor, Pentatonic }` (Default `Major`, `snake_case`).
  - `CmdBody::SetKeyShift { semitones: f32 }`, `CmdBody::SetKeyboardScale { scale: KeyboardScale }`.
  - `Kind::SetKeyShift`, `Kind::SetKeyboardScale`.
  - `EvtBody::DeckUpdated` + `DeckSnapshot` fields `key_shift: f32`, `keyboard_scale: KeyboardScale` (`#[serde(default)]`).
  - `Deck::set_key_shift_semitones(f32)`, `Deck::key_shift_semitones() -> f32`, `Deck::pitch_factor() -> f64`.
  - `DeckControlState { key_shift_semitones: f32, keyboard_scale: KeyboardScale }`.
  - `Engine::set_deck_key_shift(deck, semitones) -> Result<()>`, `Engine::set_deck_keyboard_scale(deck, scale) -> Result<()>`.

- [ ] **Step 1: Write the failing engine-dsp test**

In `crates/engine-dsp/src/deck.rs` tests, add:

```rust
#[test]
fn key_shift_changes_pitch_not_tempo() {
    let mut deck = new_deck(CHUNK);
    // 440 Hz sine at engine rate, 2 s.
    let n = ENGINE_RATE as usize * 2;
    let mut samples = vec![0.0f32; n * 2];
    for i in 0..n {
        let v = ((i as f64) * 440.0 / f64::from(ENGINE_RATE) * std::f64::consts::TAU).sin() as f32;
        samples[i * 2] = v;
        samples[i * 2 + 1] = v;
    }
    load_test_samples(&mut deck, samples, ENGINE_RATE);
    deck.set_key_shift_semitones(12.0).unwrap();
    deck.play().unwrap();
    let before = deck.position_frames();
    let mut out = Vec::new();
    for _ in 0..64 {
        out.extend_from_slice(deck.process(CHUNK).unwrap());
    }
    // Pitch doubled.
    let f = estimate_frequency(&out, ENGINE_RATE);
    assert!((f - 880.0).abs() / 880.0 < 0.05, "got {f}");
    // Tempo unchanged: ~64 * CHUNK source frames advanced.
    let advanced = deck.position_frames() - before;
    assert!((advanced - (64 * CHUNK) as i64).abs() < 2_000);
}

#[test]
fn key_shift_zero_matches_unset_output() {
    let mut a = new_deck(CHUNK);
    let mut b = new_deck(CHUNK);
    let n = ENGINE_RATE as usize;
    let samples: Vec<f32> = (0..n * 2).map(|i| (i as f32 * 0.01).sin()).collect();
    load_test_samples(&mut a, samples.clone(), ENGINE_RATE);
    load_test_samples(&mut b, samples, ENGINE_RATE);
    a.set_key_shift_semitones(0.0).unwrap();
    a.play().unwrap();
    b.play().unwrap();
    for _ in 0..8 {
        let x = a.process(CHUNK).unwrap().to_vec();
        let y = b.process(CHUNK).unwrap().to_vec();
        assert_eq!(x, y);
    }
}

#[test]
fn key_shift_clamps_and_rejects_non_finite() {
    let mut deck = new_deck(CHUNK);
    load_test_samples(&mut deck, vec![0.1; CHUNK * 2 * 8_000], ENGINE_RATE);
    deck.set_key_shift_semitones(f32::NAN).unwrap();
    assert_eq!(deck.key_shift_semitones(), 0.0);
    deck.set_key_shift_semitones(120.0).unwrap();
    assert_eq!(deck.key_shift_semitones(), 12.0);
}

#[test]
fn key_shift_with_stems_keeps_stem_audio() {
    let mut deck = new_deck(CHUNK);
    load_test_samples(&mut deck, vec![0.0; CHUNK * 2 * 8_000], ENGINE_RATE);
    let stem = Arc::new(LoadedAudio {
        samples: (0..CHUNK * 2 * 8_000).map(|i| (i as f32 * 0.02).sin() * 0.5).collect(),
        sample_rate: ENGINE_RATE,
        channels: 2,
        source_id: "stem.wav".into(),
    });
    deck.attach_stems([Arc::clone(&stem), Arc::clone(&stem), Arc::clone(&stem), Arc::clone(&stem)]);
    deck.set_key_shift_semitones(2.0).unwrap();
    deck.play().unwrap();
    let out = deck.process(CHUNK).unwrap();
    let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
    assert!(rms > 1e-3, "stems must stay audible under key shift, rms={rms}");
}

#[test]
fn key_shift_resets_on_unload() {
    let mut deck = new_deck(CHUNK);
    load_test_samples(&mut deck, vec![0.1; CHUNK * 2 * 8_000], ENGINE_RATE);
    deck.set_key_shift_semitones(5.0).unwrap();
    deck.unload().unwrap();
    assert_eq!(deck.key_shift_semitones(), 0.0);
}
```

Add an `estimate_frequency(&[f32], u32) -> f64` helper (zero-crossing over the last quarter of the signal) next to the other test helpers.

- [ ] **Step 2: Run to verify failure**

Run: `cargo --manifest-path crates/Cargo.toml test -p engine-dsp key_shift`
Expected: FAIL — `set_key_shift_semitones` not found.

- [ ] **Step 3: Implement `engine-dsp` deck key shift**

Add field `key_shift_semitones: f32` (init `0.0`). Methods:

```rust
pub fn key_shift_semitones(&self) -> f32 { self.key_shift_semitones }
pub fn pitch_factor(&self) -> f64 {
    stretch::semitones_to_pitch(self.key_shift_semitones)
}
pub fn set_key_shift_semitones(&mut self, semitones: f32) -> Result<()> {
    let s = if semitones.is_finite() { semitones.clamp(-12.0, 12.0) } else { 0.0 };
    if (s - self.key_shift_semitones).abs() < f32::EPSILON { return Ok(()); }
    self.key_shift_semitones = s;
    self.reset_stretcher_state();
    self.stretch_active = false;
    Ok(())
}
```

Change path selection in `process`:
`let want_stretch = self.key_shift_semitones != 0.0 || (self.key_lock && self.stems.is_none());`

In `play_stretched`, after `stretcher.set_tempo_rate(playback_ratio)` call
`stretcher.set_pitch_factor(self.pitch_factor());`. Replace the feed's `interpolate_stereo(audio_samples, position_frac)` with a helper
`self.read_frame_at(position_frac, audio_samples, source_rate)` that returns
`self.mix_at_position(position_frac, audio_samples, source_rate)` when
`self.stems.is_some()`, else `interpolate_stereo(...)`.

`unload()` sets `key_shift_semitones = 0.0`. `stem` attach/clear need no change.

- [ ] **Step 4: Run to verify pass**

Run: `cargo --manifest-path crates/Cargo.toml test -p engine-dsp key_shift`
Expected: PASS.

- [ ] **Step 5: Write the failing engine-core bus test**

`crates/engine-core/tests/bus_key_shift.rs` (model on `bus_key_lock.rs`):

```rust
//! Integration: SetKeyShift / SetKeyboardScale publish on DeckUpdated.

mod common;

use common::{recv_evt_kind, short_tone_fixture};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, KeyboardScale, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource, TrackId, TrackMetadata};
use omnibus::Filter;

#[test]
fn set_key_shift_and_scale_publish_fields() {
    let config = EngineConfig { backend: "null".to_string(), ..Default::default() };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|e| e.start()).expect("start");
    session.with_engine(|e| {
        e.load_track(0, AudioSource::File(FileAudioSource::new(
            TrackId::new("keyshift.wav"),
            short_tone_fixture(),
            TrackMetadata { bpm: Some(120.0), ..Default::default() },
        )))
    }).expect("load");

    let evt = session.evt_bus().subscribe(Filter::Any, Filter::Any).expect("sub");

    session.publish_cmd(Origin::Deck(0), Kind::SetKeyShift,
        encode_cmd_body(&CmdBody::SetKeyShift { semitones: 2.0 }).unwrap()).unwrap();
    let body = decode_evt_body(recv_evt_kind(&evt, Kind::Updated).payload()).unwrap();
    let EvtBody::DeckUpdated { key_shift, .. } = body else { panic!("DeckUpdated") };
    assert!((key_shift - 2.0).abs() < 1e-6);

    session.publish_cmd(Origin::Deck(0), Kind::SetKeyboardScale,
        encode_cmd_body(&CmdBody::SetKeyboardScale { scale: KeyboardScale::Minor }).unwrap()).unwrap();
    let body = decode_evt_body(recv_evt_kind(&evt, Kind::Updated).payload()).unwrap();
    let EvtBody::DeckUpdated { keyboard_scale, .. } = body else { panic!("DeckUpdated") };
    assert_eq!(keyboard_scale, KeyboardScale::Minor);
}

#[test]
fn unload_resets_key_shift_and_scale() {
    // Same session setup. Set key shift 12.0 + Pentatonic, then:
    session.with_engine(|e| e.unload_deck(0)).expect("unload");
    let snap = session.with_engine(|e| e.deck_snapshot(0)).flatten().expect("snapshot");
    assert_eq!(snap.key_shift, 0.0);
    assert_eq!(snap.keyboard_scale, KeyboardScale::Major);
}
```

(`Engine::with_engine` closures return a value; mirror how `bus_key_lock.rs`
loads and how existing tests read a snapshot.)

- [ ] **Step 6: Run to verify failure**

Run: `cargo --manifest-path crates/Cargo.toml test -p engine-core --test bus_key_shift`
Expected: FAIL — `KeyboardScale` / `SetKeyShift` unresolved.

- [ ] **Step 7: Implement the `engine-api` surface**

In `payload.rs`: add `KeyboardScale` enum; extend `PadMode` is **not** done here; add the two `CmdBody` variants; add `#[serde(default)] pub key_shift: f32` and `#[serde(default)] pub keyboard_scale: KeyboardScale` to `DeckSnapshot`, and the same two fields to `EvtBody::DeckUpdated`.
In `kind.rs`: add `SetKeyShift`, `SetKeyboardScale`.
Then fix every literal construction site the new fields break — grep for `DeckUpdated {` and `DeckSnapshot {` across `crates/` and `apps/` (e.g. `engine-api` test helpers, `engine-core/tests/session_publish_evt.rs`) and add the two fields.

- [ ] **Step 8: Implement `engine-core` state, commands, snapshot**

`DeckControlState`: add `key_shift_semitones: f32`, `keyboard_scale: KeyboardScale`; reset both in `clear_loaded_track`.

`Engine`:

```rust
pub fn set_deck_key_shift(&mut self, deck_id: usize, semitones: f32) -> Result<()> {
    let s = if semitones.is_finite() { semitones.clamp(-12.0, 12.0) } else { 0.0 };
    let control = self.deck_control.get_mut(deck_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
    control.key_shift_semitones = s;
    if let Some(dsp) = self.dsp_engine.as_ref() {
        let mut dsp = dsp.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) { deck.set_key_shift_semitones(s)?; }
    }
    Ok(())
}

pub fn set_deck_keyboard_scale(&mut self, deck_id: usize, scale: KeyboardScale) -> Result<()> { /* store on control */ }
```

`deck_snapshot_from_dsp`: `key_shift: control.key_shift_semitones, keyboard_scale: control.keyboard_scale`.
`deck_snapshot_to_evt` (`control.rs`): forward both fields.
`control.rs` dispatch: add `(Kind::SetKeyShift, CmdBody::SetKeyShift { .. })` and `(Kind::SetKeyboardScale, CmdBody::SetKeyboardScale { .. })` to the valid-pair match; dispatch to the setters returning `CmdOutcome::DeckUpdated(deck_id)`.

- [ ] **Step 9: Run to verify pass**

Run: `cargo --manifest-path crates/Cargo.toml test -p engine-core --test bus_key_shift`
Expected: PASS. Then `cargo --manifest-path crates/Cargo.toml test -p engine-dsp -p engine-api -p engine-core` — PASS.

- [ ] **Step 10: Commit**

```bash
git add crates/engine-api crates/engine-dsp crates/engine-core
git commit -m "feat(engine): session key-shift and keyboard-scale state (#298)"
```

---

### Task 3: Keyboard & Key Shift pad modes

**Files:**
- Modify: `crates/engine-api/src/payload.rs` (`PadMode` variants)
- Modify: `crates/engine-api/src/kind.rs` (pad kinds)
- Modify: `crates/engine-core/src/engine.rs` (handlers + dispatch)
- Modify: `crates/engine-core/src/control.rs` (kind dispatch)
- Modify: `crates/controller/src/catalog.rs` (`PAD_MODES`, leaves)
- Modify: `crates/controller/src/action.rs` (leaf resolution)
- Test: `crates/engine-core/tests/bus_pad_modes_kb_keyshift.rs`
- Test: `crates/controller/src/action.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: Task 2 (`set_deck_key_shift`, `set_deck_keyboard_scale`, `KeyboardScale`, `DeckControlState`).
- Produces:
  - `PadMode::Keyboard`, `PadMode::KeyShift`.
  - `Kind::KeyboardPadPress/Release`, `Kind::KeyShiftPadPress/Release`.
  - `Engine::keyboard_pad_press(deck, slot, shift)`, `keyboard_pad_release(deck, slot)`, `key_shift_pad_press(deck, slot, shift)`, `key_shift_pad_release(deck, slot)`.
  - `pub const KEY_SHIFT_PAD_SEMITONES: [i8; 8] = [0, 1, 2, 3, -4, -3, -2, -1];`
  - `pub fn keyboard_scale_degrees(scale: KeyboardScale) -> [u8; 8]` (major `[0,2,4,5,7,9,11,12]`, minor `[0,2,3,5,7,8,10,12]`, pentatonic `[0,2,4,7,9,12,14,16]`).

- [ ] **Step 1: Write the failing engine-core test**

`crates/engine-core/tests/bus_pad_modes_kb_keyshift.rs`: build a null session with a loaded track (copy the Task 2 helper), subscribe to evts, then:

```rust
#[test]
fn key_shift_pad_latches_and_clears() {
    // SetPadMode KeyShift (assert Updated.pad_mode == KeyShift)
    // KeyShiftPadPress slot 2 -> key_shift == 2.0
    // KeyShiftPadPress slot 2 again -> key_shift == 0.0
    // KeyShiftPadPress slot 6 -> key_shift == -3.0
    // KeyShiftPadPress slot 6, shift: true -> key_shift == 0.0
}

#[test]
fn keyboard_pad_sets_scale_degree_then_release_clears() {
    // SetKeyboardScale Major
    // KeyboardPadPress slot 4 -> key_shift == 7.0
    // KeyboardPadRelease slot 4 -> key_shift == 0.0
    // KeyboardPadPress slot 0, shift: true -> keyboard_scale == Major (cycle)
}

#[test]
fn keyboard_scale_tables_match_spec() {
    assert_eq!(keyboard_scale_degrees(KeyboardScale::Major), [0,2,4,5,7,9,11,12]);
    assert_eq!(keyboard_scale_degrees(KeyboardScale::Minor), [0,2,3,5,7,8,10,12]);
    assert_eq!(keyboard_scale_degrees(KeyboardScale::Pentatonic), [0,2,4,7,9,12,14,16]);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo --manifest-path crates/Cargo.toml test -p engine-core --test bus_pad_modes_kb_keyshift`
Expected: FAIL — new variants unresolved.

- [ ] **Step 3: Implement pad modes and handlers**

`engine-api`: add `PadMode::{Keyboard, KeyShift}`; add the four `Kind` pad variants.
`engine-core`:
- `pad_press` arms `PadMode::Keyboard => self.keyboard_pad_press(deck_id, slot, shift)`, `PadMode::KeyShift => self.key_shift_pad_press(deck_id, slot, shift)`; `pad_release` analogous (`Keyboard` release clears; `KeyShift` release `Ok(())`).
- `key_shift_pad_press`: validate `slot < 8`; `shift` → `set_deck_key_shift(deck, 0.0)`; else `target = KEY_SHIFT_PAD_SEMITONES[slot] as f32`; if `control.key_shift_semitones == target` set `0.0` else `target`.
- `keyboard_pad_press`: `shift` → select scale by slot (`0→Major, 1→Minor, 2→Pentatonic`, else no-op); else `degrees = keyboard_scale_degrees(control.keyboard_scale)`, `semitones = degrees[slot.min(7)] as f32`, trigger the deck cue if present (`trigger_deck_hot_cue`/`seek`; skip if none), then `set_deck_key_shift(deck, semitones)`, then `play(deck)`.
- `keyboard_pad_release`: `set_deck_key_shift(deck, 0.0)`.
- `key_shift_pad_release`: `Ok(())`.
- Export `KEY_SHIFT_PAD_SEMITONES` and `keyboard_scale_degrees` from `engine-core` (`lib.rs` re-export).
- `control.rs`: dispatch the four new kinds to the handlers (`CmdOutcome::DeckUpdated`).

`controller`:
- `catalog.rs`: `PAD_MODES` add `"keyboard" | "key_shift"`; add leaves `keyboard_pad`, `keyboard_scale`, `key_shift_pad`, `key_shift_reset` to the valid-args match (`keyboard_pad`/`key_shift_pad` take `n` like the others; `keyboard_scale` takes `mode` in `major|minor|pentatonic`; `key_shift_reset` takes no args).
- `action.rs`: `pad_mode` match adds `"keyboard" => PadMode::Keyboard, "key_shift" => PadMode::KeyShift`; `resolve_pad_slot` adds `Keyboard` → named press/release kinds, `KeyShift` → named press/release; `"keyboard_scale"` → `CmdBody::SetKeyboardScale`; `"key_shift_reset"` → `CmdBody::SetKeyShift { semitones: 0.0 }`.

- [ ] **Step 4: Run to verify pass**

Run: `cargo --manifest-path crates/Cargo.toml test -p engine-core --test bus_pad_modes_kb_keyshift -p controller`
Expected: PASS.

- [ ] **Step 5: Add the controller action test**

In `crates/controller/src/action.rs` tests:

```rust
#[test]
fn key_shift_pad_resolves_to_named_kind() {
    let snap = ControlSnapshot::default();
    let a = resolve_action("Deck(_)::key_shift_pad(n:2)", "deck_1",
        ControlValue::Absolute(1.0), true, false, &snap).unwrap();
    assert!(matches!(a, RoutedAction::EngineCmd { kind: Kind::KeyShiftPadPress, .. }));
}

#[test]
fn keyboard_scale_and_reset_resolve() {
    let snap = ControlSnapshot::default();
    let scale = resolve_action("Deck(_)::keyboard_scale(mode:minor)", "deck_1",
        ControlValue::Absolute(1.0), true, false, &snap).unwrap();
    assert!(matches!(scale, RoutedAction::EngineCmd { body: CmdBody::SetKeyboardScale { .. }, .. }));
    let reset = resolve_action("Deck(_)::key_shift_reset", "deck_1",
        ControlValue::Absolute(1.0), true, false, &snap).unwrap();
    assert!(matches!(reset, RoutedAction::EngineCmd { body: CmdBody::SetKeyShift { semitones }, .. } if semitones == 0.0));
}
```

- [ ] **Step 6: Run to verify pass**

Run: `cargo --manifest-path crates/Cargo.toml test -p controller`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/engine-api crates/engine-core crates/controller
git commit -m "feat(deck): Keyboard and Key Shift pad modes (#298)"
```

---

### Task 4: DDJ-400 mapping, session LEDs, host mirror

**Files:**
- Modify: `mappings/ddj-400/device.toml`
- Modify: `mappings/ddj-400/map.toml`
- Modify: `crates/controller/src/session.rs`
- Modify: `crates/controller/src/engine.rs` (`ControllerEngine::set_deck_key_shift`)
- Modify: `crates/host-flutter/src/api/controller.rs` (mirror `key_shift`/`keyboard_scale`)
- Modify: `docs/ddj-400-hardware-checklist.md`
- Test: `crates/controller/tests/ddj400_kb_keyshift.rs` (new; model on `crates/controller/tests/session_output.rs`)

**Interfaces:**
- Consumes: Task 3 leaves; `EvtBody::DeckUpdated { key_shift, keyboard_scale, .. }`.
- Produces: `MappingSession::set_deck_key_shift(deck: u16, semitones: f32, midi: &mut impl MidiOut)`; `ControllerEngine::set_deck_key_shift(deck: u16, semitones: f32)`.

- [ ] **Step 1: Write the failing controller test**

`crates/controller/tests/ddj400_kb_keyshift.rs`: load the shipped `mappings/ddj-400` bundle into a `MappingSession` with a fake `MidiOut` capturing bytes (follow `session_output.rs`); then

```rust
#[test]
fn keyboard_and_key_shift_banks_dispatch() {
    // handle MIDI 0x97 0x40 0x7F -> KeyShiftPadPress? no: keyboard_pad(n:1)
    // assert published Kind::KeyboardPadPress
}

#[test]
fn key_shift_pad_lights_led_and_reset_clears() {
    // handle 0x97 0x70 0x7F (key_shift_pad n:1 -> semitones 0)
    // then session.set_deck_key_shift(0, 2.0, &mut midi) -> LED for pad 3 on
    // session.set_deck_key_shift(0, 0.0, &mut midi) -> pad 3 off
}

#[test]
fn mode_buttons_set_pad_mode() {
    // handle 0x90 0x69 0x7F -> Kind::SetPadMode with mode Keyboard
    // handle 0x90 0x6F 0x7F -> Kind::SetPadMode with mode KeyShift
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo --manifest-path crates/Cargo.toml test -p controller --test ddj400_kb_keyshift`
Expected: FAIL — bundle load validation (missing aliases / validation errors).

- [ ] **Step 3: Add device + map bindings**

`device.toml` for **each** of `deck_1`/`deck_2`:
- Inputs: `pad_mode_keyboard` (deck channel, `note = 0x69`), `pad_mode_key_shift` (`note = 0x6F`); `keyboard_pad_1..8` (pad channel, `0x40..0x47`), `keyboard_scale_1..3` (pad channel + 1, `0x40..0x42`), `key_shift_pad_1..8` (pad channel, `0x70..0x77`), `key_shift_reset_1..8` (pad channel + 1, `0x70..0x77`).
- Outputs: `pad_mode_keyboard_led`, `pad_mode_key_shift_led` (deck channel, same notes, `direction = "out"`, `velocity = 0x7F`); `keyboard_pad_1_led..8_led` (pad channel, `0x40..0x47`); `key_shift_pad_1_led..8_led` (pad channel, `0x70..0x77`).
  Pad channels are `8`/`10` (deck 1/2); deck channels `1`/`2`; the "pad channel + 1" scale/reset banks are `9`/`11`.

`map.toml` for each deck:
- Inputs: mode buttons → `Deck(_)::pad_mode(mode:keyboard)` / `Deck(_)::pad_mode(mode:key_shift)`; `keyboard_pad_N` → `Deck(_)::keyboard_pad(n:N)`; `keyboard_scale_1..3` → `Deck(_)::keyboard_scale(mode:major|minor|pentatonic)`; `key_shift_pad_N` → `Deck(_)::key_shift_pad(n:N)`; `key_shift_reset_N` → `Deck(_)::key_shift_reset`.
- Outputs: `key_shift_pad_N = { on = "key_shift_pad_N_led", off = { inline note … velocity = 0x00 } }`; `pad_mode_keyboard` / `pad_mode_key_shift` mode LEDs; `keyboard_pad_N` LEDs (light while the pad's degree is active / momentary).

- [ ] **Step 4: Implement session LED + host mirror**

`session.rs`:
- `set_deck_pad_mode`: when mode is `Keyboard` or `KeyShift`, force-refresh that bank's LEDs from the mirrored state instead of `refresh_hot_cue_leds`.
- Add `set_deck_key_shift(deck, semitones, midi)`: store on the session snapshot (add `key_shift: [f32; 4]`), then light exactly the `key_shift_pad_N` alias whose `KEY_SHIFT_PAD_SEMITONES` matches and clear the others, via `apply_output_signal`.
- Add `set_deck_keyboard_scale(deck, scale, midi)` (mirror + mode LED) and `refresh_key_shift_leds(deck, midi)`.
- In the local-mirror match, handle `CmdBody::SetKeyShift` / `SetKeyboardScale` so controller-only input paths update LEDs immediately.

`crates/controller/src/engine.rs`: add `pub fn set_deck_key_shift(&mut self, deck: u16, semitones: f32)` mirroring `set_deck_pad_mode` (call the session method with each attached output).

`crates/host-flutter/src/api/controller.rs`: in `apply_engine_mirror`'s `DeckUpdated` and `EngineStatus` arms, read `key_shift` / `keyboard_scale` and call the new `ControllerEngine` methods.

- [ ] **Step 5: Run to verify pass**

Run: `cargo --manifest-path crates/Cargo.toml test -p controller`
Expected: PASS, including `ddj400_kb_keyshift`.
Then validate the shipped bundle parses: `cargo --manifest-path crates/Cargo.toml test -p controller --test integration_fake_port` (or the bundle-load test) — PASS.

- [ ] **Step 6: Commit**

```bash
git add mappings/ddj-400 crates/controller crates/host-flutter docs/ddj-400-hardware-checklist.md
git commit -m "feat(controller): DDJ-400 Keyboard and Key Shift banks + LEDs (#298)"
```

---

### Task 5: Flutter pad modes, providers, FRB

**Files:**
- Modify: `crates/host-flutter/src/api/engine.rs` (new transport methods + `EngineEvt` fields)
- Modify: `apps/gui-flutter/lib/mixer/pad_modes.dart`
- Modify: `apps/gui-flutter/lib/mixer/deck_pads_panel.dart` + `apps/gui-flutter/lib/mixer/deck_pads_host.dart`
- Create: `apps/gui-flutter/lib/mixer/pads/keyboard_pads.dart`
- Create: `apps/gui-flutter/lib/mixer/pads/key_shift_pads.dart`
- Modify: `apps/gui-flutter/lib/mixer/engine_ui.dart`, `apps/gui-flutter/lib/mixer/engine_providers.dart`
- Regenerate: `apps/gui-flutter/lib/src/rust/**` via `moon run gui-flutter:generate`
- Test: `apps/gui-flutter/test/pad_modes_test.dart`, `apps/gui-flutter/test/engine_ui_test.dart`, `apps/gui-flutter/test/keyboard_key_shift_pads_test.dart`

**Interfaces:**
- Consumes: Task 3/4 engine cmds/evts.
- Produces (Dart): `PadMode.keyboard`, `PadMode.keyShift`; `EngineEvt.keyShift` (`double?`), `EngineEvt.keyboardScale`; `EngineUiSnapshot.keyShiftFor(int)`, `keyboardScaleFor(int)`; providers `deckKeyShiftProvider`, `deckKeyboardScaleProvider`; engine methods `setKeyShift`, `setKeyboardScale`, `keyboardPadPress`, `keyboardPadRelease`, `keyShiftPadPress`, `keyShiftPadRelease`.

- [ ] **Step 1: Write the failing Dart tests**

`apps/gui-flutter/test/pad_modes_test.dart` — extend the existing cycle test:

```dart
test('cycle includes keyboard and key shift', () {
  expect(cyclePadMode(PadMode.stems, 1), PadMode.keyboard);
  expect(cyclePadMode(PadMode.keyboard, 1), PadMode.keyShift);
  expect(cyclePadMode(PadMode.keyShift, 1), PadMode.hotCue);
  expect(padModeShortLabel(PadMode.keyboard), 'Keys');
  expect(padModeShortLabel(PadMode.keyShift), 'Shift');
});
```

`apps/gui-flutter/test/engine_ui_test.dart`:

```dart
test('updated evt carries key shift and scale', () {
  final snap = applyEngineEvt(
    EngineUiSnapshot.empty,
    EngineEvt.updated(deckId: 0, keyShift: 2.0, keyboardScale: KeyboardScale.minor),
  );
  expect(snap.keyShiftFor(0), 2.0);
  expect(snap.keyboardScaleFor(0), KeyboardScale.minor);
});
```

`apps/gui-flutter/test/keyboard_key_shift_pads_test.dart`: pump `KeyShiftPads` and tap pad 2 (+1); assert `onPress(2)`; tap again; assert active highlight toggles. Pump `KeyboardPads` with a Major scale; assert the 8 labels `0,+2,+4,+5,+7,+9,+11,+12`.

- [ ] **Step 2: Run to verify failure**

Run: `cd apps/gui-flutter && flutter test test/pad_modes_test.dart test/keyboard_key_shift_pads_test.dart`
Expected: FAIL — undefined `PadMode.keyboard` / widget.

- [ ] **Step 3: Regenerate FRB and wire the Rust host**

`crates/host-flutter/src/api/engine.rs`: add transport methods publishing `CmdBody::SetKeyShift`, `SetKeyboardScale`, `KeyboardPadPress/Release`, `KeyShiftPadPress/Release`; extend `EngineEvt` and its `DeckUpdated` → `EngineEvt` mapping with `key_shift` / `keyboard_scale` (follow the existing `key_lock` field).
Run: `moon run gui-flutter:generate`
Expected: Dart bindings include the new enum + methods.

- [ ] **Step 4: Implement the Flutter surface**

- `pad_modes.dart`: `kPadModes` append `PadMode.keyboard`, `PadMode.keyShift`; labels `'Keys'`, `'Shift'`; `const kKeyShiftPadSemitones = <int>[0,1,2,3,-4,-3,-2,-1]`; `const kKeyboardScaleDegrees = { KeyboardScale.major: [0,2,4,5,7,9,11,12], … }`; `keyboardScaleShortLabel`.
- `key_shift_pads.dart`: `KeyShiftPads({ required int activeSemitones, required ValueChanged<int> onPress, bool disabled })` — 8 `PadButton`s labelled from `kKeyShiftPadSemitones`, active highlight on the matching offset, re-press clears (engine handles it).
- `keyboard_pads.dart`: `KeyboardPads({ required KeyboardScale scale, required ValueChanged<int> onPress, required ValueChanged<int> onRelease, bool disabled })` — 8 `HoldPadButton`s labelled with the scale degrees; `onBegin`/`onEnd`.
- `deck_pads_panel.dart`: constructor gains `keyShiftSemitones`, `keyboardScale`, `onKeyShiftPress`, `onKeyboardPress`, `onKeyboardRelease`; `_modeBody` adds `PadMode.keyboard` / `PadMode.keyShift`.
- `deck_pads_host.dart`: `_toEnginePadMode` cases; watch `deckKeyShiftProvider` / `deckKeyboardScaleProvider`; wire callbacks.
- `engine_ui.dart` / `engine_providers.dart`: add `keyShift` / `keyboardScale` maps + `copyWith` + providers + `applyEngineEvt` handling (mirror the `keyLocks` pattern).
- Deck key display: show the resulting key read-only (analyzed key + `keyShiftSemitones`), never persisted.

- [ ] **Step 5: Run to verify pass**

Run: `cd apps/gui-flutter && flutter test test/pad_modes_test.dart test/engine_ui_test.dart test/keyboard_key_shift_pads_test.dart`
Expected: PASS.
Then `flutter analyze` — no new issues.

- [ ] **Step 6: Commit**

```bash
git add apps/gui-flutter crates/host-flutter
git commit -m "feat(gui): Keyboard and Key Shift pad modes in Flutter (#298)"
```

---

### Task 6: Docs + full verification

**Files:**
- Modify: `docs/deck-spec.md` (§5.4 P4, §5.5 pad-mode table, §5.10, roadmap line 768)
- Modify: `docs/tech-spec.md` (remove semitone shift from MVP non-goals, or point to this design)
- Modify: `docs/ddj-400-hardware-checklist.md` if not done in Task 4

- [ ] **Step 1: Update the specs**

- deck-spec: mark P4 Key shift and the Keyboard / Key Shift pad modes as shipped, linking `docs/keyboard-key-shift-pad-modes-design.md`.
- tech-spec: change the non-goal line to note realtime key shift now exists via `stretch`'s pitch factor.
- checklist: reflect the now-bound mode buttons and banks.

- [ ] **Step 2: Run the full affected check**

Run: `cargo --manifest-path crates/Cargo.toml test -p stretch -p engine-dsp -p engine-api -p engine-core -p controller`
Expected: PASS.
Run: `cd apps/gui-flutter && flutter test`
Expected: PASS.
Run: `npx moon ci` (or the repo's `npm run lint` / `format:check`)
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add docs
git commit -m "docs(deck): ship Keyboard and Key Shift pad modes (#298)"
```

## Spec coverage

| Spec item | Task |
|-----------|------|
| Realtime pitch factor | 1 |
| PadMode + engine cmds/events + snapshot | 2, 3 |
| Key Shift layout + latch/reset | 3 |
| Keyboard layout + scale + momentary | 3 |
| Key lock / analyzed-key interaction | 2, 3, 5 |
| No persistence as manual corrections | 2, 3, 6 |
| Pitch / pad / mode-switch / state-sync tests | 1–5 |
| DDJ-400 input, mode, LED | 4 |
| Flutter UI | 5 |
| Docs | 6 |

## Execution note

Owner preference (see `docs/stems-pad-mode-plan.md`): continue to PR without per-section review; prefer inline execution over subagent handoff unless blocked.
