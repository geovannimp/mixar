# Master Bus Cue (Headphones) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Route master mix into the headphone/preview (cue) bus via Pioneer-style Master Cue + Cue/Master mix, without changing the room master bus.

**Architecture:** Add a dedicated `HeadphoneMonitor` DSP unit that blends PFL (pre-fader sum of cued playing decks) with a gated master tap (`mix_buffer` before `master_volume`). `Mixer` owns `cue_mix` / `master_cue` and invokes the monitor on the cue bus path. Engine → Tauri → mixer UI wire session controls; disabled when preview is off.

**Tech Stack:** Rust (`engine-dsp`, `engine-core`), Tauri + React (`gui-app`), existing `RotaryKnob` / mixer patterns.

**Spec:** `docs/superpowers/specs/2026-07-16-master-bus-cue-design.md`  
**Issue:** [#68](https://github.com/geovannimp/rust-dj-engine/issues/68)

## Global Constraints

- Controls: both Master Cue toggle and Cue/Master mix knob (Pioneer-style gating).
- Master tap: post-fader / post-crossfader `mix_buffer`, **before** `master_volume`.
- Defaults: `cue_mix = 0.0`, `master_cue = false`.
- Cue formula: `out = (1 - cue_mix) * pfl + cue_mix * (master_cue ? master_tap : 0)`; then × `cue_volume` + clamp.
- Session-only persistence (re-apply on engine restart like headphone cue).
- Out of scope: split cue (H3), headphone delay, volume normalizer (#67), settings TOML fields.

---

## File map

| File | Responsibility |
|------|----------------|
| `engine-dsp/src/headphone_monitor.rs` | **New** — blend PFL + gated master tap |
| `engine-dsp/src/lib.rs` | `mod headphone_monitor`; re-export if useful |
| `engine-dsp/src/mixer.rs` | `cue_mix` / `master_cue` state; cue route via monitor |
| `engine-core/src/engine.rs` | `set_cue_mix` / `set_master_cue` (+ getters) |
| `gui-app/src-tauri/src/lib.rs` | AppState fields; Tauri commands; rehydrate; `EngineStatus` |
| `gui-app/src-tauri/src/engine_controller.rs` | Include `cue_mix` / `master_cue` in status |
| `gui-app/src/types.ts` | Status fields |
| `gui-app/src/stores/engineStore.ts` | Actions + selectors |
| `gui-app/src/components/DeckMixer.tsx` | Master Cue + Cue/Master UI |

---

### Task 1: `HeadphoneMonitor` unit (TDD)

**Files:**
- Create: `engine-dsp/src/headphone_monitor.rs`
- Modify: `engine-dsp/src/lib.rs`

**Interfaces:**
- Produces:
  - `HeadphoneMonitor::render(pfl: &[f32], master_tap: &[f32], cue_mix: f32, master_cue: bool, out: &mut [f32])`
  - Writes `out[i] = (1.0 - cue_mix) * pfl[i] + cue_mix * (if master_cue { master_tap[i] } else { 0.0 })`
  - Lengths: process `min(pfl.len(), master_tap.len(), out.len())`; remaining `out` samples left unchanged or zeroed by caller (caller zeros `out` first)

- [ ] **Step 1: Write failing tests**

Create `engine-dsp/src/headphone_monitor.rs`:

```rust
//! Headphone / preview bus monitor: blend PFL with gated master tap.

use audio_core::Sample;

/// Blends pre-fader listen (PFL) with an optional master tap for the cue bus.
#[derive(Debug, Default, Clone, Copy)]
pub struct HeadphoneMonitor;

impl HeadphoneMonitor {
    /// Render interleaved stereo (or any sample buffer) into `out`.
    ///
    /// `cue_mix`: 0.0 = PFL only, 1.0 = master tap only (when `master_cue`).
    /// When `master_cue` is false, the master contribution is silence.
    pub fn render(
        pfl: &[Sample],
        master_tap: &[Sample],
        cue_mix: f32,
        master_cue: bool,
        out: &mut [Sample],
    ) {
        let _ = (pfl, master_tap, cue_mix, master_cue, out);
        unimplemented!("HeadphoneMonitor::render");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pfl_only_when_mix_zero() {
        let pfl = [1.0_f32, -1.0];
        let master = [0.5_f32, 0.5];
        let mut out = [0.0; 2];
        HeadphoneMonitor::render(&pfl, &master, 0.0, true, &mut out);
        assert!((out[0] - 1.0).abs() < 1e-6);
        assert!((out[1] + 1.0).abs() < 1e-6);
    }

    #[test]
    fn master_only_when_mix_one_and_master_cue_on() {
        let pfl = [1.0_f32, 1.0];
        let master = [0.25_f32, -0.25];
        let mut out = [0.0; 2];
        HeadphoneMonitor::render(&pfl, &master, 1.0, true, &mut out);
        assert!((out[0] - 0.25).abs() < 1e-6);
        assert!((out[1] + 0.25).abs() < 1e-6);
    }

    #[test]
    fn master_gated_off_when_master_cue_false() {
        let pfl = [1.0_f32, 1.0];
        let master = [0.9_f32, 0.9];
        let mut out = [0.0; 2];
        HeadphoneMonitor::render(&pfl, &master, 1.0, false, &mut out);
        assert!(out[0].abs() < 1e-6);
        assert!(out[1].abs() < 1e-6);
    }

    #[test]
    fn mid_blend_with_master_cue() {
        let pfl = [1.0_f32, 0.0];
        let master = [0.0_f32, 0.0];
        let mut out = [0.0; 2];
        HeadphoneMonitor::render(&pfl, &master, 0.5, true, &mut out);
        assert!((out[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn master_cue_off_mix_attenuates_pfl_only() {
        let pfl = [1.0_f32, 1.0];
        let master = [1.0_f32, 1.0];
        let mut out = [0.0; 2];
        HeadphoneMonitor::render(&pfl, &master, 0.5, false, &mut out);
        assert!((out[0] - 0.5).abs() < 1e-6);
        assert!((out[1] - 0.5).abs() < 1e-6);
    }
}
```

Add to `engine-dsp/src/lib.rs`:

```rust
pub mod headphone_monitor;
pub use headphone_monitor::HeadphoneMonitor;
```

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cargo test -p engine-dsp headphone_monitor -- --nocapture`  
Expected: FAIL (`unimplemented!` or link errors)

- [ ] **Step 3: Implement `render`**

```rust
pub fn render(
    pfl: &[Sample],
    master_tap: &[Sample],
    cue_mix: f32,
    master_cue: bool,
    out: &mut [Sample],
) {
    let mix = cue_mix.clamp(0.0, 1.0);
    let n = out.len().min(pfl.len()).min(master_tap.len());
    for i in 0..n {
        let master = if master_cue { master_tap[i] } else { 0.0 };
        out[i] = (1.0 - mix) * pfl[i] + mix * master;
    }
}
```

- [ ] **Step 4: Run tests — expect PASS**

Run: `cargo test -p engine-dsp headphone_monitor -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine-dsp/src/headphone_monitor.rs engine-dsp/src/lib.rs
git commit -m "Add HeadphoneMonitor blend unit for cue bus."
```

---

### Task 2: Mixer state + cue routing via monitor (TDD)

**Files:**
- Modify: `engine-dsp/src/mixer.rs`

**Interfaces:**
- Consumes: `HeadphoneMonitor::render`
- Produces:
  - `Mixer::cue_mix(&self) -> f32` (default `0.0`)
  - `Mixer::set_cue_mix(&mut self, mix: f32) -> Result<()>` — err if outside `0.0..=1.0`
  - `Mixer::master_cue(&self) -> bool` (default `false`)
  - `Mixer::set_master_cue(&mut self, enabled: bool)`
  - Cue bus filled by summing PFL then `HeadphoneMonitor::render(pfl, mix_buffer, …)` into cue buffer **before** applying `cue_volume`

- [ ] **Step 1: Write failing tests**

Add to `mixer.rs` `#[cfg(test)]` (reuse `load_test_tone`):

```rust
#[test]
fn cue_mix_and_master_cue_defaults() {
    let mixer = Mixer::new();
    assert_eq!(mixer.cue_mix(), 0.0);
    assert!(!mixer.master_cue());
}

#[test]
fn set_cue_mix_rejects_out_of_range() {
    let mut mixer = Mixer::new();
    assert!(mixer.set_cue_mix(-0.1).is_err());
    assert!(mixer.set_cue_mix(1.1).is_err());
    mixer.set_cue_mix(0.5).unwrap();
    assert_eq!(mixer.cue_mix(), 0.5);
}

#[test]
fn master_cue_on_mix_one_hears_master_without_pfl() {
    let mut mixer = Mixer::new();
    mixer.set_master_cue(true);
    mixer.set_cue_mix(1.0).unwrap();
    mixer.set_master_volume(0.25).unwrap(); // room quieter; cue tap must ignore this
    mixer.set_cue_volume(1.0).unwrap();

    let mut decks = vec![Deck::new(0, 48000, 512, "medium")];
    load_test_tone(&mut decks[0]);
    decks[0].play().unwrap();
    // no headphone_cue

    let mut output_buses = HashMap::new();
    output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
    output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
    mixer.process(&mut decks, 512, &mut output_buses).unwrap();

    let cue_max = output_buses[&BusId::new("cue")]
        .iter()
        .map(|&s| s.abs())
        .fold(0.0_f32, f32::max);
    let master_max = output_buses[&BusId::new("master")]
        .iter()
        .map(|&s| s.abs())
        .fold(0.0_f32, f32::max);
    assert!(cue_max > 0.1, "cue should carry master tap, got {}", cue_max);
    assert!(
        cue_max > master_max + 0.05,
        "cue tap must be pre master_volume (cue {}, master {})",
        cue_max,
        master_max
    );
}

#[test]
fn master_cue_off_mix_one_stays_silent_without_pfl() {
    let mut mixer = Mixer::new();
    mixer.set_master_cue(false);
    mixer.set_cue_mix(1.0).unwrap();

    let mut decks = vec![Deck::new(0, 48000, 512, "medium")];
    load_test_tone(&mut decks[0]);
    decks[0].play().unwrap();

    let mut output_buses = HashMap::new();
    output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
    output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
    mixer.process(&mut decks, 512, &mut output_buses).unwrap();

    let cue_max = output_buses[&BusId::new("cue")]
        .iter()
        .map(|&s| s.abs())
        .fold(0.0_f32, f32::max);
    assert!(cue_max < 1e-6, "no master bleed when Master Cue off, got {}", cue_max);
}
```

Keep existing `cue_bus_silent_when_no_headphone_cue` and `cue_bus_sums_pre_fader_when_cued` passing (defaults = old behavior).

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cargo test -p engine-dsp cue_mix_ master_cue_ -- --nocapture`  
Expected: FAIL (methods missing)

- [ ] **Step 3: Add mixer fields + setters**

On `Mixer`:

```rust
cue_mix: f32,      // default 0.0
master_cue: bool,  // default false
```

```rust
pub fn cue_mix(&self) -> f32 { self.cue_mix }
pub fn set_cue_mix(&mut self, mix: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&mix) {
        return Err(anyhow::anyhow!("Cue mix must be between 0.0 and 1.0"));
    }
    self.cue_mix = mix;
    Ok(())
}
pub fn master_cue(&self) -> bool { self.master_cue }
pub fn set_master_cue(&mut self, enabled: bool) { self.master_cue = enabled; }
```

Update `Debug` impl fields.

- [ ] **Step 4: Route cue through `HeadphoneMonitor`**

In `route_to_buses`, replace the cue branch:

```rust
"cue" => {
    // 1) Build PFL sum into a scratch buffer (or reuse output_buffer then blend).
    let mut pfl = vec![0.0; required_size];
    for deck in decks {
        if deck.headphone_cue() && deck.state() == &DeckState::Playing {
            let pre_fader = deck.pre_fader_buffer();
            for (i, &sample) in pre_fader.iter().enumerate() {
                if i < pfl.len() {
                    pfl[i] += sample;
                }
            }
        }
    }
    output_buffer.fill(0.0);
    crate::HeadphoneMonitor::render(
        &pfl,
        &self.mix_buffer,
        self.cue_mix,
        self.master_cue,
        output_buffer,
    );
}
```

Prefer a reusable `pfl_scratch: Vec<Sample>` field on `Mixer` (resize each call) to avoid allocating in `process` — add `pfl_scratch` in the same step if easy; if not, allocate once in Step 4 then follow-up micro-optimize only if needed for tests.

Do **not** change the master branch or volume/clamp loop after the match.

- [ ] **Step 5: Run tests — expect PASS**

Run: `cargo test -p engine-dsp -- --nocapture`  
Expected: PASS (all mixer + headphone_monitor tests)

- [ ] **Step 6: Commit**

```bash
git add engine-dsp/src/mixer.rs
git commit -m "Route cue bus through HeadphoneMonitor with Master Cue mix."
```

---

### Task 3: Engine API

**Files:**
- Modify: `engine-core/src/engine.rs`
- Test: `engine-core/tests/integration_tests.rs` (optional light smoke) or unit-style via existing null start

**Interfaces:**
- Consumes: `Mixer::set_cue_mix`, `Mixer::set_master_cue`, getters
- Produces:
  - `Engine::set_cue_mix(&mut self, mix: f32) -> Result<()>`
  - `Engine::cue_mix(&self) -> Option<f32>`
  - `Engine::set_master_cue(&mut self, enabled: bool) -> Result<()>`
  - `Engine::master_cue(&self) -> Option<bool>`

- [ ] **Step 1: Add methods next to `set_crossfader`**

```rust
pub fn set_cue_mix(&mut self, mix: f32) -> Result<()> {
    let dsp_engine = self
        .dsp_engine
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
    let mut dsp = dsp_engine.lock().unwrap();
    dsp.mixer_mut().set_cue_mix(mix)
}

pub fn cue_mix(&self) -> Option<f32> {
    let dsp_engine = self.dsp_engine.as_ref()?;
    let dsp = dsp_engine.lock().ok()?;
    Some(dsp.mixer().cue_mix())
}

pub fn set_master_cue(&mut self, enabled: bool) -> Result<()> {
    let dsp_engine = self
        .dsp_engine
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
    let mut dsp = dsp_engine.lock().unwrap();
    dsp.mixer_mut().set_master_cue(enabled);
    Ok(())
}

pub fn master_cue(&self) -> Option<bool> {
    let dsp_engine = self.dsp_engine.as_ref()?;
    let dsp = dsp_engine.lock().ok()?;
    Some(dsp.mixer().master_cue())
}
```

- [ ] **Step 2: Smoke in integration test**

In `engine-core/tests/integration_tests.rs`, extend `starts_with_master_and_cue_buses_on_null` (or add a sibling test):

```rust
engine.set_master_cue(true).expect("master cue");
engine.set_cue_mix(1.0).expect("cue mix");
assert_eq!(engine.master_cue(), Some(true));
assert_eq!(engine.cue_mix(), Some(1.0));
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p engine-core starts_with_master_and_cue -- --nocapture`  
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add engine-core/src/engine.rs engine-core/tests/integration_tests.rs
git commit -m "Expose cue mix and master cue on Engine."
```

---

### Task 4: Tauri commands + session rehydrate

**Files:**
- Modify: `gui-app/src-tauri/src/lib.rs`
- Modify: `gui-app/src-tauri/src/engine_controller.rs`
- Modify: `gui-app/src/types.ts`

**Interfaces:**
- Produces:
  - `AppState.cue_mix: f32` (default `0.0`), `AppState.master_cue: bool` (default `false`)
  - `EngineStatus.cue_mix: f32`, `EngineStatus.master_cue: bool`
  - Commands `set_cue_mix(cueMix: f32)`, `set_master_cue(enabled: bool)`
  - `reapply_mixer_state` / `start_engine` path re-applies both after crossfader

- [ ] **Step 1: Extend `AppState` + status**

In `AppState`:

```rust
pub cue_mix: f32,
pub master_cue: bool,
```

Initialize both where `crossfader: 0.5` is set today (`0.0` / `false`).

In the status struct serialized to the UI (same place as `crossfader`):

```rust
cue_mix: f32,
master_cue: bool,
```

Map from `state.cue_mix` / `state.master_cue` in `engine_controller` (and any other status builders).

In `gui-app/src/types.ts` `EngineStatus`:

```ts
cue_mix: number;
master_cue: boolean;
```

- [ ] **Step 2: Add Tauri commands**

Mirror `set_crossfader`:

```rust
fn set_cue_mix(cue_mix: f32, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    if !(0.0..=1.0).contains(&cue_mix) {
        return Err("Cue mix must be between 0.0 and 1.0".into());
    }
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.cue_mix = cue_mix;
    if let Some(engine) = state.engine.as_mut() {
        engine.set_cue_mix(cue_mix).map_err(|e| e.to_string())?;
    }
    // bump revision / notify if crossfader does
    Ok(())
}

fn set_master_cue(enabled: bool, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.master_cue = enabled;
    if let Some(engine) = state.engine.as_mut() {
        engine.set_master_cue(enabled).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

Register in `invoke_handler`. Match existing revision/notify patterns from `set_crossfader` exactly (copy that function’s notify/revision lines).

- [ ] **Step 3: Rehydrate on engine start**

In the helper that re-applies crossfader after start (near `engine.set_crossfader(crossfader)`):

```rust
engine.set_cue_mix(state.cue_mix).map_err(|e| e.to_string())?;
engine.set_master_cue(state.master_cue).map_err(|e| e.to_string())?;
```

- [ ] **Step 4: Commit**

```bash
git add gui-app/src-tauri/src/lib.rs gui-app/src-tauri/src/engine_controller.rs gui-app/src/types.ts
git commit -m "Wire cue mix and master cue through Tauri session state."
```

---

### Task 5: GUI — Master Cue + Cue/Master knob

**Files:**
- Modify: `gui-app/src/stores/engineStore.ts`
- Modify: `gui-app/src/components/DeckMixer.tsx`
- Optionally: `gui-app/src/hooks/useSettings.ts` (read `preview_enabled` for disable)

**Interfaces:**
- Consumes: `set_cue_mix`, `set_master_cue` invokes; `EngineStatus.cue_mix` / `master_cue`; `AppSettings.preview_enabled`
- Produces: mixer UI controls under/near crossfader

- [ ] **Step 1: Store actions**

In `engineStore.ts` (mirror `setCrossfader`):

```ts
setCueMix: async (mix: number) => {
  await invoke("set_cue_mix", { cueMix: mix });
},
setMasterCue: async (enabled: boolean) => {
  await invoke("set_master_cue", { enabled });
},
```

Export helpers / hooks:

```ts
export function useCueMix(): number {
  return useEngineStore((s) => s.status?.cue_mix ?? 0);
}
export function useMasterCue(): boolean {
  return useEngineStore((s) => s.status?.master_cue ?? false);
}
```

Ensure status merge from engine events keeps the new fields (same path as `crossfader`).

- [ ] **Step 2: Mixer UI**

In `DeckMixer.tsx`, below `Crossfader` (still inside the mixer column):

```tsx
{/* Headphone monitor: Master Cue + Cue/Master mix */}
<div className="flex shrink-0 flex-col items-center gap-1 border-t border-white/6 pt-2">
  <button
    type="button"
    disabled={!previewEnabled}
    aria-pressed={masterCue}
    aria-label="Master cue"
    title="Master cue"
    className={cn(
      buttonIcon,
      "h-6 w-full text-[9px] font-semibold uppercase tracking-wide",
      masterCue ? "bg-amber-500/20 text-amber-300" : "text-zinc-500",
    )}
    onClick={() => onMasterCueChange(!masterCue)}
  >
    Master Cue
  </button>
  <MixerKnob
    label="Cue/Mst"
    value={cueMix}
    min={0}
    max={1}
    step={0.01}
    disabled={!previewEnabled}
    onChange={onCueMixChange}
  />
</div>
```

Adapt `MixerKnob` props to match the existing `RotaryKnob` wrapper in this file (label, value range). If `MixerKnob` is EQ-oriented (±dB), add a small local `CueMixKnob` using `RotaryKnob` with `min={0} max={1}` and aria-label `"Cue master mix"`.

Wire in `DeckMixer()` container:

```ts
const cueMix = useCueMix();
const masterCue = useMasterCue();
const { settings } = useSettings();
const previewEnabled = settings?.preview_enabled ?? false;
// pass to view + onCueMixChange / onMasterCueChange → engineActions
```

- [ ] **Step 3: Manual check**

With `npm run tauri dev` and preview enabled in settings:
1. No deck PFL, Master Cue off → headphones silent (or no cue device if preview off — controls disabled).
2. Master Cue on, Cue/Mst fully Master → hear master mix in preview device.
3. Deck PFL on, mix at Cue, Master Cue off → PFL only (unchanged).
4. Master volume down → headphone master tap level unchanged; cue volume still scales phones.

- [ ] **Step 4: Commit**

```bash
git add gui-app/src/stores/engineStore.ts gui-app/src/components/DeckMixer.tsx
git commit -m "Add Master Cue and Cue/Master controls to mixer UI."
```

---

### Task 6: Spec checklist + issue note

**Files:**
- Modify: none required; optionally comment on [#68](https://github.com/geovannimp/rust-dj-engine/issues/68)

- [ ] **Step 1: Verify acceptance from spec**

Re-run:

```bash
cargo test -p engine-dsp -- --nocapture
cargo test -p engine-core starts_with_master_and_cue -- --nocapture
```

Expected: PASS

Manually confirm the five acceptance bullets in the design spec.

- [ ] **Step 2: Commit any leftover docs only if needed**

If `docs/deck-spec.md` H2 should note “implemented”, update one line and commit; otherwise skip.

```bash
# optional
git commit -m "Note Master Cue / cue mix implemented for deck-spec H2."
```

---

## Spec coverage (self-review)

| Spec requirement | Task |
|------------------|------|
| Master Cue + Cue/Master both | 2, 5 |
| Pioneer gating (no master bleed when off) | 1, 2 |
| Tap before `master_volume` | 2 (`master_cue_on_mix_one_hears_master_without_pfl`) |
| Defaults 0 / false | 2, 4 |
| `HeadphoneMonitor` unit | 1 |
| Engine setters | 3 |
| Tauri + session rehydrate | 4 |
| GUI + disable when preview off | 5 |
| Unit tests matrix | 1–2 |
| #67 out of scope | — |

No TBD placeholders; method names consistent: `cue_mix` / `master_cue` / `set_cue_mix` / `set_master_cue` across dsp → engine → Tauri (`cueMix` camelCase in invoke) → TS.
