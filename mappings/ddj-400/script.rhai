// Pioneer DDJ-400 init — Mixxx sends this SysEx so the unit stays in PC MIDI mode.
// Source: Pioneer-DDJ-400-script.js PioneerDDJ400.init
// LEDs are declarative now: map.toml `[outputs.*]` drives every lamp from engine
// state, and the session force-refreshes on attach and clears on detach.
// VU meters: host drives device `vu_meter` CC from engine Levels (not this script).

const PC_MODE_SYSEX = [
    0xF0, 0x00, 0x40, 0x05, 0x00, 0x00, 0x02, 0x06, 0x00, 0x03, 0x01, 0xF7
];

fn on_init() {
    midi_out(PC_MODE_SYSEX);
}

// Keepalive while stopped — re-assert PC MIDI mode so the unit doesn't idle.
// Skipped while any deck is playing (host only calls this when idle; VU traffic covers play).
fn idle_heartbeat() {
    midi_out(PC_MODE_SYSEX);
}
