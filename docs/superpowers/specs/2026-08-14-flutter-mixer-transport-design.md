# Flutter mixer strip → EngineTransport

Date: 2026-08-14  
Status: accepted (user waived remaining review gates until PR)  
Depends: `2026-08-13-flutter-engine-transport-design.md`, `2026-08-14-flutter-track-dnd-design.md`

## Goal

Wire the Flutter center mixer strip to `EngineTransport`: EQ / filter / gain / volume / headphone cue / crossfader send typed cmds; knobs follow expanded thin `EngineEvt`; VU meters follow `Levels` (including peak-hold). Cue mix and master cue stay out.

## Decisions

| Topic | Choice |
|--------|--------|
| Scope | Strip controls + VU; no cue mix / master cue |
| Sync | Expand flat `EngineEvt`; knobs follow `Updated`/`Status` |
| Cmds | Typed FRB methods (same pattern as `play`/`pause`) |
| EQ | `SetEqBand` per knob (`EqBand` FRB enum) |
| Soft takeover | GUI sends `false` |
| Fader UI | Widget stays 0–100; wire is 0–1 |
| Levels isolation | Snapshot stores levels separately so VU ticks do not rebuild knobs |
| Hydrate | `subscribe_events` publishes current `EngineStatus` once |
| Status fan-out | One `EngineStatus` → Status evt (running + crossfader) + Updated-shaped evt per deck |
| Disabled | Strip disabled while engine is not running |
| M/S toggle | Local widget state (not persisted) |

## Architecture

```text
MixerStrip (ConsumerWidget)
  deckMixerChannelProvider(deckId)  → knobs/fader/cue
  deckLevelsProvider(deckId)        → VU
  crossfaderProvider                → xfader
  engineRunningProvider             → enabled
  typed FRB cmds → EngineTransport.publish_cmd

EngineTransport
  set_volume / set_eq_band / set_filter / set_gain_trim / set_headphone_cue
    Origin::Deck(id)
  set_crossfader
    Origin::Mixer
  map_engine_evts(Evt) -> Vec<EngineEvt>
  subscribe_events: start forwarder, then publish engine_status_snapshot
```

`EngineEvt` gains optional: `volume`, `eq_low`, `eq_mid`, `eq_high`, `filter`, `gain_trim`, `headphone_cue`, `crossfader`, `peak_hold_l`, `peak_hold_r`.

## Behavior

- Defaults match the engine: volume `1.0`, knobs `0.5`, crossfader `0.5`, cue off, levels zero.
- `applyEngineEvt` patches only fields present on that evt (same “don’t clobber title” rule). `Levels` updates levels only.
- Cmd / runtime errors: destructive toast; do not crash.
- Widget tests keep `debugOverrideDesktopWindow = false` so the strip stays at defaults with no native audio.

## Non-goals

- Cue mix / master cue widgets or cmds
- Full `EngineStatus` / `DeckSnapshot` dump to Dart
- Generic `publish(origin, kind, body)` on FRB
- Tempo / pads / jog
- Persisting M/S meter mode

## Testing

- Host: `set_volume` → `Updated`; Status fan-out; Levels include peak-hold.
- Dart: reducer patches mixer fields; Levels do not clobber volume; Status sets crossfader.
- Widget: existing mixer shell test still pumps; optional seeded volume if cheap.
- `cargo test -p host_flutter`; `flutter test` for changed Dart tests; FRB regenerated.

## Success criteria

- [ ] Mixer knobs/faders/cue/crossfader publish cmds and follow engine events
- [ ] VU meters show engine peaks + hold
- [ ] High-rate Levels do not rebuild EQ/gain knobs
- [ ] Strip disabled until the engine is running
