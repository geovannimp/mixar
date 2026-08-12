# Flutter deck pads (Tauri parity, UI chrome)

Date: 2026-08-12  
Status: accepted (user waived per-section + written-spec review gates)  
Depends: Flutter desktop host; Tauri `DeckPadsPanel` as reference

## Goal

Port Tauri deck pads into `apps/gui-flutter` as UI chrome: mode tabs + four mode grids (hot cue, loop roll, beat jump, sampler with bank toolbar). Local state only — no FRB / engine bus.

## Decisions

| Topic | Choice |
|--------|--------|
| Scope | UI chrome mirroring Tauri layout + interactions |
| Structure | `DeckPadsPanel` + `pads/` mode widgets (Tauri file split) |
| State | Local in `DeckPadsPanel` (pad mode, demo hot cues, demo sampler banks/slots) |
| Sampler chrome | Full bank toolbar + settings dialog stub (no DnD assign) |
| Engine / FRB | Out of scope |
| Track gating | `hasTrack` prop; DeckPanel passes `false` while “No track loaded” — pad actions disabled, mode tabs still work |

## Architecture

```text
apps/gui-flutter/lib/mixer/
  deck_panel.dart              # embed DeckPadsPanel (replace placeholder)
  deck_pads_panel.dart         # mode tabs + switch on PadMode
  pad_modes.dart               # PadMode, labels, beat tables
  pads/
    pad_grid.dart              # 4×2 grid shell
    pad_button.dart            # pad-sized button + hot-cue accents
    hot_cue_pads.dart
    loop_roll_pads.dart
    beat_jump_pads.dart
    sampler_pads.dart
  pad_format.dart              # mm:ss.t time label helper (minimal)
apps/gui-flutter/test/
  pad_modes_test.dart          # beat tables + mode cycle
```

## Behavior (match Tauri)

- Tabs: Cue / Roll / Jump / Sample (`PAD_MODE_SHORT_LABELS`).
- Hot cue: empty → save; filled → trigger; Shift+click → delete; 8 slot accent colors.
- Loop roll: pointer down begin / up|leave end; beats `[1,2,4,8,16,32,64,128]`.
- Beat jump: slots 0–3 forward `[1,2,4,8]`, 4–7 back `[-1,-2,-4,-8]`.
- Sampler: ◀/▶ banks, name, play-mode badge, gear → local name/play-mode dialog; hold-like vs one-shot; Shift+click clear. No track drop zones yet.

## Non-goals

- Engine cmds / MessagePack bus / FRB pad APIs
- Library DnD onto sampler pads
- Real hot-cue persistence
- Stems / slicer modes

## Acceptance

- [ ] Deck panel shows mode tabs + 8-pad grid instead of numbered placeholder
- [ ] Switching modes swaps grid content (hot cue / roll / jump / sampler)
- [ ] Sampler shows bank toolbar chrome
- [ ] `pad_modes` unit test covers beat tables and mode ordering
- [ ] Existing Flutter widget test still passes
