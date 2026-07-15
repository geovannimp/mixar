# Multi-Bus Audio Routing (Main + Preview) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Honor master + optional preview (`cue`) bus device/channel maps with real multi-stream I/O, and route per-deck headphone cue (PFL) pre-fader audio to the cue bus; settings apply restarts the engine.

**Architecture:** Reuse `engine-core/src/routing.rs` (`resolve_device_stream_plans` + `map_buses_to_device_buffer`). On start, open one stream + ring per device plan; producer fills stereo `master`/`cue` buses then maps into each device ring (paced by the master plan). DSP keeps a pre-fader buffer and sums cued decks onto `cue`. GUI wires headphones + save_settings stop/apply/start.

**Tech Stack:** Rust (`engine-dsp`, `engine-core`, `backend-cpal`, `backend-null`), Tauri + React (`gui-app`).

**Spec:** `docs/superpowers/specs/2026-07-15-multi-bus-routing-design.md`

## Global Constraints

- Bus ids: `master` and `cue` (UI label “Preview”; `PREVIEW_BUS_ID = "cue"`).
- Channel indexes are 1-based stereo pairs; no overlaps on the same device.
- `set_bus_device` updates config only; live audio changes via engine restart/start.
- Settings save while running: stop → apply → start (rehydrate decks).
- Out of scope: cue-mix knob, split cue, cross-device clock sync, master/PFL meters.

---

## File map

| File | Responsibility |
|------|----------------|
| `engine-core/src/lib.rs` | Declare `mod routing` (file already exists with tests) |
| `engine-dsp/src/deck.rs` | `headphone_cue`, `pre_fader_buffer`, accessors |
| `engine-dsp/src/mixer.rs` | Cue bus = sum of cued pre-fader decks |
| `engine-core/src/engine.rs` | Multi-stream start/stop; `set_bus_device`; `set_deck_headphone_cue` |
| `engine-core/src/producer.rs` | Multi-plan rings; map buses → device buffers |
| `backend-cpal/src/lib.rs` | Prefer `params.channels` when selecting config |
| `engine-core/tests/integration_tests.rs` | Master+cue null-backend start |
| `gui-app/src-tauri/src/lib.rs` | Settings restart; cue command; status field |
| `gui-app/src-tauri/src/engine_controller.rs` | Include `headphone_cue` in `DeckStatus` |
| `gui-app/src/types.ts` | `headphone_cue` on `DeckStatus` |
| `gui-app/src/stores/engineStore.ts` | `setDeckHeadphoneCue` |
| `gui-app/src/components/DeckMixer.tsx` | Wire headphones to engine (drop Phase 4 tooltip) |
| `README.md` | Remove “set_bus_device stub” note when done |

---

### Task 1: Wire `routing` into `engine-core`

**Files:**
- Modify: `engine-core/src/lib.rs`
- Test: existing tests in `engine-core/src/routing.rs`

**Interfaces:**
- Consumes: `engine-core/src/routing.rs` (already present)
- Produces: `mod routing` compiled; helpers available as `crate::routing::*`

- [ ] **Step 1: Declare the module**

In `engine-core/src/lib.rs`, add `mod routing;` alongside the other `mod` lines (after `producer` is fine):

```rust
mod backend;
mod callback;
mod config;
mod engine;
mod producer;
mod routing;
mod transport;
```

- [ ] **Step 2: Run routing tests**

Run: `cargo test -p engine-core routing -- --nocapture`  
Expected: PASS (existing tests in `routing.rs`)

- [ ] **Step 3: Commit**

```bash
git add engine-core/src/lib.rs
git commit -m "Wire routing module into engine-core for bus mapping."
```

---

### Task 2: Deck headphone cue + pre-fader buffer (TDD)

**Files:**
- Modify: `engine-dsp/src/deck.rs`
- Test: `engine-dsp/src/deck.rs` (`#[cfg(test)]`)

**Interfaces:**
- Produces:
  - `Deck::set_headphone_cue(&mut self, enabled: bool)`
  - `Deck::headphone_cue(&self) -> bool`
  - `Deck::pre_fader_buffer(&self) -> &[Sample]` — last process() buffer after trim/EQ/filter, before volume

- [ ] **Step 1: Write failing tests**

Add to `deck.rs` tests (reuse existing tone-load helpers if present):

```rust
#[test]
fn headphone_cue_defaults_false_and_toggles() {
    let mut deck = Deck::new(0, 48000, 512, "medium");
    assert!(!deck.headphone_cue());
    deck.set_headphone_cue(true);
    assert!(deck.headphone_cue());
}

#[test]
fn pre_fader_buffer_ignores_volume() {
    let mut deck = Deck::new(0, 48000, 512, "medium");
    // load a constant 0.5 interleaved tone (same pattern as levels_measure_pre_volume)
    // set_volume(0.0); play(); process(frames);
    let pre = deck.pre_fader_buffer();
    assert!(!pre.is_empty());
    let peak = pre.iter().copied().map(f32::abs).fold(0.0_f32, f32::max);
    assert!(
        peak > 0.4,
        "pre-fader should stay audible when volume is 0, got {}",
        peak
    );
}
```

Fill in load/play mirroring `levels_measure_pre_volume` in the same file.

- [ ] **Step 2: Run tests — expect fail**

Run: `cargo test -p engine-dsp headphone_cue_defaults -- --nocapture`  
Expected: FAIL (method missing)

- [ ] **Step 3: Implement on `Deck`**

```rust
// fields
headphone_cue: bool,
pre_fader_buffer: Vec<Sample>,

// in new()/Default-like init
headphone_cue: false,
pre_fader_buffer: Vec::new(),

pub fn set_headphone_cue(&mut self, enabled: bool) {
    self.headphone_cue = enabled;
}

pub fn headphone_cue(&self) -> bool {
    self.headphone_cue
}

pub fn pre_fader_buffer(&self) -> &[Sample] {
    &self.pre_fader_buffer
}
```

In `process`, after filter + `level_peaks`, before volume:

```rust
self.level_peaks = LevelPeaks::from_buffer(&self.buffer);
self.pre_fader_buffer.resize(self.buffer.len(), 0.0);
self.pre_fader_buffer.copy_from_slice(&self.buffer);
for sample in &mut self.buffer {
    *sample *= self.volume;
}
```

- [ ] **Step 4: Run tests — expect pass**

Run: `cargo test -p engine-dsp pre_fader_buffer_ignores_volume headphone_cue_defaults -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine-dsp/src/deck.rs
git commit -m "Retain pre-fader buffer and headphone cue flag on decks."
```

---

### Task 3: Mixer routes cue as PFL sum (TDD)

**Files:**
- Modify: `engine-dsp/src/mixer.rs`
- Test: `engine-dsp/src/mixer.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: `Deck::headphone_cue()`, `Deck::pre_fader_buffer()`
- Produces: `cue` bus contents = sum of cued pre-fader decks × `cue_volume` (silence if none)

- [ ] **Step 1: Write failing / update tests**

Replace/update tests that assume cue == scaled master. Add:

```rust
#[test]
fn cue_bus_silent_when_no_headphone_cue() {
    let mut mixer = Mixer::new();
    let mut decks = vec![Deck::new(0, 48000, 512, "medium")];
    load_test_tone(&mut decks[0]);
    decks[0].play().unwrap();
    // headphone_cue stays false

    let mut output_buses = HashMap::new();
    output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
    output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
    mixer.process(&mut decks, 512, &mut output_buses).unwrap();

    let cue_max = output_buses[&BusId::new("cue")]
        .iter()
        .map(|&s| s.abs())
        .fold(0.0_f32, f32::max);
    assert!(cue_max < 1e-6, "cue should be silent, got {}", cue_max);
    assert!(output_buses[&BusId::new("master")]
        .iter()
        .any(|&s| s != 0.0));
}

#[test]
fn cue_bus_sums_pre_fader_when_cued() {
    let mut mixer = Mixer::new();
    let mut decks = vec![Deck::new(0, 48000, 512, "medium")];
    load_test_tone(&mut decks[0]);
    decks[0].set_volume(0.0).unwrap(); // master silent
    decks[0].set_headphone_cue(true);
    decks[0].play().unwrap();

    let mut output_buses = HashMap::new();
    output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
    output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
    mixer.process(&mut decks, 512, &mut output_buses).unwrap();

    let master_max = output_buses[&BusId::new("master")]
        .iter()
        .map(|&s| s.abs())
        .fold(0.0_f32, f32::max);
    let cue_max = output_buses[&BusId::new("cue")]
        .iter()
        .map(|&s| s.abs())
        .fold(0.0_f32, f32::max);
    assert!(master_max < 1e-6);
    assert!(cue_max > 0.1, "cued pre-fader should reach cue bus, got {}", cue_max);
}
```

- [ ] **Step 2: Run — expect fail**

Run: `cargo test -p engine-dsp cue_bus_silent -- --nocapture`  
Expected: FAIL (cue currently gets the mix)

- [ ] **Step 3: Implement cue routing**

Change `route_to_buses` (or `process` after the graph) so:

1. Fill **master** from `mix_buffer` × `master_volume` (unchanged).
2. If `output_buses` contains `BusId("cue")`:
   - zero the cue buffer
   - for each deck with `headphone_cue()`, add `pre_fader_buffer()` sample-wise
   - apply `cue_volume` + clamp (reuse existing clamp path)

Do **not** copy `mix_buffer` into cue.

Signature option — keep `route_to_buses` and pass `&[Deck]`:

```rust
fn route_to_buses(
    &self,
    frames: u32,
    decks: &[Deck],
    output_buses: &mut HashMap<BusId, Vec<Sample>>,
) -> Result<()>
```

Call it from `process` after the graph with `&decks` (after deck `process` already filled pre-fader buffers).

- [ ] **Step 4: Run mixer tests**

Run: `cargo test -p engine-dsp --lib mixer -- --nocapture`  
Expected: PASS (update any remaining master≈cue assertions)

- [ ] **Step 5: Commit**

```bash
git add engine-dsp/src/mixer.rs
git commit -m "Route cue bus from headphone-cued pre-fader decks."
```

---

### Task 4: Implement `set_bus_device` + harden `update_bus_config`

**Files:**
- Modify: `engine-core/src/engine.rs`
- Test: `engine-core/src/engine.rs` (`#[cfg(test)]`) and/or extend routing tests

**Interfaces:**
- Consumes: `crate::routing::{validate_channel_pair, resolve_device_id, ensure_channels_in_range, ensure_no_channel_conflicts, DEFAULT_DEVICE_ID}`
- Produces: `Engine::set_bus_device(bus, device, channels) -> Result<()>` updates `config.buses` (insert master/cue if missing)

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn set_bus_device_updates_master_config() {
    let mut config = EngineConfig::default();
    config.backend = "null".into();
    let mut engine = Engine::new(config).unwrap();
    engine
        .set_bus_device(
            BusId::new("master"),
            DeviceId::new("null-device"),
            [3, 4],
        )
        .unwrap();
    let bus = engine.get_bus_config(&BusId::new("master")).unwrap();
    assert_eq!(bus.channels.left, 3);
    assert_eq!(bus.channels.right, 4);
    assert_eq!(bus.device.as_str(), "null-device");
}

#[test]
fn set_bus_device_rejects_overlap_on_same_device() {
    let mut config = EngineConfig::default();
    config.backend = "null".into();
    config.buses = vec![
        BusConfig::new(
            BusId::new("master"),
            "Master".into(),
            DeviceId::new("null-device"),
            ChannelMapping::new(1, 2),
        ),
        BusConfig::new(
            BusId::new("cue"),
            "Preview".into(),
            DeviceId::new("null-device"),
            ChannelMapping::new(3, 4),
        ),
    ];
    let mut engine = Engine::new(config).unwrap();
    let err = engine
        .set_bus_device(BusId::new("cue"), DeviceId::new("null-device"), [2, 3])
        .unwrap_err();
    assert!(err.to_string().contains("overlaps"));
}
```

- [ ] **Step 2: Run — expect fail**

Run: `cargo test -p engine-core set_bus_device_updates_master -- --nocapture`  
Expected: FAIL (stub returns Ok without writing)

- [ ] **Step 3: Implement**

```rust
pub fn set_bus_device(
    &mut self,
    bus: BusId,
    device: DeviceId,
    channels: [u16; 2],
) -> Result<()> {
    let mapping = crate::routing::validate_channel_pair(channels)?;
    let devices = self.backend.list_output_devices()?;
    let resolved = crate::routing::resolve_device_id(&device, &devices)?;
    let info = devices
        .iter()
        .find(|d| d.id == resolved)
        .ok_or_else(|| anyhow::anyhow!("Output device not found: {}", resolved.as_str()))?;
    crate::routing::ensure_channels_in_range(&mapping, info.max_channels, &resolved)?;
    crate::routing::ensure_no_channel_conflicts(&self.config.buses, &bus, &resolved, &mapping)?;

    if let Some(existing) = self.config.buses.iter_mut().find(|b| b.id == bus) {
        existing.device = resolved;
        existing.channels = mapping;
    } else {
        let name = match bus.as_str() {
            "master" => "Master",
            "cue" => "Preview",
            other => other,
        };
        self.config.buses.push(BusConfig::new(
            bus,
            name.to_string(),
            resolved,
            mapping,
        ));
    }
    Ok(())
}
```

Also make `update_bus_config` re-validate with the same helpers before assigning (or call into shared logic).

- [ ] **Step 4: Run tests — pass**

Run: `cargo test -p engine-core set_bus_device -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine-core/src/engine.rs
git commit -m "Implement set_bus_device validation and config updates."
```

---

### Task 5: Multi-plan rings + producer mapping

**Files:**
- Modify: `engine-core/src/producer.rs`
- Modify: `engine-core/src/engine.rs` (stream field + start/stop; keep API surface)

**Interfaces:**
- Consumes: `DeviceStreamPlan`, `map_buses_to_device_buffer`, `resolve_device_stream_plans`
- Produces:
  - `create_device_ring_buffer(frames: u32, channels: u16) -> (Producer<Sample>, Consumer<Sample>, usize)`
  - `producer_thread_loop(..., plans: Vec<DeviceStreamPlan>, producers: Vec<Producer<Sample>>, ...)`
  - `Engine` holds `streams: Vec<Box<dyn AudioStream>>` (replace single `stream`)

- [ ] **Step 1: Extend ring helper for N channels**

```rust
pub(crate) fn create_device_ring_buffer(
    buffer_size: u32,
    channels: u16,
) -> (Producer<Sample>, Consumer<Sample>, usize) {
    const RING_BUFFER_MULTIPLIER: usize = 24;
    let samples_per_buffer = buffer_size as usize * channels as usize;
    let ring_buffer_capacity = samples_per_buffer * RING_BUFFER_MULTIPLIER;
    let (mut producer, consumer) = RingBuffer::new(ring_buffer_capacity);
    let prefill = ring_buffer_capacity.saturating_sub(2 * samples_per_buffer);
    for _ in 0..prefill {
        let _ = producer.push(0.0);
    }
    (producer, consumer, ring_buffer_capacity)
}
```

Keep existing `create_ring_buffer` as a thin wrapper around stereo (`channels = 2`) **or** update all call sites.

- [ ] **Step 2: Rewrite producer loop to write mapped device frames**

Sketch:

```rust
pub(crate) fn producer_thread_loop(
    dsp_engine: Arc<Mutex<DspEngine>>,
    mut device_producers: Vec<(DeviceStreamPlan, Producer<Sample>)>,
    running: Arc<Mutex<bool>>,
    sample_rate: u32,
    fallback_buffer_size: usize,
    // use master ring capacity for fill heuristics, or per-device
    ring_buffer_capacity: usize,
    callback_frames_atomic: Option<Arc<AtomicU32>>,
    callback_count: Arc<AtomicU64>, // master plan only
    transport_events: Arc<Mutex<Vec<TransportEvent>>>,
) {
    let mut output_buses = HashMap::new();
    // ensure master (+ cue if any plan routes it)
    output_buses.insert(BusId::new("master"), vec![0.0; fallback_buffer_size * 2]);
    if device_producers.iter().any(|(p, _)| {
        p.routes.iter().any(|r| r.bus_id.as_str() == "cue")
    }) {
        output_buses.insert(BusId::new("cue"), vec![0.0; fallback_buffer_size * 2]);
    }

    let mut device_scratch: Vec<Vec<Sample>> = device_producers
        .iter()
        .map(|(plan, _)| vec![0.0; fallback_buffer_size * plan.channels as usize])
        .collect();

    // pacing loop same as today using callback_count / frames
    // after dsp.process(chunk_frames, &mut output_buses):
    for (i, (plan, producer)) in device_producers.iter_mut().enumerate() {
        let channels = plan.channels as usize;
        let needed = chunk_frames * channels;
        if device_scratch[i].len() < needed {
            device_scratch[i].resize(needed, 0.0);
        }
        map_buses_to_device_buffer(
            chunk_frames,
            channels,
            &plan.routes,
            &output_buses,
            &mut device_scratch[i][..needed],
        );
        for &sample in &device_scratch[i][..needed] {
            if producer.push(sample).is_err() {
                break;
            }
        }
    }
}
```

- [ ] **Step 3: Change `Engine::start` to open all plans**

```rust
let devices = self.backend.list_output_devices()?;
let plans = crate::routing::resolve_device_stream_plans(&self.config.buses, &devices)?;

let mut streams = Vec::new();
let mut device_producers = Vec::new();
let mut master_callback_count = None;
let mut master_callback_frames = None;
let mut master_sample_rate = self.config.sample_rate;
let mut master_buffer_size = self.config.buffer_size as usize;

for plan in plans {
    let (producer, consumer, capacity) =
        create_device_ring_buffer(self.config.buffer_size, plan.channels);
    let callback_count = Arc::new(AtomicU64::new(0));
    let callback = Box::new(ConsumerCallback::new(consumer, Arc::clone(&callback_count)));
    let params = StreamParams::new(
        self.config.sample_rate,
        plan.channels,
        self.config.buffer_size,
        self.config.low_latency,
    );
    let stream = match self.backend.open_output_stream(&plan.device, &params, callback) {
        Ok(s) => s,
        Err(e) => {
            // stop any opened streams / running flags
            return Err(e);
        }
    };
    // verify sample rate / buffer size like today
    let is_master = plan.routes.iter().any(|r| r.bus_id.as_str() == "master");
    if is_master {
        master_callback_count = Some(Arc::clone(&callback_count));
        master_callback_frames = stream.callback_frames_atomic();
        master_sample_rate = stream.actual_sample_rate().unwrap_or(self.config.sample_rate);
        master_buffer_size = stream.actual_buffer_size().unwrap_or(self.config.buffer_size) as usize;
    }
    device_producers.push((plan, producer));
    streams.push(stream);
}

// spawn producer with master clock atomics
// warmup, then start ALL streams (or MasterStreamSetup-style start_playback on each)
self.streams = Some(streams); // or Vec always
```

Update `stop` / `abort_start` to drop all streams.

- [ ] **Step 4: Unit/integration smoke with null**

Run: `cargo test -p engine-core --tests -- --nocapture`  
Expected: existing tests still PASS (single master default). Fix any single-`stream` assumptions.

- [ ] **Step 5: Commit**

```bash
git add engine-core/src/producer.rs engine-core/src/engine.rs
git commit -m "Open one output stream per bus device plan."
```

---

### Task 6: CPAL honors requested channel count

**Files:**
- Modify: `backend-cpal/src/lib.rs`
- Test: `backend-cpal/src/lib.rs` tests if hardware-free checks exist; otherwise add a unit test on `select_stream_config` preference via documented behavior / mock if available. Prefer a focused unit test on selection logic by extracting the prefer-channels filter if needed.

**Interfaces:**
- Consumes: `StreamParams.channels`
- Produces: open stream with `channels >= params.channels` when supported; else clear error

- [ ] **Step 1: Change `select_stream_config` preference**

Replace “prefer 2 channels” with:

```rust
let desired_channels = params.channels;
let config_range = matching_configs
    .iter()
    .find(|config| config.channels() == desired_channels)
    .or_else(|| {
        matching_configs
            .iter()
            .filter(|config| config.channels() >= desired_channels)
            .min_by_key(|config| config.channels())
    })
    .copied()
    .ok_or_else(|| {
        anyhow::anyhow!(
            "No supported config for {} Hz with at least {} channels",
            params.sample_rate,
            desired_channels
        )
    })?;
```

If the chosen config has **more** channels than `params.channels`, either:
- open at that channel count and treat extra channels as silence (producer already sizes `plan.channels`), **or**
- fail if `supported_config.channels() != params.channels`.

Prefer: require exact match first; if only larger exists, bump the plan’s stream channel count to the granted count **only if** routing still fits — simplest v1: **exact match required**, else error naming the device.

After open, if `supported_config.channels() != params.channels`, return Err explaining the mismatch.

- [ ] **Step 2: Build/test**

Run: `cargo test -p backend-cpal -- --nocapture`  
Expected: PASS (device-dependent tests may skip)

- [ ] **Step 3: Commit**

```bash
git add backend-cpal/src/lib.rs
git commit -m "Prefer StreamParams.channels when opening CPAL streams."
```

---

### Task 7: Integration test — master + cue on null

**Files:**
- Modify: `engine-core/tests/integration_tests.rs`

**Interfaces:**
- Consumes: multi-stream `Engine::start` with two `BusConfig`s on `null-device` (channels 1–2 and 3–4)

- [ ] **Step 1: Add test**

```rust
#[test]
fn starts_with_master_and_cue_buses_on_null() {
    let mut config = EngineConfig::default();
    config.backend = "null".into();
    config.buses = vec![
        audio_core::BusConfig::new(
            BusId::new("master"),
            "Master".into(),
            audio_core::DeviceId::new("null-device"),
            audio_core::ChannelMapping::new(3, 4),
        ),
        audio_core::BusConfig::new(
            BusId::new("cue"),
            "Preview".into(),
            audio_core::DeviceId::new("null-device"),
            audio_core::ChannelMapping::new(1, 2),
        ),
    ];
    let mut engine = Engine::new(config).unwrap();
    assert!(engine.start().is_ok());
    engine
        .set_deck_headphone_cue(0, true)
        .expect("engine API from Task 8 may land here — if not yet, only assert start()");
    engine.stop().unwrap();
}
```

If Task 8 API is not ready, assert start/stop only in this task; add headphone line in Task 8.

- [ ] **Step 2: Run**

Run: `cargo test -p engine-core starts_with_master_and_cue -- --nocapture`  
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add engine-core/tests/integration_tests.rs
git commit -m "Integration-test multi-bus start on null backend."
```

---

### Task 8: Engine + GUI headphone cue wiring

**Files:**
- Modify: `engine-core/src/engine.rs` — `set_deck_headphone_cue`
- Modify: `gui-app/src-tauri/src/lib.rs` — `DeckInfo` / `DeckStatus` / command
- Modify: `gui-app/src-tauri/src/engine_controller.rs`
- Modify: `gui-app/src/types.ts`
- Modify: `gui-app/src/stores/defaultDeck.ts`
- Modify: `gui-app/src/stores/engineStore.ts`
- Modify: `gui-app/src/components/DeckMixer.tsx`

**Interfaces:**
- Produces:
  - `Engine::set_deck_headphone_cue(&mut self, deck_id: usize, enabled: bool) -> Result<()>`
  - Tauri: `set_deck_headphone_cue(deckId, enabled)`
  - `DeckStatus.headphone_cue: bool`

- [ ] **Step 1: Engine API**

```rust
pub fn set_deck_headphone_cue(&mut self, deck_id: usize, enabled: bool) -> Result<()> {
    let dsp_engine = self
        .dsp_engine
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
    let mut dsp = dsp_engine.lock().unwrap();
    let deck = dsp
        .deck_mut(deck_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
    deck.set_headphone_cue(enabled);
    Ok(())
}
```

- [ ] **Step 2: Tauri status + command**

Add `headphone_cue: bool` to `DeckInfo` (default false) and `DeckStatus`. Map in `engine_controller::deck_status`.

```rust
#[tauri::command]
fn set_deck_headphone_cue(
    deck_id: usize,
    enabled: bool,
    app: AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    if deck_id >= state.decks.len() {
        return Err(format!("Invalid deck ID: {}", deck_id));
    }
    state.decks[deck_id].headphone_cue = enabled;
    with_engine(&mut state, |engine| {
        engine
            .set_deck_headphone_cue(deck_id, enabled)
            .map_err(|e| e.to_string())
    })?;
    Ok(publish_deck(&app, &mut state, deck_id))
}
```

Register in `invoke_handler`. On `start_engine`, re-apply each deck’s `headphone_cue`.

- [ ] **Step 3: Frontend**

`types.ts` / `defaultDeck.ts`: add `headphone_cue: false`.

`engineStore.ts`:

```ts
setDeckHeadphoneCue: async (deckId: number, enabled: boolean) => {
  await invoke("set_deck_headphone_cue", { deckId, enabled });
},
```

`DeckMixer.tsx`: drop local-only `channelUi.cue` for headphone (or sync from `deck.headphone_cue`). Prefer engine-backed:

```tsx
cue={deck.headphone_cue}
onCueChange={(enabled) => void setDeckHeadphoneCue(deck.id, enabled)}
title="Headphone cue"
```

Remove “routing coming in Phase 4”.

- [ ] **Step 4: Verify TypeScript**

Run: `cd gui-app && npm run build` (or the repo’s usual `tsc`/vite check)  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine-core/src/engine.rs gui-app/src-tauri/src/lib.rs gui-app/src-tauri/src/engine_controller.rs gui-app/src/types.ts gui-app/src/stores/defaultDeck.ts gui-app/src/stores/engineStore.ts gui-app/src/components/DeckMixer.tsx
git commit -m "Wire headphone cue toggle through engine and UI."
```

---

### Task 9: Settings apply restarts the engine

**Files:**
- Modify: `gui-app/src-tauri/src/lib.rs` (`save_settings`, helpers)
- Optionally: `gui-app/src/hooks/useSettings.ts` (toast on restart failure)

**Interfaces:**
- Consumes: existing `apply_settings`, `start_engine` / `stop_engine` patterns
- Produces: `save_settings` while running performs stop → apply → start and rehydrates decks/tracks

- [ ] **Step 1: Replace “must stop first” guard**

```rust
#[tauri::command]
fn save_settings(
    app: AppHandle,
    settings: AppSettings,
    shared: State<'_, SharedAppState>,
) -> Result<AppSettings, String> {
    let shared_state = shared.inner().clone();
    let mut state = shared.lock().map_err(|e| e.to_string())?;

    let was_running = state.engine.is_some();
    // Snapshot what we need to reload (paths / track_ids / playing flags / headphone_cue / volumes...)
    let deck_snapshot = state.decks.clone();
    let crossfader = state.crossfader;

    if was_running {
        state.notifier = None;
        if let Some(mut engine) = state.engine.take() {
            engine.stop().map_err(|e| e.to_string())?;
        }
        // Do NOT clear_deck_info here — keep UI track metadata
    }

    apply_settings(&mut state, settings);

    if was_running {
        let config = state.engine_config.clone();
        let mut engine = Engine::new(config).map_err(|e| e.to_string())?;
        engine.start().map_err(|e| e.to_string())?;
        // Re-apply mixer state like start_engine (volumes, EQ, filter, gain, speed, headphone_cue, crossfader)
        // Reload tracks from deck_snapshot.track / track_id using the same load path as load_track command
        state.engine = Some(engine);
        state.notifier = Some(EngineNotifier::start(app.clone(), shared_state));
        let _ = publish_status(&app, &mut state);
    }

    Ok(settings_from_state(&state))
}
```

Extract shared “rehydrate engine from AppState decks” helper used by both `start_engine` and `save_settings` to avoid drift. Track reload: call the same internal load helper used by the load-track command when `track`/`track_id` is present.

- [ ] **Step 2: Manual / smoke**

Run: `cargo test -p engine-core --tests` and build tauri lib if practical:  
`cargo check -p gui-app` (or package name used in workspace)  
Expected: compile clean

- [ ] **Step 3: Commit**

```bash
git add gui-app/src-tauri/src/lib.rs
git commit -m "Restart engine when audio settings are saved."
```

- [ ] **Step 4: Docs touch-up**

Update `README.md` (remove set_bus_device stub line) and optionally check off tech-spec “Next” item for bus mapping if that doc is maintained.

```bash
git add README.md docs/tech-spec.md
git commit -m "Document completed multi-bus device routing."
```

---

## Self-review checklist

| Spec requirement | Task |
|------------------|------|
| Device-plan multi-stream | 1, 5 |
| master + optional cue buses | 5, 7, 9 |
| Channel map / conflicts | 4, existing routing tests |
| PFL pre-fader to cue | 2, 3, 8 |
| Settings restart | 9 |
| `set_bus_device` real | 4 |
| CPAL channel count | 6 |
| Integration null | 7 |

No TBD placeholders; bus id `cue` consistent with Tauri; `set_deck_headphone_cue` named consistently across Tasks 7–8.
