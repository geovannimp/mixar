// Pioneer DDJ-400 init — Mixxx sends this SysEx so the unit stays in PC MIDI mode.
// Source: Pioneer-DDJ-400-script.js PioneerDDJ400.init

fn on_init() {
    midi_out([
        0xF0, 0x00, 0x40, 0x05, 0x00, 0x00, 0x02, 0x06, 0x00, 0x03, 0x01, 0xF7
    ]);
    // Track-loaded pad lights (Mixxx uses 0x9F note 0/1); harmless if unsupported.
    midi_out([0x9F, 0x00, 0x7F]);
    midi_out([0x9F, 0x01, 0x7F]);
}

fn on_shutdown() {
    midi_out([0x9F, 0x00, 0x00]);
    midi_out([0x9F, 0x01, 0x00]);
}
