//! Audio mixer for routing and mixing multiple decks
//!
//! The mixer handles routing audio from decks to different output buses
//! and applies mixing, EQ, and other effects.

use anyhow::Result;
use audio_core::{BusId, Sample};
use std::collections::HashMap;

use crate::deck::Deck;

/// Audio mixer for routing and mixing
#[derive(Debug)]
pub struct Mixer {
    /// Sample rate
    sample_rate: u32,
    /// Master volume
    master_volume: f32,
    /// Cue volume
    cue_volume: f32,
    /// Internal mixing buffer
    mix_buffer: Vec<Sample>,
}

impl Mixer {
    /// Create a new mixer
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            master_volume: 1.0,
            cue_volume: 1.0,
            mix_buffer: Vec::new(),
        }
    }

    /// Get the master volume
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Set the master volume
    pub fn set_master_volume(&mut self, volume: f32) -> Result<()> {
        if volume < 0.0 || volume > 1.0 {
            return Err(anyhow::anyhow!("Volume must be between 0.0 and 1.0"));
        }
        self.master_volume = volume;
        Ok(())
    }

    /// Get the cue volume
    pub fn cue_volume(&self) -> f32 {
        self.cue_volume
    }

    /// Set the cue volume
    pub fn set_cue_volume(&mut self, volume: f32) -> Result<()> {
        if volume < 0.0 || volume > 1.0 {
            return Err(anyhow::anyhow!("Volume must be between 0.0 and 1.0"));
        }
        self.cue_volume = volume;
        Ok(())
    }

    /// Set the sample rate
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
    }

    /// Process audio from all decks and mix to output buses
    ///
    /// # Arguments
    /// * `decks` - Vector of decks to mix from
    /// * `frames` - Number of frames to process
    /// * `output_buses` - Map of bus ID to output buffer
    pub fn process(
        &mut self,
        decks: &mut [Deck],
        frames: u32,
        output_buses: &mut HashMap<BusId, Vec<Sample>>,
    ) -> Result<()> {
        // Ensure mix buffer is large enough
        let buffer_size = frames as usize * 2; // Stereo
        self.mix_buffer.resize(buffer_size, 0.0);

        // Clear the mix buffer
        self.mix_buffer.fill(0.0);

        // Mix all playing decks
        for deck in decks {
            if deck.state() == &crate::deck::DeckState::Playing {
                let deck_audio = deck.process(frames)?;

                // Add deck audio to mix buffer
                for (i, &sample) in deck_audio.iter().enumerate() {
                    if i < self.mix_buffer.len() {
                        self.mix_buffer[i] += sample;
                    }
                }
            }
        }

        // Apply master volume to mix
        for sample in &mut self.mix_buffer {
            *sample *= self.master_volume;
        }

        // Route to output buses
        self.route_to_buses(frames, output_buses)?;

        Ok(())
    }

    /// Route mixed audio to output buses
    fn route_to_buses(
        &self,
        frames: u32,
        output_buses: &mut HashMap<BusId, Vec<Sample>>,
    ) -> Result<()> {
        for (bus_id, output_buffer) in output_buses.iter_mut() {
            let bus_name = bus_id.as_str();

            // Ensure output buffer is large enough
            let required_size = frames as usize * 2; // Stereo
            if output_buffer.len() < required_size {
                output_buffer.resize(required_size, 0.0);
            }

            // Copy mixed audio to output buffer
            for (i, &sample) in self.mix_buffer.iter().enumerate() {
                if i < output_buffer.len() {
                    output_buffer[i] = sample;
                }
            }

            // Apply bus-specific volume
            let volume = match bus_name {
                "cue" => self.cue_volume,
                "master" => self.master_volume,
                _ => 1.0,
            };

            for sample in output_buffer.iter_mut() {
                *sample *= volume;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixer_creation() {
        let mixer = Mixer::new(48000);
        assert_eq!(mixer.master_volume(), 1.0);
        assert_eq!(mixer.cue_volume(), 1.0);
    }

    #[test]
    fn test_mixer_volume_controls() {
        let mut mixer = Mixer::new(48000);

        // Test master volume
        mixer.set_master_volume(0.5).unwrap();
        assert_eq!(mixer.master_volume(), 0.5);

        // Test cue volume
        mixer.set_cue_volume(0.7).unwrap();
        assert_eq!(mixer.cue_volume(), 0.7);

        // Test invalid volumes
        assert!(mixer.set_master_volume(-0.1).is_err());
        assert!(mixer.set_master_volume(1.1).is_err());
    }

    #[test]
    fn test_mixer_processing() {
        let mut mixer = Mixer::new(48000);
        let mut decks = vec![Deck::new(0, 48000), Deck::new(1, 48000)];

        // Start one deck
        decks[0].play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);

        // Process audio
        let result = mixer.process(&mut decks, 512, &mut output_buses);
        assert!(result.is_ok());

        // Check that output buffers have audio
        let master_audio = &output_buses[&BusId::new("master")];
        let cue_audio = &output_buses[&BusId::new("cue")];

        assert!(master_audio.iter().any(|&s| s != 0.0));
        assert!(cue_audio.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_mixer_volume_application() {
        let mut mixer = Mixer::new(48000);
        mixer.set_master_volume(0.5).unwrap();
        mixer.set_cue_volume(0.3).unwrap();

        let mut decks = vec![Deck::new(0, 48000)];
        decks[0].play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);

        mixer.process(&mut decks, 512, &mut output_buses).unwrap();

        // Check that volumes are applied correctly
        let master_audio = &output_buses[&BusId::new("master")];
        let cue_audio = &output_buses[&BusId::new("cue")];

        // Master should have higher volume than cue
        let master_max = master_audio.iter().map(|&s| s.abs()).fold(0.0, f32::max);
        let cue_max = cue_audio.iter().map(|&s| s.abs()).fold(0.0, f32::max);

        assert!(master_max > cue_max);
    }

    #[test]
    fn test_mixer_multiple_decks() {
        let mut mixer = Mixer::new(48000);
        let mut decks = vec![Deck::new(0, 48000), Deck::new(1, 48000)];

        // Start both decks
        decks[0].play().unwrap();
        decks[1].play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);

        mixer.process(&mut decks, 512, &mut output_buses).unwrap();

        // Should have mixed audio from both decks
        let master_audio = &output_buses[&BusId::new("master")];
        assert!(master_audio.iter().any(|&s| s != 0.0));
    }
}
