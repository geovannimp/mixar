//! MessagePack roundtrip for wire messages and nested bodies.

use engine_api::{
    decode_cmd_body, decode_evt_body, decode_wire, encode_cmd_body, encode_evt_body, encode_wire,
    CmdBody, EvtBody, Kind, Origin, WireMessage,
};

#[test]
fn wire_play_deck1_roundtrips() {
    let body = encode_cmd_body(&CmdBody::Empty).unwrap();
    let msg = WireMessage {
        origin: Origin::Deck(1),
        kind: Kind::Play,
        revision: 0,
        action_timestamp_ms: 0,
        body,
    };
    let bytes = encode_wire(&msg).unwrap();
    let decoded = decode_wire(&bytes).unwrap();
    assert_eq!(decoded, msg);
    assert_eq!(decode_cmd_body(&decoded.body).unwrap(), CmdBody::Empty);
}

#[test]
fn cmd_and_evt_bodies_roundtrip() {
    let cmd = CmdBody::Empty;
    assert_eq!(
        decode_cmd_body(&encode_cmd_body(&cmd).unwrap()).unwrap(),
        cmd
    );

    let evt = EvtBody::Error {
        message: "boom".into(),
    };
    assert_eq!(
        decode_evt_body(&encode_evt_body(&evt).unwrap()).unwrap(),
        evt
    );

    let seek = CmdBody::Seek {
        position_ms: 12_500,
    };
    assert_eq!(
        decode_cmd_body(&encode_cmd_body(&seek).unwrap()).unwrap(),
        seek
    );
}

#[test]
fn oversize_payload_is_rejected() {
    use engine_api::{DecodeError, MAX_WIRE_PAYLOAD_BYTES};

    let oversized = vec![0u8; MAX_WIRE_PAYLOAD_BYTES + 1];
    let err = decode_cmd_body(&oversized).unwrap_err();
    assert!(matches!(err, DecodeError::PayloadTooLarge { .. }));
}
