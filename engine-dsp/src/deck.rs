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
    /// Engine/device callback size in frames (drives rubato output chunk size).
    output_chunk_frames: u32,
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
            output_chunk_frames: 512,
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
        self.reset_resampler();
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

    /// Seek to a specific position (in source frames at the file sample rate)
    pub fn seek(&mut self, position: u64) -> Result<()> {
        self.position = position;
        self.reset_resampler();
        Ok(())
    }

    /// Set the engine output sample rate; resampling happens at playback time.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        if self.sample_rate == sample_rate {
            return;
        }

        self.sample_rate = sample_rate;
        self.configure_resampler();
    }

    /// Set the engine callback buffer size (must match CPAL `BufferSize::Fixed`).
    pub fn set_output_chunk_frames(&mut self, frames: u32) {
        let frames = frames.max(1);
        if self.output_chunk_frames == frames {
            return;
        }

        self.output_chunk_frames = frames;
        if self.resampler.is_some() || self.original_sample_rate.is_some() {
            self.configure_resampler();
        }
    }

    fn configure_resampler(&mut self) {
        let Some(original_rate) = self.original_sample_rate else {
            self.resampler = None;
            return;
        };

        if original_rate == self.sample_rate {
            self.resampler = None;
            log::info!(
                "Deck {} passthrough at {} Hz (no resampling)",
                self.id,
                self.sample_rate
            );
            return;
        }

        match create_resampler(
            original_rate,
            self.sample_rate,
            2,
            self.output_chunk_frames as usize,
        ) {
            Ok(resampler) => {
                log::info!(
                    "Deck {} realtime resampler: {} Hz -> {} Hz ({}-frame chunks)",
                    self.id,
                    original_rate,
                    self.sample_rate,
                    self.output_chunk_frames
                );
                self.resampler = Some(resampler);
            }
            Err(e) => {
                log::error!("Deck {} failed to create resampler: {}", self.id, e);
                self.resampler = None;
            }
        }
    }

    fn reset_resampler(&mut self) {
        if self.resampler.is_some() {
            self.configure_resampler();
        }
    }

    /// Load audio samples into the deck at the file's native sample rate.
    /// Resampling to the engine rate happens during playback.
    pub fn load_audio_samples(
        &mut self,
        samples: Vec<Sample>,
        original_sample_rate: u32,
        file_path: String,
    ) -> Result<()> {
        self.original_sample_rate = Some(original_sample_rate);
        self.file_path = Some(file_path);
        self.position = 0;
        self.audio_samples = Some(samples);
        self.configure_resampler();

        log::info!(
            "Loaded audio into deck {}: {} source frames at {} Hz (engine: {} Hz)",
            self.id,
            self.audio_samples.as_ref().map(|s| s.len() / 2).unwrap_or(0),
            original_sample_rate,
            self.sample_rate
        );

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
            let source_frames = self.play_loaded_audio(frames, &audio_samples);
            self.audio_samples = Some(audio_samples);
            self.position += source_frames;

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
            self.position += frames as u64;
        }

        // Apply volume
        for sample in &mut self.buffer {
            *sample *= self.volume;
        }

        Ok(&self.buffer)
    }

    /// Render loaded audio into `self.buffer`.
    ///
    /// Returns the number of source frames consumed (at the file sample rate).
    fn play_loaded_audio(&mut self, frames: u32, audio_samples: &[Sample]) -> u64 {
        let start_pos = self.position as usize * 2;

        if self.position % 1000 == 0 {
            log::debug!(
                "Deck {}: position={}, start_pos={}, audio_len={}, frames={}",
                self.id,
                self.position,
                start_pos,
                audio_samples.len(),
                frames
            );
        }

        if start_pos >= audio_samples.len() {
            self.buffer.fill(0.0);
            return 0;
        }

        if self.resampler.is_none() {
            return self.copy_source_passthrough(frames, audio_samples, start_pos);
        }

        self.resample_into_buffer(frames, audio_samples, start_pos)
    }

    fn copy_source_passthrough(
        &mut self,
        frames: u32,
        audio_samples: &[Sample],
        start_pos: usize,
    ) -> u64 {
        let available_samples = audio_samples.len() - start_pos;
        let samples_to_copy = std::cmp::min(frames as usize * 2, available_samples);

        self.buffer[..samples_to_copy]
            .copy_from_slice(&audio_samples[start_pos..start_pos + samples_to_copy]);
        self.buffer[samples_to_copy..].fill(0.0);

        (samples_to_copy / 2) as u64
    }

    fn resample_into_buffer(
        &mut self,
        frames: u32,
        audio_samples: &[Sample],
        mut src_pos: usize,
    ) -> u64 {
        let output_frames = frames as usize;
        let mut out_frames = 0usize;
        let mut total_input_frames = 0usize;
        let resampler = match self.resampler.as_mut() {
            Some(r) => r,
            None => return 0,
        };

        // Safety: keep rubato aligned if process() is ever called with a different frame count.
        if resampler.output_frames_next() != output_frames {
            resampler.set_output_chunk_frames(output_frames);
        }

        while out_frames < output_frames && src_pos < audio_samples.len() {
            let need_in = resampler.input_frames_next();
            let step_out = resampler
                .output_frames_next()
                .min(output_frames - out_frames);

            let remaining_src = (audio_samples.len() - src_pos) / 2;
            if remaining_src == 0 {
                break;
            }

            let in_frames = need_in.min(remaining_src);
            let chunk = &audio_samples[src_pos..src_pos + in_frames * 2];
            let out_start = out_frames * 2;
            let out_len = step_out * 2;

            let (out_samples, consumed) =
                resampler.process(chunk, &mut self.buffer[out_start..out_start + out_len], 2);

            let produced = out_samples / 2;
            if produced == 0 && consumed == 0 {
                break;
            }

            out_frames += produced;
            total_input_frames += consumed;
            src_pos += consumed * 2;
        }

        static mut RESAMPLE_LOG: u32 = 0;
        unsafe {
            RESAMPLE_LOG += 1;
            if RESAMPLE_LOG % 500 == 1 {
                log::info!(
                    "Deck {} resample: pos={}, consumed={}, out_frames={}/{}",
                    self.id,
                    self.position,
                    total_input_frames,
                    out_frames,
                    output_frames
                );
            }
        }

        if out_frames * 2 < self.buffer.len() {
            self.buffer[out_frames * 2..].fill(0.0);
        }

        total_input_frames as u64
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

    #[test]
    fn test_deck_realtime_resample_playback_rate() {
        let output_rate = 48000u32;
        let input_rate = 44100u32;
        let duration_secs = 10usize;
        let mut deck = Deck::new(0, output_rate);
        let frames = input_rate as usize * duration_secs;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / input_rate as f32).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }

        deck.load_audio_samples(samples, input_rate, "test.wav".to_string())
            .unwrap();
        deck.play().unwrap();

        let chunk = 512u32;
        let total_output_frames = output_rate as usize * duration_secs;
        let mut produced = 0usize;
        while produced < total_output_frames {
            let n = (total_output_frames - produced).min(chunk as usize);
            deck.process(n as u32).unwrap();
            produced += n;
        }

        let expected_source = (total_output_frames as f64 * input_rate as f64 / output_rate as f64) as u64;
        let actual = deck.position();
        let ratio = actual as f64 / expected_source as f64;
        eprintln!(
            "expected_source={expected_source}, actual={actual}, ratio={ratio:.4}"
        );
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "source consumption should match output duration (ratio {ratio:.3})"
        );
    }

    #[test]
    fn test_deck_realtime_resample_playback_rate_480_frame_callback() {
        let output_rate = 48000u32;
        let input_rate = 44100u32;
        let duration_secs = 10usize;
        let chunk = 480u32;
        let mut deck = Deck::new(0, output_rate);
        let frames = input_rate as usize * duration_secs;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / input_rate as f32).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }

        deck.load_audio_samples(samples, input_rate, "test.wav".to_string())
            .unwrap();
        deck.play().unwrap();

        let total_output_frames = output_rate as usize * duration_secs;
        let mut produced = 0usize;
        while produced < total_output_frames {
            let n = (total_output_frames - produced).min(chunk as usize);
            deck.process(n as u32).unwrap();
            produced += n;
        }

        let expected_source =
            (total_output_frames as f64 * input_rate as f64 / output_rate as f64) as u64;
        let actual = deck.position();
        let ratio = actual as f64 / expected_source as f64;
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "480-frame callbacks must not over-consume source (ratio {ratio:.3})"
        );
    }

    #[test]
    fn test_deck_realtime_resample_produces_audio() {
        let mut deck = Deck::new(0, 48000);
        let frames = 44100usize;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }

        deck.load_audio_samples(samples, 44100, "test.wav".to_string())
            .unwrap();
        deck.play().unwrap();

        let mut non_zero = 0usize;
        for _ in 0..200 {
            let audio = deck.process(512).unwrap();
            non_zero += audio.iter().filter(|&&s| s.abs() > 0.001).count();
        }

        assert!(
            non_zero > 0,
            "realtime resample should produce non-zero audio within 200 callbacks"
        );
    }
}
