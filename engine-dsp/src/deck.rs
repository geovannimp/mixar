//! Audio deck implementation for DJ-style playback
//!
//! A deck represents a single audio playback unit with controls for
//! play/pause, volume, pitch, and other DJ-style features.

use anyhow::Result;
use audio_core::Sample;
use resampler::{create_resampler, Resampler};
use std::fmt;

/// Audio deck state
#[derive(Debug, Clone, PartialEq)]
pub enum DeckState {
    /// Deck is stopped
    Stopped,
    /// Deck is playing
    Playing,
    /// Deck is paused
    Paused,
}

/// Audio deck for DJ-style playback
pub struct Deck {
    /// Deck identifier
    id: usize,
    /// Current state
    state: DeckState,
    /// Current position in samples
    position: u64,
    /// Playback speed (1.0 = normal speed)
    speed: f32,
    /// Volume level (0.0 to 1.0)
    volume: f32,
    /// Sample rate
    sample_rate: u32,
    /// Internal buffer for audio processing
    buffer: Vec<Sample>,
    /// Whether the deck is currently processing audio
    processing: bool,
    /// Loaded audio samples (interleaved stereo)
    audio_samples: Option<Vec<Sample>>,
    /// Original sample rate of loaded audio
    original_sample_rate: Option<u32>,
    /// File path of loaded track
    file_path: Option<String>,
    /// Resampler for converting between sample rates
    resampler: Option<Box<dyn Resampler>>,
}

impl fmt::Debug for Deck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Deck")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("position", &self.position)
            .field("speed", &self.speed)
            .field("volume", &self.volume)
            .field("sample_rate", &self.sample_rate)
            .field("processing", &self.processing)
            .field(
                "audio_samples",
                &self.audio_samples.as_ref().map(|s| s.len()),
            )
            .field("original_sample_rate", &self.original_sample_rate)
            .field("file_path", &self.file_path)
            .field("resampler", &"<resampler>")
            .finish()
    }
}

impl Deck {
    /// Create a new deck
    pub fn new(id: usize, sample_rate: u32) -> Self {
        Self {
            id,
            state: DeckState::Stopped,
            position: 0,
            speed: 1.0,
            volume: 1.0,
            sample_rate,
            buffer: Vec::new(),
            processing: false,
            audio_samples: None,
            original_sample_rate: None,
            file_path: None,
            resampler: None,
        }
    }

    /// Get the deck ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the current state
    pub fn state(&self) -> &DeckState {
        &self.state
    }

    /// Get the current position in samples
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Get the current playback speed
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Get the current volume
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Start playback
    pub fn play(&mut self) -> Result<()> {
        log::info!(
            "Deck {} changing state to Playing (was: {:?})",
            self.id,
            self.state
        );
        self.state = DeckState::Playing;
        Ok(())
    }

    /// Pause playback
    pub fn pause(&mut self) -> Result<()> {
        self.state = DeckState::Paused;
        Ok(())
    }

    /// Stop playback and reset position
    pub fn stop(&mut self) -> Result<()> {
        self.state = DeckState::Stopped;
        self.position = 0;
        Ok(())
    }

    /// Set playback speed
    pub fn set_speed(&mut self, speed: f32) -> Result<()> {
        if speed < 0.0 {
            return Err(anyhow::anyhow!("Speed cannot be negative"));
        }
        self.speed = speed;
        Ok(())
    }

    /// Set volume level
    pub fn set_volume(&mut self, volume: f32) -> Result<()> {
        if volume < 0.0 || volume > 1.0 {
            return Err(anyhow::anyhow!("Volume must be between 0.0 and 1.0"));
        }
        self.volume = volume;
        Ok(())
    }

    /// Seek to a specific position
    pub fn seek(&mut self, position: u64) -> Result<()> {
        self.position = position;
        Ok(())
    }

    /// Set the sample rate
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
    }

    /// Load audio samples into the deck
    pub fn load_audio_samples(
        &mut self,
        samples: Vec<Sample>,
        original_sample_rate: u32,
        file_path: String,
    ) -> Result<()> {
        self.original_sample_rate = Some(original_sample_rate);
        self.file_path = Some(file_path);
        self.position = 0; // Reset position when loading new audio

        // Check if resampling is needed
        if original_sample_rate != self.sample_rate {
            log::info!(
                "Resampling audio from {} Hz to {} Hz",
                original_sample_rate,
                self.sample_rate
            );

            // Debug: Check original samples before resampling
            let first_10_original: Vec<f32> = samples.iter().take(10).cloned().collect();
            log::info!("Original audio samples (first 10): {:?}", first_10_original);

            // Create resampler
            let mut resampler = create_resampler(original_sample_rate, self.sample_rate, 2)?;

            // Resample the entire audio file - FAST processing for testing
            let mut resampled_samples = Vec::new();
            let chunk_size = 4096 * 2; // Larger chunks for faster processing (was 1024 * 2)

            for chunk in samples.chunks(chunk_size) {
                let mut output_chunk = vec![0.0; chunk_size * 2]; // Oversize output buffer
                let processed = resampler.process(chunk, &mut output_chunk, 2);
                resampled_samples.extend_from_slice(&output_chunk[..processed]);
            }

            // Debug: Check resampled samples
            let first_10_resampled: Vec<f32> = resampled_samples.iter().take(10).cloned().collect();
            log::info!(
                "Resampled audio samples (first 10): {:?}",
                first_10_resampled
            );

            self.audio_samples = Some(resampled_samples);
            log::info!(
                "Resampled audio: {} samples at {} Hz -> {} samples at {} Hz",
                samples.len() / 2,
                original_sample_rate,
                self.audio_samples.as_ref().unwrap().len() / 2,
                self.sample_rate
            );
        } else {
            // No resampling needed
            self.audio_samples = Some(samples);
            log::info!(
                "Loaded audio samples: {} samples at {} Hz (no resampling needed)",
                self.audio_samples.as_ref().unwrap().len() / 2,
                original_sample_rate
            );
        }

        Ok(())
    }

    /// Check if audio is loaded
    pub fn has_audio_loaded(&self) -> bool {
        self.audio_samples.is_some()
    }

    /// Get the loaded file path
    pub fn file_path(&self) -> Option<&String> {
        self.file_path.as_ref()
    }

    /// Get the duration of loaded audio in seconds
    pub fn duration_seconds(&self) -> Option<f64> {
        if let (Some(samples), Some(original_rate)) =
            (&self.audio_samples, &self.original_sample_rate)
        {
            let frame_count = samples.len() / 2; // Stereo samples
            Some(frame_count as f64 / *original_rate as f64)
        } else {
            None
        }
    }

    /// Process audio for this deck
    ///
    /// # Arguments
    /// * `frames` - Number of frames to process
    ///
    /// # Returns
    /// The processed audio buffer
    pub fn process(&mut self, frames: u32) -> Result<&[Sample]> {
        if self.state != DeckState::Playing {
            // Return silence if not playing
            self.buffer.resize(frames as usize * 2, 0.0); // Stereo
                                                          // Debug: Log deck state
            static mut STATE_LOG_COUNT: u32 = 0;
            unsafe {
                STATE_LOG_COUNT += 1;
                if STATE_LOG_COUNT % 100 == 0 {
                    log::warn!("Deck {} not playing, state: {:?}", self.id, self.state);
                }
            }
            return Ok(&self.buffer);
        }

        // Ensure buffer is large enough for stereo output
        let buffer_size = frames as usize * 2;
        self.buffer.resize(buffer_size, 0.0);

        // Play loaded audio samples if available, otherwise generate test audio
        if let Some(audio_samples) = self.audio_samples.take() {
            log::info!(
                "Deck {} playing loaded audio: {} samples available",
                self.id,
                audio_samples.len()
            );
            self.play_loaded_audio(frames, &audio_samples);
            self.audio_samples = Some(audio_samples); // Put it back

            // Debug: Check if we're producing non-zero samples
            let non_zero_count = self.buffer.iter().filter(|&&s| s.abs() > 0.001).count();
            static mut PLAY_LOG_COUNT: u32 = 0;
            unsafe {
                PLAY_LOG_COUNT += 1;
                if PLAY_LOG_COUNT % 100 == 0 {
                    log::info!(
                        "Deck {} playing: {} non-zero samples out of {}",
                        self.id,
                        non_zero_count,
                        self.buffer.len()
                    );
                }
            }
        } else {
            // Fallback to test audio if no samples loaded
            log::warn!(
                "Deck {} has no audio samples loaded, generating test audio",
                self.id
            );
            self.generate_test_audio(frames);
        }

        // Apply volume
        for sample in &mut self.buffer {
            *sample *= self.volume;
        }

        // Update position based on the deck's sample rate
        // The position should advance by the number of frames processed
        // adjusted for any sample rate differences
        if let Some(original_rate) = self.original_sample_rate {
            // If we have the original sample rate, we need to account for any difference
            // For now, we'll assume the deck sample rate matches the engine rate
            // and the position represents frames at the deck's sample rate
            self.position += frames as u64;
        } else {
            // No original sample rate info, just advance by frames
            self.position += frames as u64;
        }

        Ok(&self.buffer)
    }

    /// Play loaded audio samples
    fn play_loaded_audio(&mut self, frames: u32, audio_samples: &[Sample]) {
        let start_pos = self.position as usize * 2; // Convert to sample index (stereo)

        // Debug logging (only log occasionally to avoid spam)
        if self.position % 1000 == 0 {
            log::debug!(
                "Deck {}: position={}, start_pos={}, audio_len={}, frames={}",
                self.id,
                self.position,
                start_pos,
                audio_samples.len(),
                frames
            );

            // Check if the audio samples actually contain data
            if start_pos < audio_samples.len() {
                let sample_count = std::cmp::min(10, audio_samples.len() - start_pos);
                let sample_values: Vec<f32> =
                    audio_samples[start_pos..start_pos + sample_count].to_vec();
                log::debug!(
                    "Deck {}: first {} samples: {:?}",
                    self.id,
                    sample_count,
                    sample_values
                );
            }
        }

        // Check if we've reached the end of the audio
        if start_pos >= audio_samples.len() {
            // Audio finished, fill with silence
            self.buffer.fill(0.0);
            return;
        }

        // Copy samples from loaded audio
        let available_samples = audio_samples.len() - start_pos;
        let samples_to_copy = std::cmp::min(frames as usize * 2, available_samples);

        // Copy the samples efficiently
        if start_pos + samples_to_copy <= audio_samples.len() {
            // Safe to copy all samples at once
            self.buffer[..samples_to_copy]
                .copy_from_slice(&audio_samples[start_pos..start_pos + samples_to_copy]);
        } else {
            // Copy what we can, fill the rest with silence
            let safe_copy = audio_samples.len() - start_pos;
            self.buffer[..safe_copy].copy_from_slice(&audio_samples[start_pos..]);
            self.buffer[safe_copy..samples_to_copy].fill(0.0);
        }

        // Fill remaining buffer with silence if needed
        self.buffer[samples_to_copy..].fill(0.0);
    }

    /// Generate test audio (placeholder implementation)
    fn generate_test_audio(&mut self, frames: u32) {
        let frequency = 440.0 * self.speed; // A4 note adjusted for speed
        let amplitude = 0.1;

        for frame in 0..frames {
            let phase = (self.position + frame as u64) as f32 / self.sample_rate as f32;
            let sample = amplitude * (2.0 * std::f32::consts::PI * frequency * phase).sin();

            // Write to both channels (interleaved)
            let left_idx = (frame * 2) as usize;
            let right_idx = (frame * 2 + 1) as usize;

            if left_idx < self.buffer.len() {
                self.buffer[left_idx] = sample;
            }
            if right_idx < self.buffer.len() {
                self.buffer[right_idx] = sample;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deck_creation() {
        let deck = Deck::new(0, 48000);
        assert_eq!(deck.id(), 0);
        assert_eq!(deck.state(), &DeckState::Stopped);
        assert_eq!(deck.position(), 0);
        assert_eq!(deck.speed(), 1.0);
        assert_eq!(deck.volume(), 1.0);
    }

    #[test]
    fn test_deck_playback_controls() {
        let mut deck = Deck::new(0, 48000);

        // Test play
        deck.play().unwrap();
        assert_eq!(deck.state(), &DeckState::Playing);

        // Test pause
        deck.pause().unwrap();
        assert_eq!(deck.state(), &DeckState::Paused);

        // Test stop
        deck.stop().unwrap();
        assert_eq!(deck.state(), &DeckState::Stopped);
        assert_eq!(deck.position(), 0);
    }

    #[test]
    fn test_deck_speed_control() {
        let mut deck = Deck::new(0, 48000);

        // Test valid speed
        deck.set_speed(1.5).unwrap();
        assert_eq!(deck.speed(), 1.5);

        // Test invalid speed
        assert!(deck.set_speed(-1.0).is_err());
    }

    #[test]
    fn test_deck_volume_control() {
        let mut deck = Deck::new(0, 48000);

        // Test valid volume
        deck.set_volume(0.5).unwrap();
        assert_eq!(deck.volume(), 0.5);

        // Test invalid volume
        assert!(deck.set_volume(-0.1).is_err());
        assert!(deck.set_volume(1.1).is_err());
    }

    #[test]
    fn test_deck_seek() {
        let mut deck = Deck::new(0, 48000);

        deck.seek(1000).unwrap();
        assert_eq!(deck.position(), 1000);
    }

    #[test]
    fn test_deck_audio_processing() {
        let mut deck = Deck::new(0, 48000);

        // Test processing when stopped (should return silence)
        let audio = deck.process(512).unwrap();
        assert_eq!(audio.len(), 1024); // Stereo
        assert!(audio.iter().all(|&s| s == 0.0));

        // Test processing when playing
        deck.play().unwrap();
        let audio = deck.process(512).unwrap();
        assert_eq!(audio.len(), 1024); // Stereo
        assert!(audio.iter().any(|&s| s != 0.0)); // Should have some audio
    }

    #[test]
    fn test_deck_volume_application() {
        let mut deck = Deck::new(0, 48000);
        deck.set_volume(0.5).unwrap();
        deck.play().unwrap();

        let audio = deck.process(512).unwrap();
        assert!(audio.iter().all(|&s| s.abs() <= 0.05)); // Max amplitude should be 0.1 * 0.5
    }
}
