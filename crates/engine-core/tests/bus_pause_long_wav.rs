use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource};
use omnibus::Filter;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

fn recv_evt_kind(
    sub: &omnibus::BusReceiver<Origin, Kind, std::sync::Arc<[u8]>>,
    kind: Kind,
) -> omnibus::Event<Origin, Kind, std::sync::Arc<[u8]>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = sub
            .recv_timeout(remaining.min(Duration::from_millis(50)))
            .expect("recv")
            .expect("event");
        if *event.kind() == kind {
            return (*event).clone();
        }
    }
    panic!("timeout waiting for evt kind {kind:?}");
}

fn long_wav() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../samples/Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav",
    )
}

#[test]
fn pause_preserves_position_on_long_24bit_wav() {
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
            eprintln!("loading long wav…");
            engine.load_track(0, AudioSource::File(FileAudioSource::from_path(long_wav())))?;
            let (pos, dur) = engine.deck_playback_ms(0).expect("playback ms");
            eprintln!("loaded pos={pos} dur={dur}");
            engine.seek_deck(0, 30_000)?;
            let (pos, _) = engine.deck_playback_ms(0).expect("after seek");
            eprintln!("after seek pos={pos}");
            Ok(())
        })
        .expect("load");

    let empty = encode_cmd_body(&CmdBody::Empty).unwrap();
    session
        .publish_cmd(Origin::Deck(0), Kind::Play, empty.clone())
        .expect("play");
    let _ = recv_evt_kind(&evt, Kind::Updated);
    thread::sleep(Duration::from_millis(100));
    session
        .publish_cmd(Origin::Deck(0), Kind::Pause, empty)
        .expect("pause");
    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        playing,
        position_ms,
        duration_ms,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    eprintln!("pause playing={playing} pos={position_ms:?} dur={duration_ms:?}");
    assert!(!playing);
    let pos = position_ms.expect("position");
    assert!(pos >= 29_000, "expected near 30s after pause, got {pos}");
}
