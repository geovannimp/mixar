//! Golden hex fixtures for TS wire codec parity.

use engine_api::{encode_cmd_body, encode_wire, CmdBody, Kind, Origin, WireMessage};
use std::fs;
use std::path::PathBuf;

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

#[test]
fn play_deck1_golden_bytes_stable() {
    let body = encode_cmd_body(&CmdBody::Empty).unwrap();
    let bytes = encode_wire(&WireMessage {
        origin: Origin::Deck(1),
        kind: Kind::Play,
        revision: 0,
        body,
    })
    .unwrap();

    let expected = fs::read_to_string(golden_path("play_deck1.hex"))
        .expect("golden fixture play_deck1.hex")
        .trim()
        .to_string();

    assert_eq!(hex::encode(&bytes), expected);
}
