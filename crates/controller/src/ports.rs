//! MIDI port enumeration without opening a controller session.
//!
//! [`crate::ControllerEngine::list_input_port_names`] answers the same question,
//! but only from an opened engine: opening one seeds the mapping bundles into
//! app data and starts the poll/pump threads. First-run setup needs the answer
//! before any of that, so this module owns a throwaway enumeration client.

use midir::MidiInput;

/// Names of the MIDI input ports ALSA/CoreMIDI reports, in OS order.
///
/// Enumeration-only: no port is opened, so this is safe to call while another
/// process (or a later engine start) owns the device.
pub fn list_midi_input_ports(app_name: &str) -> Result<Vec<String>, String> {
    let input = MidiInput::new(app_name)
        .map_err(|error| format!("MIDI input enumeration for '{app_name}': {error}"))?;
    Ok(input
        .ports()
        .iter()
        .filter_map(|port| input.port_name(port).ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumeration_reports_or_errors_without_panicking() {
        // A host with no ALSA sequencer makes `MidiInput::new` fail; that is a
        // reported error, not a panic, and CI boxes and desktops differ.
        // Names are not asserted unique: two identical interfaces legitimately
        // report the same name, and this machine may own a pair.
        match list_midi_input_ports("Mixar test") {
            Ok(ports) => assert!(ports.iter().all(|name| !name.is_empty())),
            Err(error) => assert!(!error.is_empty()),
        }
    }
}
