//! Optional end-to-end check for the StemgenRT streaming producer.
//!
//! ```text
//! MIXAR_STEMGEN_MODEL=/path/to/stemgenrt.onnx \
//! cargo nextest run -p analyzer-stems --manifest-path crates/Cargo.toml \
//!   --test stemgen_live_smoke --run-ignored=only --no-capture
//! ```

use std::f32::consts::TAU;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use analyzer_stems::start_stemgen_live_separator;
use audio_core::{LiveStemRing, LoadedAudio, DEFAULT_LIVE_STEM_CAPACITY};

#[test]
#[ignore = "requires MIXAR_STEMGEN_MODEL pointing at stemgenrt ONNX"]
fn stemgen_live_publishes_finite_stems() {
    let Some(model) = std::env::var_os("MIXAR_STEMGEN_MODEL").map(PathBuf::from) else {
        return;
    };
    let sample_rate = 44_100u32;
    let frames = sample_rate as usize; // 1 s
    let mut pcm = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let kick = ((t * 2.0) % 1.0 < 0.05) as u8 as f32;
        let s = (t * 110.0 * TAU).sin() * 0.2 + (t * 440.0 * TAU).sin() * 0.05 + kick * 0.15;
        pcm.push(s);
        pcm.push(s * 0.8);
    }

    let ring = Arc::new(LiveStemRing::new(sample_rate, DEFAULT_LIVE_STEM_CAPACITY));
    ring.set_playhead(0);
    let audio = Arc::new(LoadedAudio {
        samples: pcm,
        sample_rate,
        channels: 2,
        source_id: "stemgen-smoke".into(),
    });
    let separator =
        start_stemgen_live_separator(model, audio, Arc::clone(&ring)).expect("start producer");

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some((start, end)) = ring.coverage() {
            if start == 0 && end as usize >= frames {
                break;
            }
        }
        assert!(Instant::now() < deadline, "StemgenRT producer timed out");
        assert!(!ring.is_closed(), "StemgenRT producer closed early");
        std::thread::sleep(Duration::from_millis(20));
    }

    for stem in 0..4 {
        for frame in (0..frames).step_by(97) {
            let (left, right) = ring.frame(stem, frame as u64);
            assert!(
                left.is_finite() && right.is_finite(),
                "stem {stem} frame {frame} is not finite"
            );
        }
    }
    separator.join();
}
