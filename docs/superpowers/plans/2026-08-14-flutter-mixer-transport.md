# Flutter mixer strip → EngineTransport — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the Flutter mixer strip to typed `EngineTransport` cmds and expanded thin `EngineEvt` (including VU peak-hold).

**Architecture:** Grow flat `EngineEvt` with mixer optionals; typed FRB cmds (`set_volume`, `set_eq_band`, `set_filter`, `set_gain_trim`, `set_headphone_cue`, `set_crossfader`); Status fan-out + hydrate on subscribe; Riverpod snapshot with levels isolated from knobs.

**Tech Stack:** Rust (`host-flutter`, `engine-api` CmdBody), flutter_rust_bridge 2.12, Riverpod, existing MixerStrip.

## Global Constraints

- Dart never sees MessagePack; typed FRB only
- GUI `soft_takeover: false`
- Faders 0–100 in the widget, 0–1 on the wire
- Cue mix / master cue out of scope
- Prefer shortest diffs; no new packages

## File map

| Path | Role |
|------|------|
| `crates/host-flutter/src/api/engine.rs` | EngineEvt fields, map_engine_evts, typed cmds, hydrate |
| `crates/host-flutter/tests/engine_transport.rs` | Volume cmd + Status/Levels mapping tests |
| FRB generated files | Regen after Rust API change |
| `apps/gui-flutter/lib/mixer/track_drag.dart` | EngineUiSnapshot + applyEngineEvt |
| `apps/gui-flutter/lib/mixer/engine_providers.dart` | Channel/levels/crossfader providers + cmd helpers |
| `apps/gui-flutter/lib/mixer/mixer_strip.dart` | ConsumerWidget wired to providers |
| `apps/gui-flutter/test/track_drag_test.dart` | Reducer tests for mixer/levels |

---

### Task 1: Host EngineEvt + cmds + mapping

**Files:**
- Modify: `crates/host-flutter/src/api/engine.rs`
- Modify: `crates/host-flutter/tests/engine_transport.rs`

**Produces:** `EngineTransport::set_volume` / `set_eq_band` / `set_filter` / `set_gain_trim` / `set_headphone_cue` / `set_crossfader`; `map_engine_evts`; `EqBand` FRB enum; hydrate on subscribe.

- [ ] Expand `EngineEvt`, add typed cmds, fan-out Status, map Levels hold, hydrate on subscribe.
- [ ] Tests: set_volume → Updated; map Status includes crossfader + per-deck volume; map Levels includes peak_hold.
- [ ] `cargo test --manifest-path crates/Cargo.toml -p host_flutter`

### Task 2: FRB regenerate

- [ ] `cd apps/gui-flutter && mise exec -- flutter_rust_bridge_codegen generate`

### Task 3: Dart snapshot + MixerStrip

**Files:**
- Modify: `apps/gui-flutter/lib/mixer/track_drag.dart`
- Modify: `apps/gui-flutter/lib/mixer/engine_providers.dart`
- Modify: `apps/gui-flutter/lib/mixer/mixer_strip.dart`
- Modify: `apps/gui-flutter/test/track_drag_test.dart`

**Produces:** `MixerChannelUi`, isolated levels providers, strip cmds.

- [ ] Reducer + providers; MixerStrip reads/writes transport; disable when engine idle.
- [ ] Dart tests for mixer patch / levels isolation.
- [ ] `mise exec -- flutter test test/track_drag_test.dart test/widget_test.dart`

### Task 4: Verify + PR

- [ ] `cargo test -p host_flutter` and Flutter tests above
- [ ] Commit focused changes; open PR
