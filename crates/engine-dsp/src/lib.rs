//! Pure DSP components for rust-dj-engine
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
pub mod headphone_monitor;
pub mod level_meter;
pub mod mixer;
pub mod mixer_channel;
pub mod mixer_lane;
pub mod sampler;
pub mod transport;

pub use deck::{Deck, DeckState, JogMode};
pub use eq::{clamp_gain_db, DeckEqGains, ThreeBandEq, EQ_MAX_DB, EQ_MIN_DB};
pub use filter::{db_to_linear, DjFilter};
pub use headphone_monitor::HeadphoneMonitor;
pub use level_meter::{measure_stereo_peaks, LevelPeaks};
pub use mixer::Mixer;
pub use mixer_channel::{MixerChannel, AUTO_GAIN_CLAMP_DB};
pub use mixer_lane::MixerLane;
pub use sampler::{
    Sampler, SamplerPlayMode, SamplerSlotMeta, SamplerStripRoute, SAMPLER_SLOT_COUNT,
};
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
    use std::collections::HashMap;

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
        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);

        // Process some audio
        let result = engine.process(512, &mut output_buses);
        assert!(result.is_ok());
    }
}
