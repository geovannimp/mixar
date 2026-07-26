use engine_api::{decode_wire, encode_cmd_body, encode_wire, CmdBody, Kind, Origin, WireMessage};

#[test]
fn play_cmd_round_trips() {
    let body = encode_cmd_body(&CmdBody::Empty).unwrap();
    let msg = WireMessage {
        origin: Origin::Deck(1),
        kind: Kind::Play,
        revision: 0,
        body,
    };
    let bytes = encode_wire(&msg).unwrap();
    let decoded = decode_wire(&bytes).unwrap();
    assert_eq!(decoded.origin, Origin::Deck(1));
    assert_eq!(decoded.kind, Kind::Play);
}
