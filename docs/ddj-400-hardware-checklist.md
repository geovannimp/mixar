# DDJ-400 hardware checklist

Manual pass for the shipped `mappings/ddj-400` bundle. Everything below is
driven declaratively from `map.toml` `[outputs.*]` (see the header comment there),
with Keyboard / Key Shift pad banks still host-driven via `apply_output_signal`.
`crates/controller/tests/ddj400_feedback.rs` pins the declarative LED wire bytes;
`crates/controller/tests/ddj400_kb_keyshift.rs` (when present) pins Keyboard / Key
Shift banks. This checklist is about what the unit *does* with them.

## Setup

1. Power the DDJ-400, plug USB in, start Mixar with the engine running.
2. Confirm the unit is in PC MIDI mode (the `on_init` SysEx). Jog ring and button
   lamps should respond to the app rather than to the onboard DJ mode.
3. Leave at least one deck stopped for the first pass so every lamp transition is
   visible.

## Attach / detach / reconnect

- [ ] On attach, every lamp goes through a full assert sweep: all buttons and pads
      dark, then the HOT CUE page lamp (deck 1 **and** deck 2) lights. Engine
      replay also restores Keyboard / Key Shift pad lamps when those modes are
      active.
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
- [ ] LOOP IN alone lights the LOOP IN lamp (pending entrance); **LOOP OUT** lights
      the rest of the loop group (LOOP OUT / RELOOP·EXIT and both +SHIFT variants).
- [ ] RELOOP/EXIT (with an active loop) clears the loop group.
- [ ] SYNC lights on a slave with sync enabled **and** on the master deck.

## Pad pages

- [ ] HOT CUE page: pressing an empty pad saves a cue → that pad lights; pressing a
      lit pad with SHIFT deletes the cue → the pad goes dark.
- [ ] Switching to BEAT LOOP / BEAT JUMP / SAMPLER **immediately** darkens the
      hot-cue pad bank (stale LEDs are cleared, not left behind).
- [ ] Switching back to HOT CUE re-lights exactly the pads that still have cues.
- [ ] BEAT LOOP page: a saved loop lights its pad; clearing the loop darkens it.
- [ ] BEAT JUMP and SAMPLER pages stay dark (no implemented state lights them yet).

## Keyboard page

- [ ] Press **KEYBOARD** (note `0x69` on the deck channel): the KEYBOARD page
      lamp lights and the KEY SHIFT page lamp goes dark.
- [ ] The eight pads (`ch 8/10`, notes `0x40–0x47`) audition the selected root
      hot cue at the current semitone page (default page 2 = `[0..+7]`); the
      pad whose absolute semitone matches the current key shift lights.
- [ ] Release of a Keyboard pad is momentary: it restores the pre-press key
      shift and seeks back to the root, and its lamp clears.
- [ ] SHIFT + pads 1–6 on the shift bank (`ch 9/11`, notes `0x40–0x45`) deletes
      that hot cue.
- [ ] SHIFT + pad 7 / pad 8 (`0x46` / `0x47`) switches to the next / previous
      semitone page, wrapping `5 → 1` and `1 → 5`.

## Key Shift page

- [ ] Press **KEY SHIFT** (note `0x6F` on the deck channel): the KEY SHIFT page
      lamp lights and the KEYBOARD page lamp goes dark.
- [ ] The eight pads (`ch 8/10`, notes `0x70–0x77`) latch the current page's
      action (default page 2 = `[0, +1, +2, +3, +4, +5, +6, +7]`); exactly the
      pad matching the active absolute semitone lights.
- [ ] Page 5 mixes `RESET`, `DOWN`, `−5`, `−12`, `SYNC`, `UP`, `+7`, `+12`;
      RESET zeroes the shift, UP / DOWN nudge by one semitone, SYNC is a no-op.
- [ ] SHIFT + pad 7 / pad 8 (`ch 9/11`, notes `0x76` / `0x77`) switches to the
      next / previous semitone page, wrapping `5 → 1` and `1 → 5`.
- [ ] Pitch changes without changing tempo; key lock stays intact.

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
| pad pages `0x10`, `0x50` | PAD FX 1, PAD FX 2 | pad modes this mapping cannot enter |
| pad ch `0x20`–`0x27` | Beat Jump pad bank | engine beat-jump slot / press state for LEDs |
| pad ch `0x30`–`0x37` | Sampler pad bank | engine sampler slot state for LEDs |
