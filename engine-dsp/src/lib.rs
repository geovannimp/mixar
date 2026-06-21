//! Pure DSP components for rust-dj-engine
//!
//! This crate contains the core DSP functionality including decks,
//! mixer, and audio processing components. It has zero I/O dependencies
//! and is designed to be pure Rust with no external system calls.

use audio_core::{BusId, Sample};
use anyhow::Result;
use std::collections::HashMap;

pub mod deck;
pub mod mixer;
pub mod analyzer;

pub use deck::Deck;
pub use mixer::Mixer;
pub use analyzer::BpmAnalyzer;

/// DSP engine that manages all audio processing components
#[derive(Debug)]
pub struct DspEngine {
    /// Audio decks for playback
    decks: Vec<Deck>,
    /// Audio mixer for routing and mixing
    mixer: Mixer,
    /// BPM analyzer for tempo detection
    bpm_analyzer: BpmAnalyzer,
    /// Sample rate for all processing
    sample_rate: u32,
}

impl DspEngine {
    /// Create a new DSP engine
    pub fn new(sample_rate: u32, num_decks: usize) -> Self {
        let mut decks = Vec::with_capacity(num_decks);
        for i in 0..num_decks {
            decks.push(Deck::new(i, sample_rate));
        }

        Self {
            decks,
            mixer: Mixer::new(sample_rate),
            bpm_analyzer: BpmAnalyzer::new(sample_rate),
            sample_rate,
        }
    }

    /// Get the number of decks
    pub fn num_decks(&self) -> usize {
        self.decks.len()
    }

    /// Get a reference to a deck by index
    pub fn deck(&self, index: usize) -> Option<&Deck> {
        self.decks.get(index)
    }

    /// Get a mutable reference to a deck by index
    pub fn deck_mut(&mut self, index: usize) -> Option<&mut Deck> {
        self.decks.get_mut(index)
    }

    /// Get a reference to the mixer
    pub fn mixer(&self) -> &Mixer {
        &self.mixer
    }

    /// Get a mutable reference to the mixer
    pub fn mixer_mut(&mut self) -> &mut Mixer {
        &mut self.mixer
    }

    /// Get a reference to the BPM analyzer
    pub fn bpm_analyzer(&self) -> &BpmAnalyzer {
        &self.bpm_analyzer
    }

    /// Get a mutable reference to the BPM analyzer
    pub fn bpm_analyzer_mut(&mut self) -> &mut BpmAnalyzer {
        &mut self.bpm_analyzer
    }

    /// Process audio for all decks and mix to output buses
    ///
    /// # Arguments
    /// * `frames` - Number of frames to process
    /// * `output_buses` - Map of bus ID to output buffer
    pub fn process(&mut self, frames: u32, output_buses: &mut HashMap<BusId, Vec<Sample>>) -> Result<()> {
        // Mix all decks to output buses
        self.mixer.process(&mut self.decks, frames, output_buses)?;

        Ok(())
    }

    /// Get the current sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Set the sample rate (requires reinitializing components)
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;

        // Reinitialize all components with new sample rate
        for deck in &mut self.decks {
            deck.set_sample_rate(sample_rate);
        }
        self.mixer.set_sample_rate(sample_rate);
        self.bpm_analyzer.set_sample_rate(sample_rate);
    }

    /// Set callback buffer size on all decks (must match the audio backend).
    pub fn set_output_chunk_frames(&mut self, frames: u32) {
        for deck in &mut self.decks {
            deck.set_output_chunk_frames(frames);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_dsp_engine_creation() {
        let engine = DspEngine::new(48000, 2);
        assert_eq!(engine.num_decks(), 2);
        assert_eq!(engine.sample_rate(), 48000);
    }

    #[test]
    fn test_dsp_engine_deck_access() {
        let mut engine = DspEngine::new(48000, 2);
        
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
        let mut engine = DspEngine::new(48000, 2);
        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
        
        // Process some audio
        let result = engine.process(512, &mut output_buses);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sample_rate_change() {
        let mut engine = DspEngine::new(48000, 2);
        assert_eq!(engine.sample_rate(), 48000);
        
        engine.set_sample_rate(44100);
        assert_eq!(engine.sample_rate(), 44100);
    }
}
