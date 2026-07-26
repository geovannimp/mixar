use engine_api::{
    decode_cmd_body, decode_evt_body, decode_wire, encode_cmd_body, encode_evt_body, encode_wire,
    CmdBody, EvtBody, Kind, Origin, WireMessage,
};

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
    assert_eq!(decoded, msg);
}

#[test]
fn cmd_body_empty_round_trips() {
    let body = CmdBody::Empty;
    let bytes = encode_cmd_body(&body).unwrap();
    let decoded = decode_cmd_body(&bytes).unwrap();
    assert_eq!(decoded, body);
}

#[test]
fn evt_body_error_round_trips() {
    let body = EvtBody::Error {
        message: "x".into(),
    };
    let bytes = encode_evt_body(&body).unwrap();
    let decoded = decode_evt_body(&bytes).unwrap();
    assert_eq!(decoded, body);
}
