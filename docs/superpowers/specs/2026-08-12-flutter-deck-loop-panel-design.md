# Flutter deck loop panel (Tauri parity)

Date: 2026-08-12  
Status: accepted  
Depends: Flutter deck pads / tempo shells

## Goal

Port Tauri `DeckLoopPanel` into the Flutter deck as a local-state UI shell under the pads panel. Forui theme/components only. No engine or library wiring.

## Decisions

| Topic | Choice |
|--------|--------|
| Scope | UI shell + mount in `DeckPanel` |
| Look | Tauri control layout; Forui palette (`theme.colors.*`) and `FButton` |
| Secondary actions | Deferred — no shift-save / shift-delete |
| State | Local `loopActive` + `loopBeats` in the panel |
| Engine / FRB / library | Out of scope |

## Placement & chrome

In each `DeckPanel` body row:

```
[ pads ]
[ loop ]   | jog
```

- Full-width strip under the pads panel (same column as pads; jog stays beside)
- Bordered / rounded / translucent background like `DeckTempoPanel`
- When `loopActive`: use Forui primary (or secondary) border/fill tint — **not** hardcoded emerald
- Disabled when `!hasTrack` or `disabled` (deck currently passes `hasTrack: false`)

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
apps/gui-flutter/lib/mixer/deck_loop_panel.dart   # panel + beat-step helpers
apps/gui-flutter/lib/mixer/deck_panel.dart        # mount under pads
apps/gui-flutter/test/deck_loop_beats_test.dart   # step list + clamp
```

## Non-goals

- Engine cmds (`setDeckAutoLoop`, in/out, exit, beat jump)
- Saved loops / shift-click save-delete
- Waveform loop markers
- Loop-roll pad mode changes
- New dependencies

## Acceptance

- [ ] Loop strip appears under the pads panel
- [ ] Loop / length / IN / OUT update local active + beats state
- [ ] Controls disabled with no track
- [ ] Active state uses Forui theme colors
- [ ] Beat-step unit test covers list + halve/double clamps
