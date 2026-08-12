# Flutter level meter (Tauri parity, visual only)

Date: 2026-08-12  
Status: accepted  
Depends: Flutter mixer strip layout; Tauri VU design `2026-07-13-vu-level-meters-design.md`

## Goal

Port the Tauri `LevelMeter` UI to Flutter as a reusable widget and wire it into `MixerStrip` between the volume faders, with mono/stereo display. Props only — no engine / FRB levels yet (meters stay at zero until a later bus task).

## Decisions

| Topic | Choice |
|--------|--------|
| Scope | Widget + MixerStrip wiring with `zeroDeckLevels` |
| API | Controlled `LevelMeter({ levels, mode })` (not `deckId` + store) |
| Look / math | Match Tauri: 12 segments, YELLOW_FROM=8, RED_FROM=10, hold index math |
| Colors | Idle: Forui `muted`; lit: Tauri emerald/amber/red opacities |
| Mono | `max(L,R)` for peak and hold |
| Stereo | Two ladders side-by-side (width 6 each, 1px gap) |
| Mode toggle | Existing MixerStrip M/S button (local state) |
| Engine / FRB | Out of scope |

## Architecture

```text
apps/gui-flutter/lib/mixer/level_meter.dart   # DeckLevels, mode, LevelMeter, ladder math
apps/gui-flutter/lib/mixer/mixer_strip.dart   # replace _IdleLevelMeter; pass zeros
apps/gui-flutter/test/level_meter_test.dart   # segment/hold math + mono/stereo smoke
```

### Types

```dart
enum LevelMeterMode { mono, stereo }

class DeckLevels {
  final double peakL;      // linear 0..1
  final double peakR;
  final double peakHoldL;
  final double peakHoldR;
}

const zeroDeckLevels = DeckLevels(
  peakL: 0, peakR: 0, peakHoldL: 0, peakHoldR: 0,
);
```

### Ladder

- Column of 12 equal `Expanded` segments, 1px gaps, `borderRadius: 1`, width 6 (`w-1.5`).
- Index from bottom (segment 0 = quietest), same as Tauri `flex-col-reverse` + `fromBottom`.
- `segmentOn(level, i)` / `holdSegment(hold)` — port Tauri thresholds and idle epsilon.
- Lit segment or peak-hold tick uses band color; otherwise zinc-800.

### MixerStrip

- `_LevelMetersColumn` keeps GAIN/%/cue spacers for fader alignment.
- Two `LevelMeter`s (deck A / B) with `zeroDeckLevels` and current mono/stereo mode.
- Delete `_IdleLevelMeter`.

## Non-goals

- Engine `Levels` events, FRB streams, Riverpod/store for peaks
- Master / PFL meters, numeric dB readout
- Persisting mono/stereo preference
- CustomPainter optimization
- New dependencies

## Testing

- Unit: `segmentOn` / `holdSegment` edge cases (sub-threshold hold → null; red band indices).
- Widget smoke: mono builds one ladder; stereo builds two; zero levels → all off segments.
- Existing mixer shell `widget_test` still passes.

## Acceptance

- [ ] Mixer shows Tauri-style ladders between faders (idle dark at zeros)
- [ ] M/S toggle switches one vs two ladders per deck
- [ ] Segment/hold math matches Tauri (covered by unit test)
- [ ] No engine/FRB changes
