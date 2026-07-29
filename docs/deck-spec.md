# Tech Spec — DJ Deck (UI + Engine)

Reference: [`tech-spec.md`](tech-spec.md), [`dj-waveform-spec.md`](dj-waveform-spec.md), [`audio-analyzer-spec.md`](audio-analyzer-spec.md).

This document defines what a **professional DJ deck** should contain in rust-mixer, based on competitor products (Rekordbox, Serato DJ Pro, Traktor Pro, DJUCED / Hercules), the attached reference screenshots, and the gap between our current MVP and industry expectations.

---

## Table of Contents

- [1 — Summary](#1--summary)
- [2 — Competitor Reference](#2--competitor-reference)
- [3 — Current State (rust-mixer)](#3--current-state-rust-mixer)
- [4 — Deck Information Architecture](#4--deck-information-architecture)
- [5 — Feature Specification](#5--feature-specification)
  - [5.1 Track metadata & status](#51-track-metadata--status)
  - [5.2 Waveforms & visual navigation](#52-waveforms--visual-navigation)
  - [5.3 Transport & playback controls](#53-transport--playback-controls)
  - [5.4 Tempo, pitch & sync](#54-tempo-pitch--sync)
  - [5.5 Controller pads](#55-controller-pads)
  - [5.6 Loops](#56-loops)
  - [5.7 Beat grid & quantize](#57-beat-grid--quantize)
  - [5.8 Mixer channel (per deck)](#58-mixer-channel-per-deck)
  - [5.9 Effects (FX)](#59-effects-fx)
  - [5.10 Stems & pad modes](#510-stems--pad-modes)
  - [5.11 Jog / scratch / vinyl mode](#511-jog--scratch--vinyl-mode)
  - [5.12 Slip mode & advanced transport](#512-slip-mode--advanced-transport)
  - [5.13 Beat jump & navigation](#513-beat-jump--navigation)
  - [5.14 Library & load workflow](#514-library--load-workflow)
  - [5.15 Headphone cue / PFL](#515-headphone-cue--pfl)
  - [5.16 Hardware & MIDI](#516-hardware--midi)
- [6 — Data Model](#6--data-model)
- [7 — Engine vs GUI Responsibilities](#7--engine-vs-gui-responsibilities)
- [8 — API Surface](#8--api-surface)
- [9 — Engine Event System](#9--engine-event-system)
- [10 — Phased Roadmap](#10--phased-roadmap)
- [11 — Acceptance Criteria](#11--acceptance-criteria)
- [12 — References](#12--references)

---

## 1 — Summary

A DJ deck is a **performance surface** for one loaded track. It combines:

1. **Navigation** — waveforms, overview, beat grid, cues, loops.
2. **Playback control** — play/pause, cue, seek, jog/scratch.
3. **Tempo & harmony** — pitch fader, sync, key lock, key display/shift.
4. **Creative tools** — controller pads (default: hot cues), loops, FX, stems, sampler.
5. **Mixer integration** — volume, EQ, filter, cue/PFL, crossfader assignment.

Industry decks (Rekordbox Performance, Serato, Traktor) share a common layout pattern visible in the reference screenshots:

```text
┌──────────────────────────────────────────────────────────────────────┐
│  Overview waveform (full track) + cue/loop markers                   │
├──────────────────────────────────────────────────────────────────────┤
│  Scrolling detailed waveform(s) — fixed center playhead              │
├───────────────┬──────────────────────────────────────┬───────────────┤
│ Track info    │  BPM · Key · Time · Sync state       │  FX / filter  │
│ Title/Artist  │  Controller pads (mode selector)     │  Loop controls│
├───────────────┴──────────────────────────────────────┴───────────────┤
│  Jog wheel · CUE · PLAY · SYNC · Pitch fader · Filter · Loop · PFL   │
└──────────────────────────────────────────────────────────────────────┘
```

Our MVP deck today covers **load, play/pause, dual scrolling waveforms, volume, 3-band EQ, crossfader** only. This spec lists everything a complete deck needs and prioritizes implementation.

---

## 2 — Competitor Reference

| Product | Deck count | Hot cues | Memory cues | Loops | Sync | Key | Stems | FX | Notable UX |
|---------|------------|----------|-------------|-------|------|-----|-------|-----|------------|
| **Rekordbox 7** ([manual](https://cdn.rekordbox.com/files/20241213141602/rekordbox7.0.7_manual_EN.pdf)) | 2–4 | 16 | 10 | In/out, saved, hotcue-as-loop | Beat / BPM / Key sync | Musical + Camelot | Stems (subscription) | 3 slots + RMX/DJM-style | Intelligent cue analysis, phrase/vocal analysis, master deck |
| **Serato DJ Pro** ([manual](https://serato.com/dj/pro)) | 2–4 | 8 | Temp cue | Auto + manual + loop roll | Smart + Simple sync | Key detect + display | Stems FX / pad modes | 50+ built-in | Slip mode, quantize, beat jump, slicer, key lock |
| **Traktor Pro** | 2–4 | 8 | Load marker | In/out, beat-sized | Sync | Key + transpose | Stems (version-dependent) | 2 FX units + filter | Colored waveforms, flux/slip variants, MIDI mapping |
| **Virtual DJ** ([manual](https://www.virtualdj.com/manuals/)) | 2–99 | 8 (pad mode) | — | In/out, pad modes | Beat / BPM sync | Key detect + display | **Stems pad mode** (Vocal, Instru, Bass, Kick, HiHat, Stems FX) | Pad-assigned FX | **8 performance pads** switch function by mode (Hot Cue, Stems, Sampler, …); vertical PADS / LOOP side labels |
| **DJUCED / Hercules** (screenshot ref.) | 2 | 8 labeled | — | 1/2× length, IN/OUT | Beat / Key / Master sync | Key shift ± | Vocal / Drums / Inst mute | 3 FX dropdowns | Named hot cues (Intro, Drop), quantize, slip, vinyl |

Common expectations across all products:

- **Dual-resolution waveforms** (overview + scrolling detail).
- **Beat grid** aligned to offline analysis; sync/quantize depend on it.
- **Controller pads** (8 slots, 2×4 grid) whose **function changes by pad mode**; default mode is **Hot Cue** (Virtual DJ, Serato pad modes).
- **Hot cues** (in Hot Cue mode): color, optional name/comment, jump on trigger.
- **Pitch/tempo** via fader or numeric control; **key lock** when tempo changes.
- **Sync** to align deck tempo (and optionally key) to master or other deck.
- **Loops** with beat-quantized length and halve/double.
- **Per-deck FX** (at least filter + 1–2 insert effects).
- **Stem or frequency isolation** increasingly standard (mute vocals, drums-only, etc.).

---

## 3 — Current State (rust-mixer)

### GUI (`gui-app`)

| Area | Implemented | Missing / next |
|------|-------------|----------------|
| Deck panel | Load (picker + drag-drop), play/pause, metadata, transport, pads, sync, sampler | Layout polish; some Phase 4+ (slip, FX UI) |
| Waveforms | Dual-lane scroll + overview preview, beat grid when analyzed | Zoom; richer cue/loop overlays |
| Mixer strip | Volume, 3-band EQ, filter, gain trim, crossfader, cue/PFL, VU | — |
| Engine start | Auto-start on Decks via store `ensureEngineRunning` → `publishCmd("engine", "start_engine")` | — |
| State sync | **`EngineTransport`** → `engine://bus` → store (`applyBusEvent`) | MIDI host; optional richer hydrate cmd on the bus |
| Library UI | **`LibraryTransport`** for tracks / artwork / waveform raster | — |

### Engine / DSP (`engine-dsp`, `engine-core`)

| Capability | Status |
|------------|--------|
| Play / pause / stop | Yes |
| Volume / EQ / filter / gain trim | Yes |
| Playback speed (`set_speed`) | Yes (GUI pitch control) |
| Seek | Yes (waveform / scrub) |
| Hot cues / loops / quantize | Yes |
| Sync / master deck | Yes |
| Sampler pads / banks | Yes |
| FX chain / stems / scratch | No (later phases) |

### Library metadata

Analysis + DB fields (`title`, `artist`, `bpm`, `key`, `duration_secs`, beat grid, loudness, artwork) feed deck status via host-enriched bus payloads and `LibraryTransport`.

### Runtime playback time (in `DeckStatus`)

| Field | Source | Meaning |
|-------|--------|---------|
| `position_secs` | High-rate `position` on `engine://bus` | Current playhead (elapsed) |
| `duration_secs` | Loaded track metadata on deck snapshot | Total track length |

`remaining_secs` is derived in the UI as `duration_secs - position_secs` when both are set.

---

## 4 — Deck Information Architecture

Each deck exposes a single **`DeckState`** snapshot to the UI (poll or event stream) and accepts **`DeckCommand`** mutations.

```text
DeckState
├── identity: deck_id (0..N-1)
├── loaded: Option<LoadedTrackView>
│   ├── track_id, path, title, artist, album, artwork_ref
│   ├── bpm, key, duration_secs
│   └── analysis: beat_grid_ref, cue_points[], loops[]
├── transport: playing, position_secs, remaining_secs
├── tempo: original_bpm, effective_bpm, pitch_percent, pitch_range
├── sync: { off | arm | tempo_sync | beat_sync }, master, key_sync_enabled
├── key: display_key, key_shift_semitones, key_lock
├── loop: { inactive | active(in, out, length_beats, rolling) }
├── slip: enabled, shadow_position_secs
├── pads: { mode, slots[8] }          -- mode selects pad function; slots are mode-specific state
│   └── hot_cue mode → maps to persisted track_hot_cue rows
├── fx: filter, slots[3]
├── stems: { vocal, instrumental, bass, drums, hihat } mute/solo gains
├── mixer: volume, eq{low,mid,high}, gain_trim_db, cue_enabled
└── waveform: scroll_window_secs, zoom_level
```

UI layout zones (match competitor ergonomics):

| Zone | Priority | Contents |
|------|----------|----------|
| **A — Waveform stack** | P0 | Overview + scrolling lane + playhead + grid + markers |
| **B — Metadata bar** | P0 | Title, artist, elapsed/remain/total, BPM, key |
| **C — Controller pads** | P1 | 8 performance pads (2×4); **mode selector**; default **Hot Cue** mode |
| **D — Loop / jump** | P1 | Loop in/out, length, ½/2×, beat jump (separate panel; Virtual DJ “LOOP” side label) |
| **E — Transport row** | P0 | Cue, Play/Pause, Sync, optional Reverse |
| **F — Tempo column** | P1 | Pitch fader, BPM readout, pitch range, key lock |
| **G — FX / filter** | P2 | Filter knob, 1–3 FX slots |
| **H — Extended pad modes** | P3+ | Stems, Sampler, Beat Jump, Slicer (reuse same 8 pads) |
| **I — Jog area** | P2 | Jog wheel / platter (touch or drag) |

---

## 5 — Feature Specification

### 5.1 Track metadata & status

| ID | Feature | Description | Competitors | Priority |
|----|---------|-------------|-------------|----------|
| M1 | **Title & artist** | Primary and secondary line; truncate with tooltip | All | P0 |
| M2 | **Album art** | Circular or square thumbnail; placeholder when missing | Rekordbox, Serato | P1 |
| M3 | **Duration** | Total track length | All | P0 |
| M4 | **Elapsed time** | Display `position_secs` as mm:ss.ms | All | P0 |
| M5 | **Remaining time** | `-mm:ss` from `duration_secs - position_secs` | All | P0 |
| M6 | **Original BPM** | From library analysis | All | P0 |
| M7 | **Effective BPM** | After pitch adjustment (`original × pitch_ratio`) | All | P1 |
| M8 | **Musical key** | e.g. `Gm`, `8A` (user preference) | All | P0 |
| M9 | **Sync state indicator** | Off / armed / tempo synced / beat synced / master | Serato, Rekordbox | P1 |
| M10 | **Track rating / color** | Optional library field on deck | Serato, Traktor | P3 |
| M11 | **Loading / analyzing state** | Spinner when decode or waveform job running | All | P0 |

**Data source:** `library` track row + live `DeckStatus` from engine.

---

### 5.2 Waveforms & visual navigation

See [`dj-waveform-spec.md`](dj-waveform-spec.md) for rendering details.

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| W1 | **Scrolling main waveform** | Fixed center playhead; spectral RGB | P0 (partial) |
| W2 | **Overview waveform** | Full-track strip above main lane; click to seek | P0 |
| W3 | **Beat grid overlay** | Vertical lines from `beat_grid`; downbeat emphasis | P0 |
| W4 | **Hot cue markers** | Colored flags on overview + scroll | P1 |
| W5 | **Loop region highlight** | Active loop bracket on waveform | P1 |
| W6 | **Zoom** | Adjust `visible_secs` (e.g. 4–64 s); mouse wheel or buttons | P1 |
| W7 | **Stacked dual-deck view** | Deck A lane above Deck B (current) | P0 |
| W8 | **Phase / beat phase indicator** | Small bar showing position within beat/bar (Serato) | P2 |
| W9 | **End-of-track warning** | Visual cue near track end | P2 |
| W10 | **Intro / outro markers** | From analysis phrases (Rekordbox) | P3 |

---

### 5.3 Transport & playback controls

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| T1 | **Play / Pause** | Toggle playback | P0 |
| T2 | **Cue (hold)** | Hold = temporary cue point audition; release = resume | P0 |
| T3 | **Cue (set)** | Set temporary cue at current position (Serato) | P1 |
| T4 | **Previous cue / jump to start** | Jump to first hot cue or track start | P2 |
| T5 | **Unload / eject** | Clear deck | P1 |
| T6 | **Reverse** | Play backward while held or toggled | P3 |
| T7 | **Emergency brake** | Instant stop + cue (hardware pattern) | P3 |

**Keyboard shortcuts** (Serato-style): cue, play, sync, hot cues 1–8 — map in GUI layer.

---

### 5.4 Tempo, pitch & sync

| ID | Feature | Description | Engine notes | Priority |
|----|---------|-------------|--------------|----------|
| P1 | **Pitch fader** | Vertical slider; selectable range ±6 / ±10 / ±50 % | Maps to `Deck::set_speed` | P1 |
| P2 | **Pitch bend buttons** | Momentary ± adjustment | Temporary speed offset | P2 |
| P3 | **Key lock / Master Tempo** | Change tempo without changing key | Requires time-stretch (not in MVP engine) | P2 |
| P4 | **Key shift** | ± semitones independent of tempo | Pitch shift DSP | P3 |
| P5 | **Sync (beat)** | Match phase and tempo to master deck | Compare beat grids + positions | P1 |
| P6 | **Sync (tempo only)** | Match BPM without phase lock | Adjust pitch fader target | P1 |
| P7 | **Sync (key)** | Harmonic match via key metadata | Key shift or reject incompatible | P2 |
| P8 | **Master deck** | One deck defines master BPM/phase | UI + engine flag | P1 |
| P9 | **BPM display (live)** | Updates during pitch fader move | Derived | P1 |
| P10 | **Snap pitch to BPM** | Optional: round effective BPM to 0.01 | UX nicety | P3 |

**Critical dependency:** True **key lock** and **quality tempo change** need a time-stretch engine (Rubber Band, SoundTouch, or phase-vocoder). Until then, pitch fader changes **both** tempo and key (classic vinyl behavior) and UI must label this honestly (“Vinyl tempo”).

---

### 5.5 Controller pads

The **8 numbered buttons** (slots 1–8) on each deck are **controller pads**, not “hot cue buttons only.” Industry software (especially **Virtual DJ**) reuses the same physical/UI pad grid for **multiple pad modes** selected via a dropdown or cycle control above the grid.

```text
┌─────────────────────────────────────────┐
│  [◀]  HOT CUE  [▶]     ← mode selector  │
├───────┬───────┬───────┬───────┤
│   1   │   2   │   3   │   4   │  row A
├───────┼───────┼───────┼───────┤
│   5   │   6   │   7   │   8   │  row B
└───────┴───────┴───────┴───────┘
```

**Default mode:** **Hot Cue** — matches Serato/Rekordbox behavior and our Phase 2 implementation.

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| PD1 | **Pad grid** | Fixed **8 slots** in **2×4** layout (rows 1–4 / 5–8); mirrored per deck (Deck A cues outer-left, Deck B outer-right) | P1 |
| PD2 | **Pad mode selector** | Dropdown or ◀/▶ cycle above grid (Virtual DJ pattern); shows current mode name | P2 |
| PD3 | **Per-deck active mode** | `pad_mode` stored in runtime deck state (not per track); MIDI maps to slot + mode | P2 |
| PD4 | **Mode-specific labels** | Pads show mode labels when set (e.g. Stems: Vocal, Kick; Hot Cue: time or user label) | P2 |
| PD5 | **Pad active state** | Visual on/off per pad (border highlight, underline color — Virtual DJ stems reference) | P2 |
| PD6 | **Empty pad affordance** | Unassigned pad shows slot number; assigned pad shows label/color | P1 |

#### Pad modes (target)

| Mode | Pad function | Persistence | Priority |
|------|--------------|-------------|----------|
| **Hot Cue** | Jump (or hotcue-loop) to stored point | `track_hot_cue` per slot | **P1 (default)** |
| **Loop Roll** | Temporary quantized loop while held | — | P2 |
| **Beat Jump** | Jump forward/back N beats | — | P2 |
| **Sampler** | Trigger one-shot from library / bank | Named 8-slot banks in `library.db` | P3 |
| **Stems** | Mute/solo/isolate stem (Vocal, Instru, Bass, Kick, HiHat, …) | Per-session | P3 |
| **Stems FX** | Stem-aware effect on pad | — | P4 |
| **Slicer** | Rhythmic slice/repeat | — | P4 |

**Virtual DJ reference (screenshot):** mode **STEMS** maps pads to Vocal, Instru, Bass, Kick, HiHat, Stems FX with colored underlines and toggle borders; **PADS** and **LOOP** appear as vertical side labels flanking the pad/loop areas.

#### Hot Cue mode (default)

When `pad_mode = hot_cue`, pads behave as hot cues:

| ID | Feature | Description | Count | Priority |
|----|---------|-------------|-------|----------|
| C1 | **Hot cues** | Instant jump; optional stored loop | 8 (Serato) → 16 (Rekordbox) | P1 |
| C2 | **Hot cue color** | User-selectable palette | — | P1 |
| C3 | **Hot cue label** | Short text (e.g. “Drop”, “Intro”) | — | P1 |
| C4 | **Hot cue set / delete** | Empty pad = set at playhead; shift+click = delete | — | P1 |
| C5 | **Memory cues** | Non-destructive timeline markers (Rekordbox) | 10 | P2 |
| C6 | **Cue quantize on set/trigger** | Snap to beat grid when quantize on | — | P1 |
| C7 | **Persist cues in library** | Save per track_id; load on deck load | — | P1 |
| C8 | **Intelligent / auto cues** | Analysis-suggested cues (Rekordbox 7) | — | P3 |

**Interaction model:** numbered 1–8 grid; show time + label when set; green = cue, orange = loop cue (Rekordbox convention). Keyboard shortcuts 1–8 trigger pad in **current mode** (Hot Cue in Phase 2).

**Implementation note:** Current rust-mixer code (`DeckPadsPanel`, `track_hot_cue`, `save_hot_cue`) implements **Hot Cue mode only** with mode selector placeholder.

---

### 5.6 Loops

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| L1 | **Auto loop** | Quantized loop of N beats (1, 2, 4, 8, 16, 32) | P1 |
| L2 | **Loop in / out** | Manual set in and out points | P1 |
| L3 | **Loop halve / double** | ÷2 / ×2 current length | P1 |
| L4 | **Loop roll** | Temporary loop while held (Serato) | P2 |
| L5 | **Saved loops** | Named loops persisted per track | P2 |
| L6 | **Active loop on waveform** | Visual bracket + beat count | P1 |
| L7 | **Reloop / exit loop** | Toggle loop off; optional slip exit | P1 |

Loop engine must **wrap read position** within `[in, out)` while optionally advancing **shadow position** for slip mode.

---

### 5.7 Beat grid & quantize

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| G1 | **Beat grid display** | From library analysis | P0 |
| G2 | **Grid edit mode** | Adjust downbeat / BPM (Traktor, Serato) | P3 |
| G3 | **Quantize toggle (Q)** | Snap cue/loop/hotcue to grid | P1 |
| G4 | **Quantize value** | 1/2 beat, 1 beat, 1 bar | P1 |
| G5 | **Phase nudge** | Micro-adjust phase vs master (± ms) | P2 |
| G6 | **Tap tempo** | Manual BPM override | P3 |

---

### 5.8 Mixer channel (per deck)

Currently in center `DeckMixer`; may stay centralized or duplicate mini-strips on wide layouts.

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| X1 | **Channel volume fader** | 0–100% | P0 |
| X2 | **3-band EQ** | Low / mid / high kill or ± dB | P0 |
| X3 | **Filter (HP/LP)** | Single knob wet/dry or crossfade | P1 |
| X4 | **Gain trim** | Pre-fader level; persisted per track (Serato) | P2 |
| X5 | **VU / level meter** | Peak or RMS per deck | P2 |
| X6 | **Crossfader assign** | A / B / thru (4-deck future) | P2 |
| X7 | **Channel fader curve** | Configurable crossfader law | P3 |

---

### 5.9 Effects (FX)

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| F1 | **Filter effect** | DJ-style one-knob HP/LP (Traktor) | P1 |
| F2 | **FX slot 1–3** | Insert or send; dropdown selection | P2 |
| F3 | **FX parameters** | 1–3 knobs per effect | P2 |
| F4 | **FX on/off & wet/dry** | Per slot | P2 |
| F5 | **Beat-synced FX** | LFO synced to deck BPM | P3 |
| F6 | **FX favorites / banks** | User presets | P3 |

**MVP FX list (industry common):** Filter, Echo/Delay, Reverb, Flanger, Phaser, Bit Crusher, Roll (loop + decay).

**Engine placement:** Per-deck pre-fader insert chain in `engine-dsp` before mixer bus.

---

### 5.10 Additional pad modes (Stems, Sampler, …)

Pad modes beyond **Hot Cue** reuse the same 8-slot grid (§5.5). This section details non–hot-cue modes.

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| S1 | **Stem mute toggles** | Vocal, instrumental, bass, drums, hihat (Virtual DJ Stems mode) | P3 |
| S2 | **Stem isolation gain** | Per-stem level 0–100% | P3 |
| S3 | **Sampler pads** | Trigger one-shots from persisted named banks (8 slots); see `docs/superpowers/specs/2026-07-22-sampler-banks-design.md` | P3 |
| S4 | **Slicer / beat repeat** | Chop playing deck into rhythmic slices | P3 |
| S5 | **Loop roll pad mode** | Beat-quantized temporary loop per pad | P2 |
| S6 | **Beat jump pad mode** | ±N beats per pad | P2 |

**Dependency:** Offline or real-time stem separation (Rekordbox Stems, Serato Stems, Virtual DJ stems). Requires separate analysis pipeline or third-party model — **not** in current analyzer MVP.

**Removed from this section:** pad mode selector and grid layout — defined in §5.5 (controller pads are the primary abstraction).

---

### 5.11 Jog / scratch / vinyl mode

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| J1 | **Jog wheel (UI)** | Drag to seek/scratch; visual rotation | P2 |
| J2 | **Vinyl mode** | Touch top = stop platter; outer ring = scratch | P2 |
| J3 | **CDJ mode** | Constant-speed jog vs vinyl | P3 |
| J4 | **Scratch algorithm** | Interpolation + motor model; low latency | P2 |
| J5 | **Jog sensitivity** | Configurable | P3 |
| J6 | **Platter animation** | Sync rotation to effective BPM | P2 |

**Engine:** Jog updates `seek` + brief speed override; scratch requires small buffer and high callback rate.

---

### 5.12 Slip mode & advanced transport

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| A1 | **Slip mode** | Shadow playhead continues during loop/scratch/cue; catch up on exit ([Serato manual](https://serato.com/dj/pro)) | P2 |
| A2 | **Censor / censor button** | Temporary reverse or mute (Serato) | P3 |
| A3 | **Brake / spin down** | Vinyl stop effect | P3 |

---

### 5.13 Beat jump & navigation

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| B1 | **Beat jump forward/back** | ±1, 2, 4, 8, 16, 32 beats | P2 |
| B2 | **Seek via overview click** | Jump to timestamp | P0 |
| B3 | **Seek via waveform drag** | Scrub playhead | P1 |
| B4 | **Bar / phrase jump** | Jump to next analyzed phrase boundary | P3 |

---

### 5.14 Library & load workflow

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| D1 | **Load from library** | Drag track or double-click | P0 |
| D2 | **Load from file picker** | Current behavior | P0 |
| D3 | **Instant double** | Load same track on other deck | P2 |
| D4 | **Load to play from cue** | Start at hot cue 1 or first memory cue | P2 |
| D5 | **Analyze on first load** | BPM/key/grid if missing | P0 (partial) |
| D6 | **Prepare / pre-load** | Decode next track in background | P3 |

---

### 5.15 Headphone cue / PFL

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| H1 | **Cue button per deck** | Route pre-fader audio to preview bus | P2 |
| H2 | **Cue mix knob** | Master vs cue balance (mixer section) | P2 |
| H3 | **Split cue** | Mono split left=master right=cue | P3 |

**Engine:** Requires preview bus routing (config exists in settings; engine routing incomplete per main tech spec).

---

### 5.16 Hardware & MIDI

| ID | Feature | Description | Priority |
|----|---------|-------------|----------|
| HW1 | **MIDI map deck controls** | Learn mode; maps to `EngineCommand` → same events as UI (§9) | P3 |
| HW2 | **HID controller profiles** | Rekordbox / Serato compatible devices | P4 |
| HW3 | **Motorized fader feedback** | — | P4 |
| HW4 | **Low-latency WASAPI/ASIO** | Windows pro audio | v2 (main spec) |

---

## 6 — Data Model

### 6.1 Persisted per track (`library.db`)

Deck-specific data lives in **dedicated tables** in the same `library.db` as waveforms — not as extra columns on `tracks` or `track_analysis`. Same pattern as `track_waveform` (see [`dj-waveform-spec.md`](dj-waveform-spec.md) §9.4).

```sql
-- Existing / planned
track_analysis (bpm, key, duration, ...)
track_beat_grid (beat timestamps, downbeats)
track_waveform (overview blob)          -- see dj-waveform-spec.md

-- Deck performance data (separate tables, CASCADE on track delete)
CREATE TABLE IF NOT EXISTS track_hot_cue (
    track_id            TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    slot_index          INTEGER NOT NULL,   -- 0..7 (expand to 15)
    position_secs       REAL NOT NULL,
    loop_length_beats   INTEGER,            -- NULL = jump cue; set = hotcue loop
    color               TEXT,
    label               TEXT,
    updated_at          TEXT NOT NULL,
    PRIMARY KEY (track_id, slot_index)
);

CREATE TABLE IF NOT EXISTS track_loop (
    track_id            TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    slot_index          INTEGER NOT NULL,
    in_secs             REAL NOT NULL,
    out_secs            REAL NOT NULL,
    label               TEXT,
    color               TEXT,
    updated_at          TEXT NOT NULL,
    PRIMARY KEY (track_id, slot_index)
);

-- Optional later
track_deck_prefs (
  track_id, gain_trim_db, last_pitch_percent, last_key_shift,
  PRIMARY KEY (track_id)
);
```

| Table | Rows per track | Written by |
|-------|----------------|------------|
| `track_waveform` | 1 | `analyze_track` |
| `track_hot_cue` | 0–8+ (one per slot) | `save_hot_cue` (Hot Cue **pad mode** only) |
| `track_loop` | 0–N (one per slot) | `save_loop` |

Load path: when a track is loaded to a deck, read all `track_hot_cue` / `track_loop` rows for that `track_id` into engine memory. No separate bulk `load_*` command required unless we add lazy fetch later.

### 6.2 Runtime per deck (engine)

Extend `Deck` in `engine-dsp` with:

- `transport`: cue point, reverse, slip shadow position
- `loop`: active region, roll state; saved loops via **`save_loop`** → `track_loop` table
- `sync`: master reference, tempo target
- `pads`: `{ mode, slots[8] }` — mode-specific runtime state; **hot_cue** mode loads from **`track_hot_cue`**
- `fx_chain`: ordered effects
- `scratch/jog`: transient speed override

### 6.3 GUI view model

**Today** (`DeckStatus` in `apps/gui-app/src/types.ts`):

| Field | Type | Notes |
|-------|------|-------|
| `position_secs` | `number \| null` | Live playhead; null when no track loaded |
| `duration_secs` | `number \| null` | From loaded track; null when unknown |
| `playing`, `volume`, `eq`, `track`, `track_id` | — | Already wired |

**Target:** extend `DeckStatus` to expose everything in §4 for 60 fps-safe polling (waveform remains separate invoke). `remaining_secs` stays a UI-derived field unless we add it to the API for convenience.

---

## 7 — Engine vs GUI Responsibilities

| Concern | Engine (Rust) | GUI (React) |
|---------|---------------|-------------|
| Audio playback, EQ, FX | Yes | Controls only |
| Beat grid math, sync, quantize | Yes | Display grid lines |
| Waveform rasterization | Yes ([dj-waveform-spec.md](dj-waveform-spec.md)) | Blit images |
| Hot cue / loop persistence | Via `library` commands (hot cues = pad mode data) | Pad UI + mode selector |
| Keyboard shortcuts | — | Yes |
| Jog/scratch gesture | Receives delta commands | Pointer events |
| State sync to UI | **Emit engine events** (§9) | Subscribe; render only |
| Toast / errors | Returns structured errors | coss toast |

**Rule:** Any action that affects audio within **<10 ms** must be engine-side; UI sends commands, never computes audio.

**Rule:** The UI must **not** assume it is the only writer of engine state. All consumers (React, future MIDI mapper, future OSC) observe the same **event stream** from the backend (§9).

---

## 8 — API Surface

### 8.1 Current (implemented)

There is **no** `get_deck_state` command. Authoritative deck/engine state for the UI is the **evt omnibus** (`engine://bus`), mirrored in the Zustand store.

```text
EngineStatus (hydrate + status events)
├── running, backend, sample_rate, crossfader, cue_mix, master_cue, master_deck?
└── decks: DeckStatus[]     // one entry per deck (0 = A, 1 = B)
        ├── id, track, track_id, title, artist, bpm, key, playing, volume, eq, …
        ├── position_secs, duration_secs, hot_cues, loops, sampler bank, …
        └── levels (from high-rate levels events)
```

| Path | Role |
|------|------|
| `EngineTransport.publish` → `engine_publish` | All engine cmds (transport, mixer, load, pads, sampler, **start_engine**, …) |
| `EngineTransport.subscribe` → `engine://bus` | Status / updated / position / levels / notice / error (store owns subscribe) |
| Settings / devices / FS | Non-engine host APIs (`get_settings`, `save_settings`, device list, …) |
| `LibraryTransport` | Tracks, artwork, waveform raster (not on the engine bus) |

Deck mutations do **not** return `DeckStatus` for the UI to merge. The store updates from bus events. There is no `get_status` hydrate — status arrives after `start_engine` emits on the bus.

**Not planned:** a separate `get_deck_state`. Prefer richer `DeckSnapshot` / `EngineStatus` on the bus.

### 8.2 Engine cmds (via `publishCmd` / omnibus)

Host-handled (library + `AppState` in `bus_bridge`, then emit on bus): load path / library track, sampler bank assign/clear/select, related bank CRUD.

Engine-native (forwarded to cmd omnibus): play/pause, seek, volume/EQ/speed, crossfader, cue mix, pads, sync, sampler trigger, etc.

### 8.3 Library persistence

One row per cue/loop slot, same DB as waveforms:

```text
save_hot_cue(track_id, slot_index, position_secs, loop_length_beats?, color?, label?)
delete_hot_cue(track_id, slot_index)

save_loop(track_id, slot_index, in_secs, out_secs, label?, color?)
delete_loop(track_id, slot_index)
```

No bulk `save_hot_cues` / `save_loops` — each user action upserts or deletes one row. `delete_*` clears the slot; engine reload picks up changes on next track load (or immediately if we add a refresh hook).

---

## 9 — Engine Event System

**Current implementation:** [`2026-07-26-engine-event-bus-design.md`](superpowers/specs/2026-07-26-engine-event-bus-design.md) — engine-owned **omnibus** cmd/evt buses, MessagePack wire, Tauri bridges bytes only, frontend **`EngineTransport`**. The JSON `engine://event` path is **retired**; runtime traffic is `engine_publish` / `engine://bus` only.

Library metadata / decode / waveform / artwork stay on **`LibraryTransport`** (separate from the engine bus). Hosts prepare playback via `LibraryManager` → `PreparedTrackPlayback` → `Engine::load_prepared_track`, without holding `AppState` across decode.

### 9.1 Problem

UI-only request/response breaks when:

- A **MIDI controller** adjusts volume, pitch, or transport on a background thread.
- **Sync logic** changes deck tempo without a matching UI action.
- **End-of-track** or **loop wrap** updates transport state from the audio thread.
- Two UI surfaces must stay in sync without duplicate invokes.

The UI must subscribe to **engine-originated changes**, not only refresh after its own commands.

### 9.2 Design goals

| Goal | Approach |
|------|----------|
| Single source of truth | Engine state lives in Rust; UI is a read-only mirror |
| Any input path | UI, MIDI, keyboard shortcuts, automation → same cmd bus |
| Push, not poll | `engine://bus` for discrete + high-rate kinds |
| Efficient | Coalesce noisy sources (MIDI CC); don’t emit full status at audio rate |
| Testable | Headless omnibus + `MemoryEngineTransport` without a window |

### 9.3 Architecture

```text
UI / MIDI / host
      │
      ▼
EngineTransport.publish  →  invoke("engine_publish")  →  cmd omnibus
                                                              │
                                                     control thread
                                                              │
                                                              ▼
                                                         evt omnibus
                                                              │
                                                     Tauri forwarder
                                                              │
                                                              ▼
                                              emit("engine://bus", bytes)
                                                              │
                                                              ▼
                                   EngineTransport.subscribe → applyBusEvent → store
```

Host-only cmds (load path/library track, sampler bank persistence) are handled in `bus_bridge` (library + `AppState`), then emit rich `status` / `updated` payloads on `engine://bus`. Engine-native cmds forward to the omnibus.

### 9.4 Event kinds (wire)

Conceptual kinds on the evt bus (MessagePack envelope with `origin`, `kind`, `revision`, `body`):

| Kind | Role |
|------|------|
| `status` | Full snapshot (start/stop, crossfader, multi-deck) |
| `updated` | Single-deck patch (preferred for transport/mixer tweaks) |
| `position` | High-rate playhead |
| `levels` | High-rate VU |
| `notice` / `error` | Non-fatal / session-fatal |

Increment **`revision`** on every emit so the UI can ignore out-of-order duplicates.

**Coalescing:** MIDI CC floods may be coalesced to **≤60 Hz** per `(deck_id, control)` before emit.

### 9.5 When to emit

| Source | Kind |
|--------|------|
| `start_engine` / `stop_engine` | `status` |
| load, play/pause, seek, cue, pads, sampler | `updated` or `status` |
| volume / EQ / pitch / FX | `updated` |
| crossfader / cue mix / master cue | `status` |
| MIDI mapping (future) | Same as equivalent cmd |
| Track ended / loop wrap | `updated` |
| Engine error | `error` |

### 9.6 Position updates vs full status

`position_secs` changes continuously during playback. **Current:** high-rate `position` (and `levels`) on `engine://bus`; waveform hooks extrapolate between updates. Do not resurrect a separate `engine://position` channel unless the bus path is insufficient.

### 9.7 UI integration

```text
Mount / enter decks
  └─ engine store ensureEngineRunning
       ├─ await EngineTransport.subscribe (engine://bus → applyBusEvent)
       └─ publishCmd("engine", "start_engine")   // host emits status

User action (e.g. play)
  ├─ EngineTransport.publish(...)      // fire-and-forget OK
  └─ UI updates from bus events, not command return values
```

Library hooks use `LibraryTransport` (not raw `invoke`) for tracks, artwork, waveforms.

### 9.8 MIDI (future consumer)

```text
MIDI IN → map to cmd → publish on cmd omnibus → evt omnibus → UI
```

Same events the UI sees from mouse clicks.

### 9.9 Implementation notes (Rust)

- Audio callback must **not** emit to Tauri — control thread / forwarder only.
- Host load path: prepare (`prepare_*_for_playback` / `ensure_track_waveform` on `&Mutex<LibraryManager>`) **outside** `AppState` and without holding `library` across decode/waveform generation; emit bus payload **after** unlock; sampler bank select after first deck emit.
- Never hold `AppState` while waiting on `library` (starves every other host command).
- Unit tests: headless session + `MemoryEngineTransport` / `MemoryLibraryTransport`.

---

## 10 — Phased Roadmap

### Phase 1 — “Real DJ app shell” (current focus)

Make **what we already have** reliable and **look like** professional deck software (Rekordbox / Serato / Traktor layout), without new engine features yet.

**Engine / behavior (fix & wire existing):**

- Stable load → play/pause on both decks (file picker + library drag-drop)
- `position_secs` / `duration_secs` polled and shown (elapsed + remaining)
- Volume faders, 3-band EQ, crossfader — responsive, no stale UI
- Waveform scroll tracks playhead smoothly during playback
- Engine auto-start + errors via coss toasts (done)
- Load library track metadata: **title, artist, BPM, key** on deck (from `TrackSummary` / analysis, not just filename)
- **Engine event bus (§9):** `engine://bus` via `EngineTransport`; UI subscribes in bootstrap (foundation for MIDI)

**UI layout (visual parity, placeholders OK):**

- **Metadata bar** per deck: title, artist, BPM, key, elapsed / remaining / total
- **Deck chrome**: accent colors, transport row (play/pause prominent; cue/sync as disabled placeholders)
- **Jog / platter** area: visual only (rotation tied to BPM when playing)
- **Mixer column** between decks (desktop): faders + EQ + crossfader — already present; polish spacing and labels
- **Waveform stack** on top: dual scrolling lanes (done); reserve space for overview strip (can be empty or low-res overview until Phase 2)
- Responsive: deck controls usable at common window sizes

**Explicitly not Phase 1:** hot cues, loops, sync, pitch fader, beat grid overlay, FX, stems, scratch, PFL.

### Phase 2 — “Performance controls” (P1)

- Overview waveform + click seek
- Beat grid overlay on scroll lane
- Cue button (hold) + seek/scrub on waveform
- **Controller pads** in **Hot Cue mode** (default): set, trigger, delete + **`save_hot_cue`** / **`delete_hot_cue`** → `track_hot_cue` table
- Auto loop + manual loop in/out + **`save_loop`** / **`delete_loop`** → `track_loop` table
- Quantize toggle
- Unload / eject track
- Pitch fader (vinyl-style speed) + effective BPM display

### Phase 3 — “Sync & mix tools” (P2)

- Beat sync + master deck
- **Pad mode selector** (PD2); Loop Roll + Beat Jump pad modes
- Loop halve/double, beat jump
- Filter knob (audio + optional waveform tint per dj-waveform-spec §8.6)
- Key display modes (musical / Camelot)
- Album art
- Gain trim per track

### Phase 4 — “Pro features” (P3)

- Slip mode
- Key lock / time-stretch (requires DSP crate)
- FX slots (filter + echo + reverb)
- Jog wheel / scratch (functional)
- Cue/PFL routing to preview bus
- VU meters
- Memory cues

### Phase 5 — “Differentiators” (P4+)

- **Stems / Sampler / Slicer pad modes** (§5.10)
- Intelligent cues
- Grid editor
- **MIDI mapping** (consumes §9 event bus + shared `EngineCommand` path)
- 4-deck layout

---

## 11 — Acceptance Criteria

**Phase 1 complete when:**

1. Both decks: load (picker + drag-drop), play, pause work reliably with no silent failures.
2. Deck UI shows **title, artist, BPM, key**, **elapsed** (`position_secs`), **remaining**, and **total** (`duration_secs`).
3. Layout reads as a **DJ app**: waveform stack → deck panels → center mixer; transport and platter visible per deck.
4. **Volume, EQ, crossfader** reflect engine state; changes apply without glitching audio.
5. Scrolling **waveforms track the playhead** during playback without visible drift vs. audio.
6. Disabled placeholders for future controls (cue, sync, hot cues) do not clutter — clear “coming later” or omitted until Phase 2.
7. Engine errors use **coss toasts** only.
8. **`engine://bus`** delivered to UI: headless / simulated publish updates React state without a matching UI invoke return value.

**Phase 2 adds:** overview, beat grid, **pads in Hot Cue mode**, loops with **`save_hot_cue`** / **`save_loop`** persistence; high-rate position already on the bus (§9.6).

---

## 12 — References

### Competitor documentation

| Resource | URL |
|----------|-----|
| Rekordbox 7 introduction | https://cdn.rekordbox.com/files/20260409151246/rekordbox7.2.14_introduction_EN.pdf |
| Rekordbox 7 manual (hot cues, analysis) | https://cdn.rekordbox.com/files/20241213141602/rekordbox7.0.7_manual_EN.pdf |
| Rekordbox features (sync, layouts) | https://rekordbox.com/en/feature/style/ |
| Serato DJ Pro features | https://serato.com/dj/pro |
| Serato DJ Pro user manual (cue, loop, slip, sync) | https://d1aeri3ty3izns.cloudfront.net/media/36/366330/download_366330.pdf |

### This repository

| Resource | Path |
|----------|------|
| Engine deck DSP | `crates/engine-dsp/src/deck.rs` |
| GUI deck panel | `apps/gui-app/src/components/DeckPanel.tsx` |
| GUI deck grid | `apps/gui-app/src/components/DeckGrid.tsx` |
| Waveform spec | `docs/dj-waveform-spec.md` |
| Analyzer / beat grid | `docs/audio-analyzer-spec.md` |

### Reference screenshots (session)

- Rekordbox-style: FX row, stem pads, hot cues, loop controls, sync, pitch — `assets/image-42583e66-*.png`
- DJUCED-style: labeled hot cues, key sync/shift, stem mute, master sync — `assets/image-27f8012a-*.png`
- Traktor-style: FX assign, colored hot cues 1–8, filter, sync, loop on waveform — `assets/image-f960b3e2-*.png`
- Virtual DJ-style: **STEMS pad mode**, mode dropdown, 2×4 pad grid, PADS/LOOP side labels — `assets/image-7f761d7a-*.png`

---

## Decision log (initial)

| # | Topic | Decision |
|---|--------|----------|
| DK1 | Pad count | **8 slots** (2×4 grid); schema allows **16** for Hot Cue mode expansion |
| DK12a | Pad abstraction | **8 controller pads** with **mode selector**; **Hot Cue = default mode** (Virtual DJ / Serato model); `track_hot_cue` stores Hot Cue mode data only |
| DK2 | Key lock | **Deferred** until time-stretch exists; pitch fader = vinyl mode |
| DK3 | Waveform EQ link | Static analysis colors MVP; optional EQ tint post-MVP (dj-waveform-spec) |
| DK4 | Stems | **Phase 4**; separate spec when chosen |
| DK5 | Deck layout | **Stacked waveforms + side mixer** (current); optional single-deck expanded view later |
| DK6 | Cue persistence | **`track_hot_cue`** table; **`save_hot_cue`** per slot (Phase 2) |
| DK7 | Loop persistence | **`track_loop`** table; **`save_loop`** per slot (Phase 2) |
| DK8 | Deck state API | Bus `status` / `updated` snapshots — no `get_deck_state`; no `get_status` hydrate |
| DK9 | Error UX | **coss toast**; engine start uses **promise toast** |
| DK10 | Phase 1 scope | **Polish existing features + DJ app look** — no new performance engine features |
| DK11 | Position stream | Poll in Phase 1; **`engine://position`** push in Phase 2 |
| DK12 | State sync | **Event bus** — UI subscribes; all inputs use `EngineController::apply` |
