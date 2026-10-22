//! Optional end-to-end check: live ring windows are bit-identical to the
//! offline separation for the same PCM.
//!
//! ```text
//! cargo nextest run -p analyzer-stems --manifest-path crates/Cargo.toml \
//!   --test live_split_matches_offline --run-ignored=only --no-capture
//! ```

use std::f32::consts::TAU;
use std::sync::Arc;
use std::time::{Duration, Instant};

use analyzer_stems::{resolve_model, separate_interleaved, start_live_separator, DEFAULT_MODEL};
use audio_core::{LiveStemRing, LoadedAudio, DEFAULT_LIVE_STEM_CAPACITY};

fn test_model() -> (tempfile::TempDir, analyzer_stems::ModelHandle) {
    let dir = tempfile::tempdir().expect("tempdir");
    let models = std::env::var_os("MIXAR_STEMS_MODEL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| dir.path().join("models"));
    std::fs::create_dir_all(&models).expect("models dir");
    let handle = resolve_model(&models, DEFAULT_MODEL, None).expect("model");
    (dir, handle)
}

#[test]
#[ignore = "downloads HTDemucs ONNX (~304MB) and runs ort inference"]
fn live_windows_match_offline_split() {
    let sample_rate = 44_100u32;
    // Two chunks: long enough for one overlap seam, short enough to fit the
    // producer's lookahead without the deck advancing the playhead.
    let frames = (sample_rate as usize) * 23 / 2;
    let mut pcm = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let beat = ((t * 2.0) % 1.0 < 0.08) as u8 as f32;
        let s = (t * 110.0 * TAU).sin() * 0.2 + (t * 330.0 * TAU).sin() * 0.05 + beat * 0.1;
        pcm.push(s);
        pcm.push(s * 0.8);
    }

    // Keep the temp dir alive for the model handle.
    let (_dir, handle) = test_model();

    let (offline, _) =
        separate_interleaved(&handle.local_path, &pcm, sample_rate, None).expect("offline");

    let ring = Arc::new(LiveStemRing::new(sample_rate, DEFAULT_LIVE_STEM_CAPACITY));
    ring.set_playhead(0);
    let audio = Arc::new(LoadedAudio {
        samples: pcm,
        sample_rate,
        channels: 2,
        source_id: "live-test".into(),
    });
    let separator = start_live_separator(handle.local_path.clone(), audio, Arc::clone(&ring))
        .expect("start live");

    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        if let Some((start, end)) = ring.coverage() {
            if start == 0 && end as usize >= frames {
                break;
            }
        }
        assert!(Instant::now() < deadline, "live producer timed out");
        assert!(!ring.is_closed(), "live producer closed early");
        std::thread::sleep(Duration::from_millis(50));
    }

    for (stem, expected_stem) in offline.iter().enumerate() {
        for frame in 0..frames {
            let live = ring.frame(stem, frame as u64);
            let expected = (expected_stem[frame * 2], expected_stem[frame * 2 + 1]);
            assert_eq!(
                live, expected,
                "stem {stem} frame {frame} differs from offline"
            );
        }
    }

    separator.join();
}

#[test]
#[ignore = "downloads HTDemucs ONNX (~304MB) and runs ort inference"]
fn live_seek_uses_offline_chunk_grid() {
    let sample_rate = 44_100u32;
    let frames = 26 * sample_rate as usize;
    let mut pcm = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let beat = ((t * 2.0) % 1.0 < 0.08) as u8 as f32;
        let s = (t * 110.0 * TAU).sin() * 0.2 + (t * 330.0 * TAU).sin() * 0.05 + beat * 0.1;
        pcm.push(s);
        pcm.push(s * 0.8);
    }

    let (_dir, handle) = test_model();

    let (offline, _) =
        separate_interleaved(&handle.local_path, &pcm, sample_rate, None).expect("offline");

    let ring = Arc::new(LiveStemRing::new(sample_rate, DEFAULT_LIVE_STEM_CAPACITY));
    ring.set_playhead(0);
    let audio = Arc::new(LoadedAudio {
        samples: pcm,
        sample_rate,
        channels: 2,
        source_id: "live-seek-test".into(),
    });
    let separator =
        start_live_separator(handle.local_path.clone(), audio, Arc::clone(&ring)).expect("start");

    let wait = |predicate: &dyn Fn() -> bool, what: &str| {
        let deadline = Instant::now() + Duration::from_secs(300);
        while !predicate() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            assert!(
                !ring.is_closed(),
                "producer closed while waiting for {what}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    };

    wait(&|| ring.coverage().is_some(), "the initial window");

    // Far forward seek: the producer must re-epoch on the offline chunk grid
    // and compute the previous chunk so the overlap fade matches.
    let seek = 20 * sample_rate as u64;
    ring.set_playhead(seek);
    wait(
        &|| {
            ring.coverage()
                .is_some_and(|(start, end)| start <= seek && end >= seek + 2_000)
        },
        "the post-seek window",
    );

    let (start, _) = ring.coverage().expect("coverage");
    for (stem, expected) in offline.iter().enumerate() {
        for offset in 0..2_000 {
            let frame = start as usize + offset;
            assert_eq!(
                ring.frame(stem, frame as u64),
                (expected[frame * 2], expected[frame * 2 + 1]),
                "post-seek epoch stem {stem} frame {frame}"
            );
        }
        for offset in 0..2_000 {
            let frame = seek as usize + offset;
            assert_eq!(
                ring.frame(stem, frame as u64),
                (expected[frame * 2], expected[frame * 2 + 1]),
                "seek position stem {stem} frame {frame}"
            );
        }
    }

    separator.join();
}
