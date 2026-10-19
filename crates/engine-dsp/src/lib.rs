//! Pure DSP components for Mixar
//!
//! This crate contains the core DSP functionality including decks,
//! mixer, and audio processing components. It has zero I/O dependencies
//! and is designed to be pure Rust with no external system calls.
//!
//! BPM, key, and beat grid come from library track metadata (offline analysis),
//! not from live buffer analysis in this crate.

use anyhow::Result;
use audio_core::{BusId, Sample};
use std::collections::HashMap;

pub mod deck;
pub mod eq;
pub mod filter;
pub mod fx;
pub mod headphone_monitor;
pub mod level_meter;
pub mod mixer;
pub mod mixer_channel;
pub mod mixer_lane;
pub mod platter_motion;
pub mod sampler;
pub mod tempo;
pub mod transport;

pub use deck::{
    Deck, DeckLoadSpec, DeckState, JogMode, PreparedDeckLoad, PreparedStretcher, StretcherSpec,
};
pub use eq::{clamp_gain_db, DeckEqGains, ThreeBandEq, EQ_MAX_DB, EQ_MIN_DB};
pub use filter::{db_to_linear, DjFilter};
pub use fx::{FxChain, FxKind, FxSlot, FX_PARAM_COUNT, FX_SLOT_COUNT};
pub use headphone_monitor::HeadphoneMonitor;
pub use level_meter::{measure_stereo_peaks, LevelPeaks};
pub use mixer::Mixer;
pub use mixer_channel::{MixerChannel, AUTO_GAIN_CLAMP_DB};
pub use mixer_lane::MixerLane;
pub use platter_motion::{MotionMode, PlatterMotion};
pub use sampler::{
    Sampler, SamplerPlayMode, SamplerSlotMeta, SamplerStripRoute, SAMPLER_SLOT_COUNT,
};
/// Shared session key-shift bound (also used by the engine control layer).
pub use stretch::KEY_SHIFT_SEMITONE_LIMIT;
pub use transport::DeckTransportEvent;

/// DSP engine that manages all audio processing components
#[derive(Debug)]
pub struct DspEngine {
    /// Mixer owns lanes (decks) and channel graph nodes.
    mixer: Mixer,
    /// Immutable engine output sample rate (from config)
    sample_rate: u32,
    /// Immutable engine callback size in frames (from config)
    buffer_size: u32,
}

impl DspEngine {
    /// Create a new DSP engine with an immutable output clock from config.
    pub fn new(
        sample_rate: u32,
        buffer_size: u32,
        num_decks: usize,
        resampler_quality: &str,
        sampler_strip_route: SamplerStripRoute,
    ) -> Self {
        let buffer_size = buffer_size.max(1);
        Self {
            mixer: Mixer::new(
                sample_rate,
                buffer_size,
                num_decks,
                resampler_quality,
                sampler_strip_route,
            ),
            sample_rate,
            buffer_size,
        }
    }

    /// Get the number of decks / lanes
    pub fn num_decks(&self) -> usize {
        self.mixer.num_lanes()
    }

    /// Get a reference to a deck by index
    pub fn deck(&self, index: usize) -> Option<&Deck> {
        self.mixer.lane(index).map(|lane| lane.deck())
    }

    /// Get a mutable reference to a deck by index
    pub fn deck_mut(&mut self, index: usize) -> Option<&mut Deck> {
        self.mixer.lane_mut(index).map(|lane| lane.deck_mut())
    }

    /// Get a reference to the mixer
    pub fn mixer(&self) -> &Mixer {
        &self.mixer
    }

    /// Get a mutable reference to the mixer
    pub fn mixer_mut(&mut self) -> &mut Mixer {
        &mut self.mixer
    }

    /// Process audio for all lanes and mix to output buses.
    pub fn process(
        &mut self,
        frames: u32,
        output_buses: &mut HashMap<BusId, Vec<Sample>>,
    ) -> Result<()> {
        self.mixer.process(frames, output_buses)
    }

    /// Drain transport events from all decks after a process cycle.
    pub fn drain_transport_events(&mut self) -> Vec<(usize, DeckTransportEvent)> {
        let mut events = Vec::new();
        for deck_id in 0..self.mixer.num_lanes() {
            let Some(lane) = self.mixer.lane_mut(deck_id) else {
                continue;
            };
            for event in lane.deck_mut().drain_transport_events() {
                events.push((deck_id, event));
            }
        }
        events
    }

    /// Get the immutable output sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the immutable callback buffer size in frames
    pub fn buffer_size(&self) -> u32 {
        self.buffer_size
    }

    /// Sampler for a deck/lane.
    pub fn sampler(&self, deck_id: usize) -> Option<&Sampler> {
        self.mixer.lane(deck_id).map(|lane| lane.sampler())
    }

    /// Mutable sampler for a deck/lane.
    pub fn sampler_mut(&mut self, deck_id: usize) -> Option<&mut Sampler> {
        self.mixer.lane_mut(deck_id).map(|lane| lane.sampler_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_core::LoadedAudio;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Engine rate for FX tests (a multiple of the mixer's 64-frame chunk).
    const FX_SAMPLE_RATE: u32 = 48_000;
    /// Frames rendered per `process` call in FX tests.
    const FX_CHUNK: u32 = 512;

    #[test]
    fn test_dsp_engine_creation() {
        let engine = DspEngine::new(48000, 512, 2, "medium", SamplerStripRoute::BeforeStrip);
        assert_eq!(engine.num_decks(), 2);
        assert_eq!(engine.sample_rate(), 48000);
        assert_eq!(engine.buffer_size(), 512);
    }

    #[test]
    fn test_dsp_engine_deck_access() {
        let mut engine = DspEngine::new(48000, 512, 2, "medium", SamplerStripRoute::BeforeStrip);

        // Test deck access
        assert!(engine.deck(0).is_some());
        assert!(engine.deck(1).is_some());
        assert!(engine.deck(2).is_none());

        // Test mutable deck access
        assert!(engine.deck_mut(0).is_some());
        assert!(engine.deck_mut(1).is_some());
        assert!(engine.deck_mut(2).is_none());
    }

    #[test]
    fn test_dsp_engine_processing() {
        let mut engine = DspEngine::new(48000, 512, 2, "medium", SamplerStripRoute::BeforeStrip);

        // An empty engine only proves silence, so load a tone and play it.
        let audio = LoadedAudio {
            samples: vec![0.8f32; 4096 * 2],
            sample_rate: 48000,
            channels: 2,
            source_id: "test.wav".to_string(),
        };
        let deck = engine.deck_mut(0).expect("deck 0");
        deck.load(Arc::new(audio)).expect("load test tone");
        deck.play().expect("play");

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);

        engine.process(512, &mut output_buses).expect("process");

        let master = &output_buses[&BusId::new("master")];
        assert!(
            master.iter().any(|&s| s != 0.0),
            "master bus should carry the loaded tone"
        );
        // This assertion only holds while deck 0 is not PFL'd to the cue bus, so
        // state that precondition explicitly instead of assuming the default.
        assert!(
            !engine
                .mixer()
                .channel(0)
                .expect("deck 0 channel")
                .headphone_cue(),
            "test precondition: deck 0 must not be cue-enabled"
        );
        let cue_max = output_buses[&BusId::new("cue")]
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max);
        assert!(cue_max < 1e-6, "cue should be silent without headphone cue");
    }

    /// Unleashed full-scale frame at the start of the track, then silence.
    fn impulse_audio(frames: usize) -> Arc<LoadedAudio> {
        let mut samples = vec![0.0f32; frames * 2];
        samples[0] = 1.0;
        samples[1] = 1.0;
        Arc::new(LoadedAudio {
            samples,
            sample_rate: FX_SAMPLE_RATE,
            channels: 2,
            source_id: "impulse.wav".to_string(),
        })
    }

    /// Echo delay of a half beat at `bpm`, in output frames.
    fn half_beat_frames(bpm: f64) -> usize {
        (60_000.0 / bpm * 0.5 * f64::from(FX_SAMPLE_RATE) / 1000.0).round() as usize
    }

    /// Render `frames` of the master bus with an impulse track on deck 0 and the
    /// shared echo in slot 0 (wet-only, so the direct impulse is removed).
    fn render_master_echo(
        bpm: f64,
        ratio: f32,
        master_deck: usize,
        echo_enabled: bool,
        frames: usize,
    ) -> Vec<f32> {
        let mut engine = DspEngine::new(
            FX_SAMPLE_RATE,
            FX_CHUNK,
            2,
            "medium",
            SamplerStripRoute::BeforeStrip,
        );
        {
            let deck = engine.deck_mut(0).expect("deck 0");
            deck.load(impulse_audio(FX_SAMPLE_RATE as usize))
                .expect("load impulse");
            deck.set_track_bpm(Some(bpm));
            deck.set_playback_ratio(ratio).expect("ratio");
            deck.play().expect("play");
        }
        engine.mixer_mut().set_master_deck(master_deck);
        {
            let channel = engine.mixer_mut().channel_mut(0).expect("deck 0 channel");
            channel.set_fx_routed(true);
            let slot = channel.fx_mut().slot_mut(0).expect("slot 0");
            slot.set_kind(FxKind::Echo);
            slot.set_wet(1.0);
            slot.set_enabled(echo_enabled);
        }

        let mut out = Vec::with_capacity(frames);
        let mut buses = HashMap::new();
        buses.insert(BusId::new("master"), vec![0.0; FX_CHUNK as usize * 2]);
        for _ in 0..(frames / FX_CHUNK as usize) {
            engine.process(FX_CHUNK, &mut buses).expect("process");
            let master = &buses[&BusId::new("master")];
            for frame in 0..FX_CHUNK as usize {
                out.push(master[frame * 2]);
            }
        }
        out
    }

    fn onset(samples: &[f32]) -> Option<usize> {
        samples.iter().position(|sample| sample.abs() > 0.05)
    }

    /// Render `bpm`/`ratio` with `master_deck` as the tempo source and assert the
    /// echo repeat lands half a beat of `sync_bpm` after the dry impulse.
    fn assert_echo_repeat(bpm: f64, ratio: f32, master_deck: usize, sync_bpm: f64) {
        let frames = FX_SAMPLE_RATE as usize * 3;
        // Reference: same deck with the slot bypassed, so the direct impulse
        // measures the deck/lane latency both renders share.
        let direct = onset(&render_master_echo(bpm, ratio, master_deck, false, frames))
            .expect("dry impulse must reach the master bus");
        let expected = half_beat_frames(sync_bpm);

        let wet = render_master_echo(bpm, ratio, master_deck, true, frames);
        let repeat = onset(&wet).expect("echo repeat");
        assert!(
            repeat.abs_diff(direct + expected) <= 2,
            "repeat {repeat}, direct {direct}, expected offset {expected} \
             (sync {sync_bpm} BPM, master deck {master_deck})"
        );
        // Wet 1.0 replaces the dry signal with the delayed one.
        assert!(wet[direct].abs() < 1e-6, "dry impulse must be replaced");
    }

    #[test]
    fn fx_echo_repeat_follows_master_deck_tempo() {
        // Half a beat at 140 BPM = 214.29 ms = 10286 frames.
        assert_echo_repeat(140.0, 1.0, 0, 140.0);

        // The tempo fader shortens the repeat: 140 BPM x 2.0 = 280 BPM.
        assert_echo_repeat(140.0, 2.0, 0, 280.0);

        // The sync tempo comes from the *master* deck: deck 1 has no BPM, so the
        // unit falls back to 120 BPM even though deck 0 is 140 BPM.
        let fallback = f64::from(crate::fx::DEFAULT_FX_BPM);
        assert_ne!(
            half_beat_frames(fallback),
            half_beat_frames(140.0),
            "fallback must differ from deck 0"
        );
        assert_echo_repeat(140.0, 1.0, 1, fallback);
    }
}
