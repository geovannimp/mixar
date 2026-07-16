# Multi-Bus Audio Routing (Main + Preview) Design

**Issue:** [#54](https://github.com/geovannimp/rust-dj-engine/issues/54)  
**Spec refs:** `docs/tech-spec.md` §5.3 Channel Mapping, §14 Next; `docs/deck-spec.md` §5.15 Headphone cue / PFL (H1)  
**Date:** 2026-07-15

## Goal

Make master and optional preview (cue) bus routing real: map buses to devices and stereo channel pairs, open one multi-channel stream per unique device (or one stream per device when buses land on different hardware), and route per-deck headphone cue (PFL) pre-fader audio to the cue bus. Settings changes restart the engine so new routing takes effect.

## Requirements

| Decision | Choice |
|----------|--------|
| Approach | Device-plan multi-stream (group buses by device via existing `resolve_device_stream_plans`) |
| Buses | `master` always; `cue` when preview is enabled in settings (UI label “Preview”) |
| Channel map | Stereo pair per bus; 1-based indexes; no overlaps on same device |
| Devices | Same device multi-channel **or** dedicated headphone/secondary device |
| PFL (H1) | Per-deck headphones toggle → sum **pre-fader** audio of cued decks onto cue bus |
| Settings apply | If engine running: stop → apply config → start again |
| `set_bus_device` | Validate + update `EngineConfig` only; live audio via restart/start |

Out of scope: cue-mix knob (H2), split cue (H3), live hot-swap without restart, master/PFL VU meters, cross-device sample-rate conversion.

## Architecture

```text
Settings (master_bus / preview_bus)
  → EngineConfig.buses  (master always; cue if preview_enabled)
  → resolve_device_stream_plans()
  → N DeviceStreamPlans (grouped by device)

Per plan:
  open_output_stream(device, channels=plan.channels)
  ring buffer of device-interleaved frames
  ConsumerCallback (pop interleaved → out)

Producer (paced by master plan's callback_count):
  DspEngine::process → HashMap<BusId, stereo bus>
    master: post-fader mix (existing mixer path)
    cue:    sum of pre-fader decks where headphone_cue == true
  map_buses_to_device_buffer → each device ring
```

Bus ids stay `master` and `cue` (`PREVIEW_BUS_ID = "cue"` in Tauri already). UI wording remains “Preview” / headphones cue.

## Components

### DSP (`engine-dsp`)

- **`Deck`:** add `headphone_cue: bool` with getters/setters. During `process`, after gain trim → EQ → filter and **before** volume (same tap as VU peaks), retain a reusable pre-fader interleaved buffer for the mixer.
- **`Mixer::route_to_buses`:**
  - **master** — current post-fader + crossfader mix × `master_volume`
  - **cue** — sum of cued decks’ pre-fader buffers × `cue_volume`; silence if no deck is cued or the cue bus is absent
- Update mixer tests that currently assume cue equals a scaled copy of master.

### Engine (`engine-core`)

- **`start`:** resolve plans from `config.buses` (empty → default master on default device, channels 1–2). Open one stream + ring buffer per plan. Producer writes all plans; master plan’s `callback_count` paces production.
- Prefer mapping in the producer (`map_buses_to_device_buffer`) so `ConsumerCallback` stays allocation-free and layout-agnostic.
- **`set_bus_device` / `update_bus_config`:** validate channel pair, device existence, range vs `DeviceInfo.max_channels`, and no channel conflicts; update config. Do not reopen streams in place.
- Abort start if any stream fails; tear down streams opened earlier in that attempt.

### Backends

- Honor `StreamParams.channels` when opening streams (CPAL today prefers stereo). Required for same-device mappings such as cue 1–2 / master 3–4. If the device cannot provide the requested channel count, fail with a clear error.

### GUI (`gui-app`)

- Wire mixer headphones toggle → Tauri command → `Engine::set_deck_headphone_cue` (and status / `DeckUpdated`).
- Enable cue when a track is loaded; remove “routing coming in Phase 4” copy.
- **`save_settings`:** allow save while engine is running: stop → `apply_settings` → start; rehydrate volumes/EQ/filter/gain/cue and reload tracks into the engine when still present in UI state (extend today’s `start_engine` rehydrate path; avoid unnecessary `clear_deck_info` on settings restart).

## Data flow (PFL)

```text
Deck::process
  trim → EQ → filter
       │
       ├─► pre_fader_buffer (for cue sum + VU)
       └─► × volume → mixer graph → master mix

Mixer cue bus = Σ pre_fader_buffer[i] for decks with headphone_cue
```

## Error handling

- Invalid pairs, overlaps, unknown devices, out-of-range channels → validation errors at config update / start; toast on settings apply.
- Partial start failure → full teardown; no half-running engine.
- Preview disabled → only master bus/plan.
- Two physical devices: pace on master; secondary underruns remain silence (no cross-device clock sync in this issue).

## Testing

| Layer | Coverage |
|-------|----------|
| Unit (`engine-dsp`) | Cue = pre-fader sum of cued decks only; silent when none cued |
| Unit (`engine-core` routing) | Existing same-device multi-bus + `map_buses_to_device_buffer`; `set_bus_device` validation |
| Integration (null backend) | Start with master+cue; producer fills mapped rings |
| GUI / Tauri | Headphones → engine; save_settings while running restarts with new buses |

## Success criteria

1. Master bus plays on the configured device/channels.
2. With preview enabled, cue bus opens on its configured device/channels (same or different device).
3. Headphones on a playing deck sends that deck’s pre-fader audio to cue only; master mix unchanged by cue toggles.
4. Settings apply while playing restarts the engine and applies new routing without requiring a manual stop-first dance.
5. `set_bus_device` is no longer a no-op stub (validates and persists into config).
