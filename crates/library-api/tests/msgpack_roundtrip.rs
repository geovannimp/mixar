//! MessagePack roundtrip for library wire messages and nested bodies.

use library_api::{
    decode_cmd_body, decode_evt_body, decode_wire, encode_cmd_body, encode_evt_body, encode_wire,
    CmdBody, EvtBody, HotCue, Kind, Origin, SavedLoop, TrackSummary, WireMessage,
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

    let save = CmdBody::SaveHotCue {
        track_id: "t1".into(),
        slot: 2,
        position_ms: 12_500,
        loop_length_beats: None,
        color: None,
        label: Some("drop".into()),
    };
    assert_eq!(
        decode_cmd_body(&encode_cmd_body(&save).unwrap()).unwrap(),
        save
    );

    let cues = EvtBody::HotCuesChanged {
        track_id: "t1".into(),
        hot_cues: vec![HotCue {
            slot: 2,
            position_ms: 12_500,
            loop_length_beats: None,
            color: None,
            label: Some("drop".into()),
        }],
    };
    assert_eq!(
        decode_evt_body(&encode_evt_body(&cues).unwrap()).unwrap(),
        cues
    );

    let loops = EvtBody::LoopsChanged {
        track_id: "t1".into(),
        loops: vec![SavedLoop {
            slot: 0,
            in_ms: 0,
            out_ms: 2000,
            label: None,
            color: None,
        }],
    };
    assert_eq!(
        decode_evt_body(&encode_evt_body(&loops).unwrap()).unwrap(),
        loops
    );
}

#[test]
fn library_navigation_origin_and_navigate_kinds_roundtrip() {
    let msg = WireMessage {
        origin: Origin::LibraryNavigation,
        kind: Kind::NavigateNext,
        revision: 1,
        action_timestamp_ms: 0,
        body: encode_evt_body(&EvtBody::Empty).unwrap(),
    };
    let decoded = decode_wire(&encode_wire(&msg).unwrap()).unwrap();
    assert_eq!(decoded.origin, Origin::LibraryNavigation);
    assert_eq!(decoded.kind, Kind::NavigateNext);
    assert_eq!(decode_evt_body(&decoded.body).unwrap(), EvtBody::Empty);

    let prev = WireMessage {
        origin: Origin::LibraryNavigation,
        kind: Kind::NavigatePrev,
        revision: 2,
        action_timestamp_ms: 0,
        body: encode_evt_body(&EvtBody::Empty).unwrap(),
    };
    assert_eq!(
        decode_wire(&encode_wire(&prev).unwrap()).unwrap().kind,
        Kind::NavigatePrev
    );
}

#[test]
fn load_focused_to_deck_kind_and_body_roundtrip() {
    let body = EvtBody::LoadFocusedToDeck { deck: 1 };
    let msg = WireMessage {
        origin: Origin::LibraryNavigation,
        kind: Kind::LoadFocusedToDeck,
        revision: 3,
        action_timestamp_ms: 0,
        body: encode_evt_body(&body).unwrap(),
    };
    let decoded = decode_wire(&encode_wire(&msg).unwrap()).unwrap();
    assert_eq!(decoded.kind, Kind::LoadFocusedToDeck);
    assert_eq!(decode_evt_body(&decoded.body).unwrap(), body);
}
