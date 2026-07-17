# Mixer Channel Graph Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the per-deck channel strip from `Deck` into mixer-owned `dasp_graph` channel nodes without changing audible signal order or GUI APIs.

**Architecture:** `Deck` renders dry resampled stereo into source nodes. Each source feeds one stateful `MixerChannel` node that applies gain, EQ, filter, pre-fader metering/PFL capture, channel fader, and crossfader gain before `Sum`.

**Tech Stack:** Rust, `dasp_graph`, `audio-core`, existing `engine-dsp` EQ/filter/meter primitives, Cargo tests.

## Global Constraints

- Preserve `playback → auto+trim → EQ → filter → VU/PFL → channel fader → bus sum`.
- Keep `engine-dsp` pure Rust with zero I/O dependencies.
- Keep existing Tauri command names and GUI status fields.
- Do not change volume-normalizer math, trim persistence, FX, or crossfader assignment.
- Use test-first red/green cycles for behavior changes.

---

### Task 1: Stateful mixer channel graph node

**Files:**
- Create: `engine-dsp/src/mixer_channel.rs`
- Modify: `engine-dsp/src/lib.rs`
- Test: `engine-dsp/src/mixer_channel.rs`

**Interfaces:**
- Produces: `MixerChannel::new(sample_rate: u32) -> Self`
- Produces: getters/setters for volume, EQ, filter, trim, auto gain, headphone cue, crossfader gain, peaks, and pre-fader buffer
- Produces: `begin_render(&mut self, sample_count: usize)` and `dasp_graph::Node`

- [ ] **Step 1: Write failing node tests**

Add tests that instantiate `MixerChannel`, process stereo buffers, and assert:

```rust
assert_eq!(channel.auto_gain_db(), 0.0);
assert_eq!(channel.gain_trim_db(), 0.0);
assert_eq!(channel.volume(), 1.0);
assert!(channel.set_volume(1.1).is_err());
```

Also assert +6 dB auto gain raises pre-fader output, zero channel volume silences graph output while leaving PFL non-zero, and peaks are measured before fader.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `cargo test -p engine-dsp mixer_channel -- --nocapture`  
Expected: FAIL because `mixer_channel` and `MixerChannel` do not exist.

- [ ] **Step 3: Implement the node**

Create one type owning `ThreeBandEq`, `DjFilter`, gain fields, volume, crossfader gain, cue state, `LevelPeaks`, and a reusable interleaved PFL buffer. Its `Node::process` must copy each stereo input frame through:

```rust
let gain = db_to_linear(self.auto_gain_db + self.gain_trim_db);
// gain → EQ → filter → capture/measure → volume * crossfader_gain
```

Use existing clamp/validation behavior and append each processed graph chunk to the render PFL buffer.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run: `cargo test -p engine-dsp mixer_channel -- --nocapture`  
Expected: PASS.

### Task 2: Rewire Mixer graph and cue routing

**Files:**
- Modify: `engine-dsp/src/mixer.rs`
- Test: `engine-dsp/src/mixer.rs`

**Interfaces:**
- Consumes: `MixerChannel`
- Produces: `Mixer::new(sample_rate: u32, num_channels: usize) -> Self`
- Produces: `channel(&self, index) -> Option<&MixerChannel>` and `channel_mut(&mut self, index) -> Option<&mut MixerChannel>`

- [ ] **Step 1: Convert mixer tests to the desired channel API**

Replace deck-owned setup such as:

```rust
decks[0].set_volume(0.0)?;
decks[0].set_headphone_cue(true);
```

with:

```rust
mixer.channel_mut(0).unwrap().set_volume(0.0)?;
mixer.channel_mut(0).unwrap().set_headphone_cue(true);
```

Add an assertion that graph channel count equals deck count and that PFL remains audible with a closed fader.

- [ ] **Step 2: Run mixer tests and verify RED**

Run: `cargo test -p engine-dsp mixer::tests -- --nocapture`  
Expected: FAIL because the constructor and channel accessors do not exist.

- [ ] **Step 3: Rewire graph topology**

Change `MixerNode` to support `DeckSource`, `Channel(MixerChannel)`, and `Sum`; create edges `source → channel → sum`. Render dry deck buffers, reset channel capture, assign crossfader gains, process the graph, and build cue PFL from channel-owned buffers.

- [ ] **Step 4: Run mixer tests and verify GREEN**

Run: `cargo test -p engine-dsp mixer::tests -- --nocapture`  
Expected: PASS.

### Task 3: Make Deck playback-only

**Files:**
- Modify: `engine-dsp/src/deck.rs`
- Test: `engine-dsp/src/deck.rs`

**Interfaces:**
- Produces: `Deck::load(audio: Arc<LoadedAudio>) -> Result<()>`
- Produces: `Deck::process(frames) -> Result<&[Sample]>` returning dry resampled PCM

- [ ] **Step 1: Replace channel-strip deck tests with a dry-output test**

Add a test that loads fixed samples, plays, processes, and asserts the output equals the playback amplitude without channel gain/fader processing. Remove tests for APIs that now belong to `MixerChannel`.

- [ ] **Step 2: Run deck tests and verify RED**

Run: `cargo test -p engine-dsp deck::tests -- --nocapture`  
Expected: FAIL while `Deck::load` still requires auto gain and the old strip remains.

- [ ] **Step 3: Remove channel-strip state and processing**

Delete channel volume, EQ, filter, trim, auto gain, peaks, PFL, and headphone cue fields/methods from `Deck`. Change `load` to accept only audio and stop processing after dry playback/resampling and track-end handling.

- [ ] **Step 4: Run engine-dsp tests**

Run: `cargo test -p engine-dsp -- --nocapture`  
Expected: PASS.

### Task 4: Delegate engine APIs to mixer channels

**Files:**
- Modify: `engine-dsp/src/lib.rs`
- Modify: `engine-core/src/engine.rs`
- Modify: engine-core tests and call sites using `Deck::load`

**Interfaces:**
- Consumes: `DspEngine::mixer[_mut]` and indexed mixer channels
- Preserves: `Engine::load_track(deck_id, audio, auto_gain_db)`
- Preserves: existing `set_deck_*` public method names

- [ ] **Step 1: Add failing delegation tests**

Assert that loading with `6.0` stores `6.0` on mixer channel 0 and that volume/EQ/filter/trim/headphone cue setters update that channel, not `Deck`.

- [ ] **Step 2: Run focused engine tests and verify RED**

Run: `cargo test -p engine-core --lib -- --nocapture`  
Expected: FAIL because engine methods still target deck-owned controls.

- [ ] **Step 3: Update construction, loading, snapshots, and setters**

Construct `Mixer` with sample rate and deck count. In `load_track`, validate the index, call `deck.load(audio)`, then set channel auto gain. Route level snapshots and all mixer-strip setters through `dsp.mixer[_mut]().channel[_mut](deck_id)`.

- [ ] **Step 4: Update remaining Rust call sites**

Change direct DSP deck loads from:

```rust
deck.load(audio, 0.0)?;
```

to:

```rust
deck.load(audio)?;
```

Keep `Engine::load_track(..., auto_gain_db)` unchanged for Tauri and examples.

- [ ] **Step 5: Run affected crates**

Run: `cargo test -p engine-dsp -p engine-core -- --nocapture`  
Expected: PASS.

### Task 5: Full regression verification

**Files:**
- Modify only files required by compiler-reported stale APIs

**Interfaces:**
- Preserves all public GUI/Tauri behavior

- [ ] **Step 1: Format and check**

Run: `cargo fmt --all -- --check`  
Expected: PASS.

Run: `cargo check --workspace --all-targets`  
Expected: PASS.

- [ ] **Step 2: Run workspace tests**

Run: `cargo test --workspace`  
Expected: PASS.

- [ ] **Step 3: Inspect diagnostics and diff**

Confirm no new IDE diagnostics, no duplicate gain processing in `Deck`, and no changed GUI command/data contracts.
