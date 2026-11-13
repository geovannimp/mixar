//! Auto Cue: the track cue hydrates a deck on load and moves the playhead.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use common::write_silence_then_tone_wav;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use library::CueSource;
use library::Library;
use library::LibraryConfig;
use library::LibrarySession;
use library::NewCollection;
use library::TrackCue;
use library::TrackId;
use library::WritableLibrary;
use std::time::Duration;
use std::time::Instant;

struct Fixture {
    _dir: tempfile::TempDir,
    /// Owned here: dropping it stops the library worker, and the manual-cue
    /// save below needs that worker draining `SaveTrackCue`.
    library_session: LibrarySession,
    session: EngineSession,
    track_id: TrackId,
}

/// Import a `silence + tone` probe and start an engine on deck 0.
///
/// `auto_cue` is applied before the first load, so it covers both switch states.
fn fixture(silence_ms: u32, auto_cue: bool) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("cue-probe.wav");
    write_silence_then_tone_wav(&wav, silence_ms, 500, 48_000);

    let library_session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = {
        let library = library_session.library();
        let mut lib = library.lock().unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let tracks = lib.list_collection_tracks(&folder.id).unwrap();
        assert_eq!(tracks.len(), 1);
        tracks[0].id().clone()
    };

    let session = EngineSession::new_with_library_bus(
        EngineConfig {
            backend: "null".to_string(),
            ..Default::default()
        },
        library_session.library(),
        library_session.bus(),
    )
    .expect("engine session");
    session
        .with_engine(|engine| {
            engine.start()?;
            engine.set_auto_cue(auto_cue);
            Ok(())
        })
        .expect("start");

    Fixture {
        _dir: dir,
        library_session,
        session,
        track_id,
    }
}

impl Fixture {
    fn load(&self) {
        let track_id = self.track_id.clone();
        self.session
            .with_engine(|engine| {
                engine.load_track_from_library(0, &track_id)?;
                Ok(())
            })
            .expect("load");
    }

    fn cue_and_position(&self) -> (Option<i32>, Option<i32>) {
        self.session
            .with_engine(|engine| {
                let snapshot = engine.deck_snapshot(0).expect("deck snapshot");
                Ok((snapshot.cue_point_ms, snapshot.position_ms))
            })
            .expect("snapshot")
    }

    fn wait_for_manual_cue(&self, expected: i32) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let cue = self
                .library_session
                .library()
                .lock()
                .unwrap()
                .track_cue(&self.track_id)
                .unwrap();
            if cue
                == Some(TrackCue {
                    position_ms: expected,
                    source: CueSource::Manual,
                })
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("timeout waiting for the manual cue at {expected} ms");
    }
}

#[test]
fn auto_cue_moves_the_playhead_to_the_track_cue() {
    let fixture = fixture(1000, true);
    fixture.load();

    // The load-time analysis detected the first sound and the deck followed it.
    assert_eq!(
        fixture
            .library_session
            .library()
            .lock()
            .unwrap()
            .track_cue(&fixture.track_id)
            .unwrap(),
        Some(TrackCue {
            position_ms: 1000,
            source: CueSource::Analysis,
        })
    );
    let (cue_point_ms, position_ms) = fixture.cue_and_position();
    assert_eq!(cue_point_ms, Some(1000));
    assert_eq!(position_ms, Some(1000));
}

#[test]
fn auto_cue_off_keeps_the_cue_but_not_the_playhead() {
    let fixture = fixture(1000, false);
    fixture.load();

    let (cue_point_ms, position_ms) = fixture.cue_and_position();
    assert_eq!(cue_point_ms, Some(1000), "the cue marker still hydrates");
    assert_eq!(position_ms, Some(0), "the playhead stays at the start");
}

#[test]
fn auto_cue_moves_to_a_manual_cue() {
    let fixture = fixture(1000, true);
    fixture.load();

    fixture
        .session
        .with_engine(|engine| {
            engine.set_deck_cue_point(0, 500)?;
            engine.save_deck_track_cue(0)?;
            Ok(())
        })
        .expect("persist manual cue");
    fixture.wait_for_manual_cue(500);

    // A reload hydrates the manual cue, not the analyzed one it replaced.
    fixture.load();
    let (cue_point_ms, position_ms) = fixture.cue_and_position();
    assert_eq!(cue_point_ms, Some(500));
    assert_eq!(position_ms, Some(500));
}

#[test]
fn auto_cue_ignores_a_cue_at_zero() {
    // No silence: the first frame is already sound, so the cue is 0.
    let fixture = fixture(0, true);
    fixture.load();

    let (cue_point_ms, position_ms) = fixture.cue_and_position();
    assert_eq!(cue_point_ms, Some(0));
    assert_eq!(position_ms, Some(0));
}
