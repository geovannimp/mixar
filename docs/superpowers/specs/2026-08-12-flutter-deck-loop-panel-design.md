# Flutter deck loop panel (Tauri parity)

Date: 2026-08-12  
Status: accepted  
Depends: Flutter deck pads / tempo shells

## Goal

Port Tauri `DeckLoopPanel` into the Flutter deck as a local-state UI shell, reached via an exclusive left performance-mode rail (Pads / Loop). Forui theme/components only. No engine or library wiring.

## Decisions

| Topic | Choice |
|--------|--------|
| Scope | UI shell + mount in `DeckPanel` via `DeckPerformancePanel` |
| Look | Tauri loop controls; Forui palette (`theme.colors.*`) and `FButton` |
| Placement | Left vertical rail toggles Pads vs Loop (extensible); exclusive content |
| Secondary actions | Deferred — no shift-save / shift-delete |
| State | Local `loopActive` + `loopBeats` in the loop panel; `IndexedStack` keeps mode state |
| Engine / FRB / library | Out of scope |

## Placement & chrome

```
[ Pads ] | <pads or loop body>  | jog
[ Loop ] |
```

- `DeckPerformancePanel` owns outer chrome + left rail (`DeckPerformanceMode`)
- Default mode: Pads
- Active rail item: Forui secondary tint; labels rotated (Virtual DJ–style)
- Loop body: 2×2 (`Loop | size` / `IN OUT | ±4`), fills content area
- Disabled when `!hasTrack` or `disabled`

## Controls & behavior

Beat lengths: `[1, 2, 4, 8, 16, 32]`, default `4`.

| Control | Shell behavior |
|---------|----------------|
| Loop | Toggle `loopActive`; enable uses current `loopBeats` |
| ‹ / › | Step beat list; if already active, stay active at new length |
| Beats readout | Display only |
| IN / OUT | Set `loopActive = true` |
| −4 / +4 | Enabled when track loaded; no-op otherwise |

Active Loop / ‹ › / IN / OUT use Forui secondary (pressed) styling when `loopActive`.

## Architecture

```
apps/gui-flutter/lib/mixer/performance_modes.dart       # DeckPerformanceMode enum
apps/gui-flutter/lib/mixer/deck_performance_panel.dart  # left rail + IndexedStack
apps/gui-flutter/lib/mixer/deck_loop_panel.dart         # loop body + beat-step helpers
apps/gui-flutter/lib/mixer/deck_pads_panel.dart         # bordered: false when embedded
apps/gui-flutter/lib/mixer/deck_panel.dart              # mounts DeckPerformancePanel
apps/gui-flutter/test/deck_loop_beats_test.dart         # step list + clamp
```

## Non-goals

- Engine cmds (`setDeckAutoLoop`, in/out, exit, beat jump)
- Saved loops / shift-click save-delete
- Waveform loop markers
- Loop-roll pad mode changes
- New dependencies

## Acceptance

- [ ] Left rail toggles exclusive Pads / Loop content
- [ ] Loop / length / IN / OUT update local active + beats state
- [ ] Controls disabled with no track
- [ ] Active rail + loop chrome use Forui theme colors
- [ ] Beat-step unit test covers list + halve/double clamps
- [ ] Adding a future rail mode only requires enum + body entry
