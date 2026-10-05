# DDJ-400 hardware checklist

Manual pass for the shipped `mappings/ddj-400` bundle. Everything below is
driven declaratively from `map.toml` `[outputs.*]` (see the header comment there);
`crates/controller/tests/ddj400_feedback.rs` pins the exact MIDI bytes, so this
checklist is about what the unit *does* with them.

## Setup

1. Power the DDJ-400, plug USB in, start Mixar with the engine running.
2. Confirm the unit is in PC MIDI mode (the `on_init` SysEx). Jog ring and button
   lamps should respond to the app rather than to the onboard DJ mode.
3. Leave at least one deck stopped for the first pass so every lamp transition is
   visible.

## Attach / detach / reconnect

- [ ] On attach, every lamp goes through a full assert sweep: all buttons and pads
      dark, then the HOT CUE page lamp (deck 1 **and** deck 2) lights.
- [ ] On detach (Settings → Detach, or unplug), **all** lamps go dark within a
      moment — nothing stays lit.
- [ ] Unplug and replug while a track is playing on deck 1: after re-attach the
      PLAY lamp, the CUE lamp, pad lamps and page lamp all match the engine state
      again, without touching the engine.

## Deck transport (repeat per deck)

- [ ] PLAY/PAUSE: lamp lights on play, goes dark on pause.
- [ ] Starting a track from the GUI lights the lamp too (engine → controller path).
- [ ] Toggling play from the controller lights the lamp before the engine echo.
- [ ] CUE: lamp lights while the cue is held, dark on release.
- [ ] SYNC: lamp follows the sync toggle.
- [ ] QUANTIZE: lamp follows the quantize toggle.
- [ ] HEADPHONE CUE: lamp follows the PFL toggle.
- [ ] LOOP IN alone leaves the loop lamps dark; **LOOP OUT** lights the loop group
      (LOOP IN / LOOP OUT / RELOOP·EXIT and both +SHIFT variants).
- [ ] RELOOP/EXIT (with an active loop) clears the loop group.

## Pad pages

- [ ] HOT CUE page: pressing an empty pad saves a cue → that pad lights; pressing a
      lit pad with SHIFT deletes the cue → the pad goes dark.
- [ ] Switching to BEAT LOOP / BEAT JUMP / SAMPLER **immediately** darkens the
      hot-cue pad bank (stale LEDs are cleared, not left behind).
- [ ] Switching back to HOT CUE re-lights exactly the pads that still have cues.
- [ ] BEAT LOOP page: a saved loop lights its pad; clearing the loop darkens it.
- [ ] BEAT JUMP and SAMPLER pages stay dark (no implemented state lights them yet).

## Shift indicators

- [ ] Hold SHIFT on a deck: all four +SHIFT page indicators **flash** (≈2.5 Hz)
      and the lit page's secondary function is available.
- [ ] Release SHIFT: indicators go dark and stop flashing.

## Master

- [ ] MASTER CUE: lamp follows the master-cue toggle from either the GUI or the
      controller.

## Robustness

- [ ] With the unit attached but the app's MIDI output unavailable, input still
      works and audio is unaffected (check the log for rate-limited
      `midi send failed` warnings).
- [ ] Holding several knobs/pads at once produces no stuck lamps.

## Not yet driven

These addresses are declared in `device.toml` but have no implemented engine
state, so their lamps stay dark. Bind them when the feature lands:

| Address | Control | Waiting on |
| --- | --- | --- |
| `0x17` (deck channel) | VINYL mode ring | engine vinyl-mode state |
| `0x47` / `0x48` (deck channel) | +SHIFT PLAY·PAUSE / CUE | — |
| `0x60` / `0x4C` (deck channel) | 4-BEAT LOOP and its +SHIFT-long press | bound to tempo range in this mapping |
| `0x78` (master channel) | +SHIFT MASTER CUE | — |
| `0x94 0x43` / `0x94 0x47` | BEAT FX ON/OFF | engine beat FX |
| pad pages `0x10`, `0x40`, `0x50`, `0x70` | PAD FX 1, KEYBOARD, PAD FX 2, KEY SHIFT | pad modes this mapping cannot enter |
