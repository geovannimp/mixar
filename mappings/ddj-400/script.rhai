// Pioneer DDJ-400 init — Mixxx sends this SysEx so the unit stays in PC MIDI mode.
// Source: Pioneer-DDJ-400-script.js PioneerDDJ400.init
// VU meters: host drives device `vu_meter` CC from engine Levels (not this script).
//
// Top-level `const` is not visible in hooks: Mixar compiles the AST then calls
// named fns with a fresh Scope (no top-level eval). Keep the payload function-local.

fn pc_mode_sysex() {
    [0xF0, 0x00, 0x40, 0x05, 0x00, 0x00, 0x02, 0x06, 0x00, 0x03, 0x01, 0xF7]
}

fn on_init() {
    midi_out(pc_mode_sysex());
    // Track-loaded pad lights (Mixxx uses 0x9F note 0/1); harmless if unsupported.
    midi_out([0x9F, 0x00, 0x7F]);
    midi_out([0x9F, 0x01, 0x7F]);
}

// Keepalive while stopped — re-assert PC MIDI mode so the unit doesn't idle.
// Skipped while any deck is playing (host only calls this when idle; VU traffic covers play).
fn idle_heartbeat() {
    midi_out(pc_mode_sysex());
}

fn on_shutdown() {
    midi_out([0x9F, 0x00, 0x00]);
    midi_out([0x9F, 0x01, 0x00]);
}
