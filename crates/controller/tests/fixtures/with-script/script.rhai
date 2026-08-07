fn on_init() {
    midi_out([0x90, 0x01, 0x7F]);
}

fn idle_heartbeat() {
    midi_out([0xB0, 0x02, 0x00]);
}

fn on_shutdown() {
}
