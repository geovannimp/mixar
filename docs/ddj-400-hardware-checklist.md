# DDJ-400 hardware checklist

Manual pass for the shipped `mappings/ddj-400` bundle. LEDs are driven from the
mapping session's snapshot via `apply_output_signal`
(`crates/controller/src/session.rs`); `crates/controller/tests/ddj400_kb_keyshift.rs`
pins the exact MIDI bytes for the Keyboard / Key Shift banks, so this checklist
is about what the unit *does* with them.

## Setup

1. Power the DDJ-400, plug USB in, start Mixar with the engine running.
2. Confirm the unit is in PC MIDI mode. Jog ring and button lamps should respond
   to the app rather than to the onboard DJ mode.
3. Leave at least one deck stopped for the first pass so every lamp transition
   is visible.

## Attach / detach / reconnect

- [ ] On attach, the app replays engine state (`EngineStatus`) so the active pad
      page, hot-cue LEDs, and the Keyboard / Key Shift pad lamp match the engine
      without touching the unit. (Transport lamps are not replayed here; see the
      transport section.)
- [ ] On detach (Settings → Detach, or unplug), all lamps go dark within a moment.
- [ ] Unplug and replug while a track is playing: after re-attach the pad / mode
      lamps match engine state again.

## Deck transport (repeat per deck)

- [ ] PLAY/PAUSE: lamp lights on play, goes dark on pause (from GUI or unit).
- [ ] CUE: lamp lights while the cue is held, dark on release.
- [ ] SYNC / QUANTIZE / HEADPHONE CUE: lamps follow their toggles.

## Pad pages

- [ ] HOT CUE page: pressing an empty pad saves a cue → that pad lights; pressing
      a lit pad with SHIFT deletes the cue → the pad goes dark.
- [ ] Switching to another page **immediately** darkens the previous bank (stale
      LEDs are cleared, not left behind).
- [ ] Switching back to HOT CUE re-lights exactly the pads that still have cues.

## Keyboard page

- [ ] Press **KEYBOARD** (note `0x69` on the deck channel): the KEYBOARD page
      lamp lights and the KEY SHIFT page lamp goes dark.
- [ ] The eight pads (`ch 8/10`, notes `0x40–0x47`) play the deck at ascending
      scale degrees; the pad whose degree matches the current key shift lights.
- [ ] Release of a Keyboard pad clears the pitch offset (momentary) and its lamp.
- [ ] SHIFT + pad 1/2/3 on the scale bank (`ch 9/11`, notes `0x40–0x42`) selects
      Major / Minor / Pentatonic; the lit pad follows the new scale's degrees.

## Key Shift page

- [ ] Press **KEY SHIFT** (note `0x6F` on the deck channel): the KEY SHIFT page
      lamp lights and the KEYBOARD page lamp goes dark.
- [ ] The eight pads (`ch 8/10`, notes `0x70–0x77`) latch semitone offsets
      `[0, +1, +2, +3, -4, -3, -2, -1]`; exactly the matching pad lights.
- [ ] Re-pressing the active pad clears the offset; the `0` pad (index 1) lights
      again.
- [ ] Any reset pad on the shift bank (`ch 9/11`, notes `0x70–0x77`) clears the
      offset and lights the `0` pad.
- [ ] Pitch changes without changing tempo; key lock stays intact.

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
| `0x78` (master channel) | +SHIFT MASTER CUE | — |
| `0x94 0x43` / `0x94 0x47` | BEAT FX ON/OFF | engine beat FX |
| pad pages `0x10`, `0x50` | PAD FX 1, PAD FX 2 | pad modes this mapping cannot enter |
