# Flutter Waveform Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dual-deck Flutter waveforms with L0 overview, lazy L1, beat grid, dual overviews, and seek/scrub, painted on Impeller.

**Architecture:** Rust returns packed RGB peaks; Dart records a `ui.Picture` and scrolls it with a transform. High-rate `position_ms` stays off the engine chrome snapshot.

**Tech Stack:** Flutter/Impeller, Riverpod, FRB 2.12, `library` + `audio-core`, `host_flutter`.

## Global Constraints

- No `render_waveform_lane` / packed RGBA frames.
- Peaks = mono RGB uint8 (`count × 3`).
- Visible span 24_000 ms × speed ratio (0.5–2).
- Unity EQ gains. No zoom. No cue/loop overlays.
- Cargo via `cargo --manifest-path crates/Cargo.toml`. FRB: `moon run gui-flutter:generate`.
- Flutter tests: `mise exec -- flutter test` from `apps/gui-flutter`.

---

### Task 1: Dart peak / layout math

**Files:**
- Create: `apps/gui-flutter/lib/mixer/waveform/peaks.dart`
- Create: `apps/gui-flutter/lib/mixer/waveform/spectral_color.dart`
- Create: `apps/gui-flutter/lib/mixer/waveform/layout.dart`
- Create: `apps/gui-flutter/lib/mixer/waveform/beat_grid.dart`
- Test: `apps/gui-flutter/test/waveform_math_test.dart`

- [ ] Failing tests then implement decode, `peakAtTime`, spectral RGB, visible ms, window rect, center-scrub, beat x.

### Task 2: Library window + host FRB

**Files:**
- Modify: `crates/library/src/lib.rs` (`compute_waveform_window`)
- Modify: `crates/host-flutter/src/api/library.rs`
- Modify: `crates/host-flutter/src/api/engine.rs` (seek + evt fields)
- Test: library + `host_flutter` tests
- FRB generate

### Task 3: Engine UI isolation + providers

**Files:**
- Modify: `apps/gui-flutter/lib/mixer/engine_ui.dart`
- Modify: `apps/gui-flutter/lib/mixer/engine_providers.dart`
- Test: `apps/gui-flutter/test/engine_ui_test.dart`

### Task 4: Widgets

**Files:**
- Create painter / scrolling lane / overview strip
- Modify: `waveform_section.dart`, `mixer_page.dart`, `deck_panel.dart`
- Test: widget_test empty copy + deck overview present

### Task 5: Verify + PR
