//! MessagePack roundtrip for library wire messages and nested bodies.

use library_api::{
    decode_cmd_body, decode_evt_body, decode_wire, encode_cmd_body, encode_evt_body, encode_wire,
    CmdBody, EvtBody, Kind, Origin, TrackSummary, WireMessage,
};

#[test]
fn wire_analyze_track_roundtrips() {
    let body = encode_cmd_body(&CmdBody::AnalyzeTrack {
        track_id: "t1".into(),
        force: false,
    })
    .unwrap();
    let msg = WireMessage {
        origin: Origin::Library,
        kind: Kind::AnalyzeTrack,
        revision: 0,
        action_timestamp_ms: 1_700_000_000_000,
        body,
    };
    let bytes = encode_wire(&msg).unwrap();
    let decoded = decode_wire(&bytes).unwrap();
    assert_eq!(decoded, msg);
    assert_eq!(
        decode_cmd_body(&decoded.body).unwrap(),
        CmdBody::AnalyzeTrack {
            track_id: "t1".into(),
            force: false,
        }
    );
}

#[test]
fn cmd_and_evt_bodies_roundtrip() {
    let cmd = CmdBody::Empty;
    assert_eq!(
        decode_cmd_body(&encode_cmd_body(&cmd).unwrap()).unwrap(),
        cmd
    );

    let track = TrackSummary {
        id: "t1".into(),
        display_name: "Artist — Title".into(),
        artist: Some("Artist".into()),
        title: Some("Title".into()),
        album: None,
        genre: None,
        bpm: Some(128.0),
        key: Some("8A".into()),
        duration_ms: Some(180_000),
        path: "/music/a.mp3".into(),
    };
    let evt = EvtBody::TrackAnalyzed {
        track: track.clone(),
    };
    assert_eq!(
        decode_evt_body(&encode_evt_body(&evt).unwrap()).unwrap(),
        evt
    );

    let err = EvtBody::Error {
        message: "boom".into(),
        track_id: Some("t1".into()),
    };
    assert_eq!(
        decode_evt_body(&encode_evt_body(&err).unwrap()).unwrap(),
        err
    );
}
