//! Vinyl jog while paused must publish Position so UI playhead/time track live.

mod common;

use common::{recv_evt_kind, short_tone_fixture};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource};
use omnibus::Filter;

#[test]
fn paused_vinyl_jog_touch_publishes_position_before_release() {
    let session = EngineSession::new(EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    })
    .expect("session");
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    session
        .with_engine(|engine| {
            engine.start()?;
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::from_path(short_tone_fixture())),
            )?;
            engine.seek_deck(0, 80)?;
            Ok(())
        })
        .expect("load");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::JogTouch,
            encode_cmd_body(&CmdBody::JogTouch { touching: true }).unwrap(),
        )
        .expect("touch");
    let touch = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        playing,
        jog_touching,
        ..
    } = decode_evt_body(touch.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(!playing);
    assert!(jog_touching);

    // Scratch ticks are Silent (no DeckUpdated). UI must still get Position while
    // touched — otherwise waveform/time only jump on jog-touch release.
    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::JogTurn,
            encode_cmd_body(&CmdBody::JogTurn { delta: 90 }).unwrap(),
        )
        .expect("turn");

    let pos = recv_evt_kind(&evt, Kind::Position);
    let EvtBody::Position { position_ms, .. } = decode_evt_body(pos.payload()).expect("decode")
    else {
        panic!("expected Position");
    };
    // Contract under test: a Position evt is emitted at all while the deck is
    // jog-touched (`recv_evt_kind` panics on timeout) and carries a playhead
    // inside the 0.25 s fixture. The wheel's rate math is asserted separately by
    // engine-dsp's `paused_vinyl_jog_advances_position_ms`, which owns it.
    assert!(
        (0..=250).contains(&position_ms),
        "Position must be within the 250 ms fixture, got {position_ms}"
    );
}
