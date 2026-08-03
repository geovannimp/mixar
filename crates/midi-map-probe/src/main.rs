//! Live-probe a mapping bundle against a real MIDI port (midir).
//!
//! Built outside the Cargo workspace so midir's alsa-sys does not clash with cpal.
//!
//! Usage (from repo root / worktree):
//!   cargo run --manifest-path tools/midi-map-probe/Cargo.toml -- mappings/ddj-400
//!   cargo run --manifest-path tools/midi-map-probe/Cargo.toml -- mappings/ddj-400 "DDJ-400"

use std::env;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use controller::{load_bundle, BusPublish, MappingSession, MidiOut};
use engine_api::{CmdBody, Kind, Origin};
use midir::{Ignore, MidiInput};

struct Printer;

impl BusPublish for Printer {
    fn publish(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
        println!("  → cmd  {origin:?}  {kind:?}  {body:?}");
    }
}

struct MidiSink;

impl MidiOut for MidiSink {
    fn send(&mut self, bytes: &[u8]) {
        println!("  ← midi_out  {:02X?}", bytes);
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let bundle_path = PathBuf::from(args.next().unwrap_or_else(|| "mappings/ddj-400".into()));
    let name_filter = args.next();

    let bundle = load_bundle(&bundle_path).unwrap_or_else(|e| {
        eprintln!("failed to load {}: {e}", bundle_path.display());
        std::process::exit(1);
    });
    println!("loaded {} ({})", bundle.device.name, bundle.device.id);
    if let (Some(v), Some(p)) = (bundle.device.usb_vid, bundle.device.usb_pid) {
        println!("device usb  {v:04x}:{p:04x}");
    }

    let mut session = MappingSession::from_bundle(bundle).unwrap_or_else(|e| {
        eprintln!("session: {e}");
        std::process::exit(1);
    });

    let mut midi_in = MidiInput::new("midi-map-probe").expect("MidiInput");
    midi_in.ignore(Ignore::None);
    let ports = midi_in.ports();
    if ports.is_empty() {
        eprintln!("no MIDI input ports found");
        std::process::exit(1);
    }

    println!("MIDI inputs:");
    for (i, p) in ports.iter().enumerate() {
        let name = midi_in.port_name(p).unwrap_or_default();
        println!("  [{i}] {name}");
    }

    let idx = if let Some(filter) = &name_filter {
        ports.iter().position(|p| {
            midi_in
                .port_name(p)
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains(&filter.to_ascii_lowercase())
        })
    } else {
        ports.iter().position(|p| {
            midi_in
                .port_name(p)
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains("ddj")
        })
    };

    let idx = match idx {
        Some(i) => i,
        None if name_filter.is_some() => {
            eprintln!("no port matched filter {:?}", name_filter);
            std::process::exit(1);
        }
        None => {
            eprint!("select port index: ");
            let _ = stdout().flush();
            let mut line = String::new();
            stdin().read_line(&mut line).ok();
            line.trim().parse().unwrap_or(0)
        }
    };

    let port = ports.get(idx).expect("port index");
    let port_name = midi_in.port_name(port).unwrap_or_default();
    println!("opening: {port_name}");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _conn = midi_in
        .connect(
            port,
            "midi-map-probe",
            move |_stamp, message, _| {
                let _ = tx.send(message.to_vec());
            },
            (),
        )
        .expect("connect MIDI in");

    let mut bus = Printer;
    let mut midi_out = MidiSink;
    if let Err(e) = session.on_init(&mut bus, &mut midi_out) {
        eprintln!("on_init: {e}");
    }

    println!("listening — press controls on the DDJ-400 (Ctrl-C to quit)");
    loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(bytes) => {
                println!("midi  {:02X?}", bytes);
                session.handle_midi(&bytes, &mut bus);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // keep alive for Ctrl-C
                thread::yield_now();
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}
