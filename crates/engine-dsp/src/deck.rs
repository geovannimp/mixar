//! Audio deck implementation for DJ-style playback
//!
//! A deck represents a single audio playback unit with transport and
//! playback-speed controls.

use crate::transport::DeckTransportEvent;
use anyhow::Result;
use audio_core::{LoadedAudio, Sample};
use dasp_graph::Buffer;
use resampler::Resampler;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

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
    /// Current position in samples (source frames, integer floor)
    position: u64,
    /// Sub-frame source position for tempo / loop accuracy
    position_frac: f64,
    /// Playback speed (1.0 = normal speed)
    speed: f32,
    /// Immutable engine output sample rate (from config).
    sample_rate: u32,
    /// Immutable engine callback size in frames (from config).
    buffer_size: u32,
    /// Internal buffer for audio processing
    buffer: Vec<Sample>,
    /// Whether the deck is currently processing audio
    processing: bool,
    /// Shared decoded audio (cache and multiple decks can reference the same buffer).
    loaded: Option<Arc<LoadedAudio>>,
    /// Resampler quality preset (`low`, `medium`, or `high`).
    resampler_quality: String,
    /// Resampler for converting between sample rates (created on load when needed)
    resampler: Option<Box<dyn Resampler>>,
    /// Active loop region in source frames (inclusive start, exclusive end).
    loop_region: Option<(f64, f64)>,
    /// Temporary cue point in seconds (source time).
    cue_point_secs: Option<f64>,
    /// Saved transport state while cue is held.
    cue_hold_return: Option<(f64, bool)>,
    /// Events to deliver to the engine notifier after processing.
    pending_transport: Vec<DeckTransportEvent>,
}

impl fmt::Debug for Deck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Deck")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("position", &self.position)
            .field("speed", &self.speed)
            .field("sample_rate", &self.sample_rate)
            .field("buffer_size", &self.buffer_size)
            .field("processing", &self.processing)
            .field(
                "loaded",
                &self.loaded.as_ref().map(|audio| audio.samples.len()),
            )
            .field(
                "source_id",
                &self.loaded.as_ref().map(|audio| audio.source_id.as_str()),
            )
            .field("resampler", &"<resampler>")
            .finish()
    }
}

impl Deck {
    /// Create a new deck with an immutable output clock from engine config.
    pub fn new(id: usize, sample_rate: u32, buffer_size: u32, resampler_quality: &str) -> Self {
        Self {
            id,
            state: DeckState::Stopped,
            position: 0,
            position_frac: 0.0,
            speed: 1.0,
            sample_rate,
            buffer_size: buffer_size.max(1),
            buffer: Vec::new(),
            processing: false,
            loaded: None,
            resampler_quality: resampler_quality.to_string(),
            resampler: None,
            loop_region: None,
            cue_point_secs: None,
            cue_hold_return: None,
            pending_transport: Vec::new(),
        }
    }

    /// Take pending transport events produced during the last process cycle.
    pub fn drain_transport_events(&mut self) -> Vec<DeckTransportEvent> {
        std::mem::take(&mut self.pending_transport)
    }

    fn check_track_end(&mut self) {
        if self.state != DeckState::Playing || self.loop_region.is_some() {
            return;
        }
        let Some(audio) = self.loaded.as_ref() else {
            return;
        };
        let total_frames = audio.samples.len() / 2;
        if total_frames == 0 {
            return;
        }
        if self.position_frac >= total_frames as f64 {
            self.state = DeckState::Paused;
            self.pending_transport.push(DeckTransportEvent::TrackEnded);
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

    /// Current playback position in seconds (source file time).
    pub fn position_seconds(&self) -> Option<f64> {
        let audio = self.loaded.as_ref()?;
        Some(self.position_frac / f64::from(audio.sample_rate))
    }

    /// Temporary cue point in seconds, if set.
    pub fn cue_point_secs(&self) -> Option<f64> {
        self.cue_point_secs
    }

    /// Active loop region in seconds, if any.
    pub fn loop_region_secs(&self) -> Option<(f64, f64)> {
        let audio = self.loaded.as_ref()?;
        let rate = f64::from(audio.sample_rate);
        self.loop_region
            .map(|(start, end)| (start / rate, end / rate))
    }

    /// Get the current playback speed
    pub fn speed(&self) -> f32 {
        self.speed
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
        self.position_frac = 0.0;
        self.cue_hold_return = None;
        self.reset_resampler_state();
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

    /// Seek to a specific position (in source frames at the file sample rate)
    pub fn seek(&mut self, position: u64) -> Result<()> {
        self.position = position;
        self.position_frac = position as f64;
        self.reset_resampler_state();
        Ok(())
    }

    /// Seek to a position in seconds (source file time).
    pub fn seek_secs(&mut self, secs: f64) -> Result<()> {
        let audio = self
            .loaded
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No track loaded"))?;
        let max_secs = audio.samples.len() as f64 / 2.0 / f64::from(audio.sample_rate);
        let clamped = secs.clamp(0.0, max_secs);
        let frames = clamped * f64::from(audio.sample_rate);
        self.position = frames as u64;
        self.position_frac = frames;
        self.reset_resampler_state();
        Ok(())
    }

    /// Clear loaded audio and reset transport state.
    pub fn unload(&mut self) -> Result<()> {
        self.state = DeckState::Stopped;
        self.position = 0;
        self.position_frac = 0.0;
        self.loaded = None;
        self.resampler = None;
        self.loop_region = None;
        self.cue_point_secs = None;
        self.cue_hold_return = None;
        self.buffer.clear();
        Ok(())
    }

    /// Store the temporary cue point at the given source time.
    pub fn set_cue_point_secs(&mut self, secs: f64) -> Result<()> {
        self.cue_point_secs = Some(secs.max(0.0));
        Ok(())
    }

    /// Jump to cue and play while the button is held.
    pub fn begin_cue_hold(&mut self) -> Result<()> {
        let cue_secs = self
            .cue_point_secs
            .or_else(|| self.position_seconds())
            .ok_or_else(|| anyhow::anyhow!("No track loaded"))?;
        let return_pos = self.position_seconds().unwrap_or(0.0);
        let was_playing = self.state == DeckState::Playing;
        self.cue_hold_return = Some((return_pos, was_playing));
        self.seek_secs(cue_secs)?;
        self.play()?;
        Ok(())
    }

    /// Return to the pre-hold transport state.
    pub fn end_cue_hold(&mut self) -> Result<()> {
        let Some((return_pos, was_playing)) = self.cue_hold_return.take() else {
            return Ok(());
        };
        self.seek_secs(return_pos)?;
        if was_playing {
            self.play()?;
        } else {
            self.pause()?;
        }
        Ok(())
    }

    /// Activate a loop region in source seconds.
    pub fn set_loop_region_secs(&mut self, in_secs: f64, out_secs: f64) -> Result<()> {
        let audio = self
            .loaded
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No track loaded"))?;
        if out_secs <= in_secs {
            return Err(anyhow::anyhow!("Loop out must be after loop in"));
        }
        let rate = f64::from(audio.sample_rate);
        self.loop_region = Some((in_secs * rate, out_secs * rate));
        Ok(())
    }

    /// Clear the active loop region.
    pub fn clear_loop(&mut self) {
        self.loop_region = None;
    }

    /// Create or clear the resampler based on the loaded source rate and the
    /// immutable engine output clock. Called on load only.
    fn create_resampler(&mut self) -> Result<()> {
        let Some(source_rate) = self.loaded.as_ref().map(|audio| audio.sample_rate) else {
            self.resampler = None;
            return Ok(());
        };

        if source_rate == self.sample_rate {
            self.resampler = None;
            log::info!(
                "Deck {} passthrough at {} Hz (no resampling)",
                self.id,
                self.sample_rate
            );
            return Ok(());
        }

        // Mixer processes dasp_graph Buffer::LEN chunks, not the device buffer size.
        // Rubato FixedSync::Output must match that quantum or it returns (0,0) forever.
        self.resampler = Some(resampler::create_resampler(
            source_rate,
            self.sample_rate,
            2,
            Buffer::LEN,
            Some(&self.resampler_quality),
        )?);
        log::info!(
            "Deck {} realtime resampler: {} Hz -> {} Hz (chunk_frames={}, quality={})",
            self.id,
            source_rate,
            self.sample_rate,
            Buffer::LEN,
            self.resampler_quality
        );
        Ok(())
    }

    /// Clear resampler filter/history state after a position jump (seek/stop).
    fn reset_resampler_state(&mut self) {
        if let Some(resampler) = self.resampler.as_mut() {
            resampler.reset();
        }
    }

    /// Load shared decoded audio. Creates a resampler when the source rate differs from the engine rate.
    pub fn load(&mut self, audio: Arc<LoadedAudio>) -> Result<()> {
        self.position = 0;
        self.position_frac = 0.0;
        self.loop_region = None;
        self.cue_point_secs = Some(0.0);
        self.cue_hold_return = None;
        self.loaded = Some(audio);
        self.create_resampler()?;

        if let Some(loaded) = self.loaded.as_ref() {
            log::info!(
                "Loaded audio into deck {} from {}: {} source frames at {} Hz ({} channels, engine: {} Hz)",
                self.id,
                loaded.source_id,
                loaded.samples.len() / 2,
                loaded.sample_rate,
                loaded.channels,
                self.sample_rate
            );
        }

        Ok(())
    }

    /// Borrow the loaded audio reference, if any.
    pub fn loaded_audio(&self) -> Option<&Arc<LoadedAudio>> {
        self.loaded.as_ref()
    }

    /// Check if audio is loaded
    pub fn has_audio_loaded(&self) -> bool {
        self.loaded.is_some()
    }

    /// Get the loaded file path
    pub fn file_path(&self) -> Option<&str> {
        self.loaded.as_ref().map(|audio| audio.source_id.as_str())
    }

    /// Get the duration of loaded audio in seconds
    pub fn duration_seconds(&self) -> Option<f64> {
        let audio = self.loaded.as_ref()?;
        let frame_count = audio.samples.len() / 2;
        Some(frame_count as f64 / audio.sample_rate as f64)
    }

    /// Process audio for this deck.
    ///
    /// # Arguments
    /// * `frames` - Number of frames to process (should match the engine buffer size)
    pub fn process(&mut self, frames: usize) -> Result<&[Sample]> {
        if self.state != DeckState::Playing {
            // Return silence if not playing. resize() only zero-fills new capacity;
            // after playback the buffer may already be the right length with stale audio.
            self.buffer.resize(frames * 2, 0.0);
            self.buffer.fill(0.0);
            return Ok(&self.buffer);
        }

        // Ensure buffer is large enough for stereo output
        let buffer_size = frames * 2;
        self.buffer.resize(buffer_size, 0.0);

        // Play loaded audio samples if available, otherwise generate test audio
        if let Some(loaded) = self.loaded.clone() {
            let source_rate = loaded.sample_rate;
            let use_interp = (self.speed - 1.0).abs() > f32::EPSILON || self.loop_region.is_some();
            if use_interp || self.resampler.is_none() {
                self.play_interpolated(frames, &loaded.samples, source_rate);
            } else {
                let source_frames = self.play_loaded_audio(frames, &loaded.samples);
                self.position += source_frames;
                self.position_frac = self.position as f64;
            }

            // Debug: Check if we're producing non-zero samples
            let non_zero_count = self.buffer.iter().filter(|&&s| s.abs() > 0.001).count();
            static mut PLAY_LOG_COUNT: u32 = 0;
            unsafe {
                PLAY_LOG_COUNT += 1;
                if PLAY_LOG_COUNT.is_multiple_of(100) {
                    log::info!(
                        "Deck {} playing: {} non-zero samples out of {}",
                        self.id,
                        non_zero_count,
                        self.buffer.len()
                    );
                }
            }
        } else {
            static NO_TRACK_WARN: AtomicU32 = AtomicU32::new(0);
            if NO_TRACK_WARN.fetch_add(1, Ordering::Relaxed) == 0 {
                log::warn!(
                    "Deck {} is playing but no track is loaded; outputting silence",
                    self.id
                );
            }
            self.buffer.fill(0.0);
        }

        self.check_track_end();

        Ok(&self.buffer)
    }

    /// Render loaded audio with fractional stepping (vinyl tempo + loops).
    fn play_interpolated(&mut self, frames: usize, audio_samples: &[Sample], source_rate: u32) {
        let buffer_size = frames * 2;
        self.buffer.resize(buffer_size, 0.0);

        let total_source_frames = audio_samples.len() / 2;
        let step = self.speed as f64 * f64::from(source_rate) / f64::from(self.sample_rate);

        for out in 0..frames {
            if self.position_frac >= total_source_frames as f64 {
                self.buffer[out * 2] = 0.0;
                self.buffer[out * 2 + 1] = 0.0;
                continue;
            }

            let (left, right) = interpolate_stereo(audio_samples, self.position_frac);
            self.buffer[out * 2] = left;
            self.buffer[out * 2 + 1] = right;
            self.position_frac += step;

            if let Some((loop_in, loop_out)) = self.loop_region {
                if self.position_frac >= loop_out {
                    let span = loop_out - loop_in;
                    if span > 0.0 {
                        self.position_frac = loop_in + ((self.position_frac - loop_out) % span);
                    }
                }
            }
        }

        self.position = self.position_frac as u64;
    }

    /// Render loaded audio into `self.buffer`.
    ///
    /// Returns the number of source frames consumed (at the file sample rate).
    fn play_loaded_audio(&mut self, frames: usize, audio_samples: &[Sample]) -> u64 {
        let start_pos = self.position_frac as usize * 2;

        if self.position.is_multiple_of(1000) {
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

        if let Some(resampler) = self.resampler.as_mut() {
            resampler.set_output_chunk_frames(frames);
        }
        self.resample_into_buffer(frames, audio_samples, start_pos)
    }

    fn copy_source_passthrough(
        &mut self,
        frames: usize,
        audio_samples: &[Sample],
        start_pos: usize,
    ) -> u64 {
        let available_samples = audio_samples.len() - start_pos;
        let samples_to_copy = std::cmp::min(frames * 2, available_samples);

        self.buffer[..samples_to_copy]
            .copy_from_slice(&audio_samples[start_pos..start_pos + samples_to_copy]);
        self.buffer[samples_to_copy..].fill(0.0);

        (samples_to_copy / 2) as u64
    }

    fn resample_into_buffer(
        &mut self,
        frames: usize,
        audio_samples: &[Sample],
        mut src_pos: usize,
    ) -> u64 {
        let output_frames = frames;
        let mut out_frames = 0usize;
        let mut total_input_frames = 0usize;
        let resampler = match self.resampler.as_mut() {
            Some(r) => r,
            None => return 0,
        };

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
}

fn interpolate_stereo(samples: &[Sample], pos: f64) -> (Sample, Sample) {
    let frame = pos.floor() as usize;
    let frac = (pos - frame as f64) as Sample;
    let i0 = frame * 2;
    if i0 + 3 >= samples.len() {
        if i0 + 1 < samples.len() {
            return (samples[i0], samples[i0 + 1]);
        }
        return (0.0, 0.0);
    }
    let l0 = samples[i0];
    let r0 = samples[i0 + 1];
    let l1 = samples[i0 + 2];
    let r1 = samples[i0 + 3];
    (l0 + (l1 - l0) * frac, r0 + (r1 - r0) * frac)
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_core::LoadedAudio;
    use std::sync::Arc;

    const ENGINE_RATE: u32 = 48000;
    const CHUNK: usize = 512;

    fn new_deck(chunk_frames: usize) -> Deck {
        Deck::new(0, ENGINE_RATE, chunk_frames as u32, "medium")
    }

    fn load_test_samples(deck: &mut Deck, samples: Vec<Sample>, sample_rate: u32) {
        let audio = LoadedAudio {
            samples,
            sample_rate,
            channels: 2,
            source_id: "test.wav".to_string(),
        };
        deck.load(Arc::new(audio)).unwrap();
    }

    #[test]
    fn test_deck_creation() {
        let deck = new_deck(CHUNK);
        assert_eq!(deck.id(), 0);
        assert_eq!(deck.state(), &DeckState::Stopped);
        assert_eq!(deck.position(), 0);
        assert_eq!(deck.speed(), 1.0);
    }

    #[test]
    fn test_deck_playback_controls() {
        let mut deck = new_deck(CHUNK);

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
        let mut deck = new_deck(CHUNK);

        // Test valid speed
        deck.set_speed(1.5).unwrap();
        assert_eq!(deck.speed(), 1.5);

        // Test invalid speed
        assert!(deck.set_speed(-1.0).is_err());
    }

    #[test]
    fn test_deck_seek() {
        let mut deck = new_deck(CHUNK);

        deck.seek(1000).unwrap();
        assert_eq!(deck.position(), 1000);
    }

    #[test]
    fn test_deck_audio_processing() {
        let mut deck = new_deck(CHUNK);

        // Test processing when stopped (should return silence)
        let audio = deck.process(CHUNK).unwrap();
        assert_eq!(audio.len(), 1024); // Stereo
        assert!(audio.iter().all(|&s| s == 0.0));

        // Playing with no track loaded outputs silence
        deck.play().unwrap();
        let audio = deck.process(CHUNK).unwrap();
        assert_eq!(audio.len(), 1024);
        assert!(audio.iter().all(|&s| s == 0.0));

        // Loaded audio should produce non-zero samples
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2], ENGINE_RATE);
        let audio = deck.process(CHUNK).unwrap();
        assert!(audio.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn process_returns_dry_playback_samples() {
        let mut deck = new_deck(2);
        let samples = vec![0.25, -0.5, 0.75, -1.0];
        load_test_samples(&mut deck, samples.clone(), ENGINE_RATE);
        deck.play().unwrap();

        let output = deck.process(2).unwrap();

        assert_eq!(output, samples);
    }

    #[test]
    fn test_deck_realtime_resample_playback_rate() {
        let input_rate = 44100u32;
        let duration_secs = 10usize;
        let mut deck = new_deck(CHUNK);
        let frames = input_rate as usize * duration_secs;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / input_rate as f32).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }

        load_test_samples(&mut deck, samples, input_rate);
        deck.play().unwrap();

        // Only full chunks — resampler is sized to the immutable buffer size.
        let total_output_frames = (ENGINE_RATE as usize * duration_secs / CHUNK) * CHUNK;
        let mut produced = 0usize;
        while produced < total_output_frames {
            deck.process(CHUNK).unwrap();
            produced += CHUNK;
        }

        let expected_source =
            (total_output_frames as f64 * input_rate as f64 / ENGINE_RATE as f64) as u64;
        let actual = deck.position();
        let ratio = actual as f64 / expected_source as f64;
        eprintln!("expected_source={expected_source}, actual={actual}, ratio={ratio:.4}");
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "source consumption should match output duration (ratio {ratio:.3})"
        );
    }

    #[test]
    fn test_deck_realtime_resample_playback_rate_480_frame_callback() {
        let input_rate = 44100u32;
        let duration_secs = 10usize;
        let chunk = 480usize;
        let mut deck = new_deck(chunk);
        let frames = input_rate as usize * duration_secs;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / input_rate as f32).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }

        load_test_samples(&mut deck, samples, input_rate);
        deck.play().unwrap();

        let total_output_frames = (ENGINE_RATE as usize * duration_secs / chunk) * chunk;
        let mut produced = 0usize;
        while produced < total_output_frames {
            deck.process(chunk).unwrap();
            produced += chunk;
        }

        let expected_source =
            (total_output_frames as f64 * input_rate as f64 / ENGINE_RATE as f64) as u64;
        let actual = deck.position();
        let ratio = actual as f64 / expected_source as f64;
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "480-frame callbacks must not over-consume source (ratio {ratio:.3})"
        );
    }

    #[test]
    fn test_deck_resample_advances_when_process_uses_graph_chunk() {
        // Engine device buffer is often 512; mixer always asks for Buffer::LEN (64).
        // Rubato FixedSync::Output sized to 512 returns (0,0) into a 64-frame out buf.
        let mut deck = new_deck(CHUNK);
        let frames = 44100usize;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }
        load_test_samples(&mut deck, samples, 44100);
        deck.play().unwrap();

        let mut heard = false;
        for _ in 0..8 {
            let audio = deck.process(Buffer::LEN).unwrap();
            heard |= audio.iter().any(|&s| s.abs() > 0.01);
        }
        assert!(
            heard,
            "graph-sized process must produce audio after 44.1→48k resample"
        );
        assert!(
            deck.position() > 0,
            "source position must advance (got {})",
            deck.position()
        );
    }

    #[test]
    fn test_deck_realtime_resample_produces_audio() {
        let mut deck = new_deck(CHUNK);
        let frames = 44100usize;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }

        load_test_samples(&mut deck, samples, 44100);
        deck.play().unwrap();

        let mut non_zero = 0usize;
        for _ in 0..200 {
            let audio = deck.process(CHUNK).unwrap();
            non_zero += audio.iter().filter(|&&s| s.abs() > 0.001).count();
        }

        assert!(
            non_zero > 0,
            "realtime resample should produce non-zero audio within 200 callbacks"
        );
    }

    #[test]
    fn test_deck_shares_loaded_audio_arc() {
        let audio = Arc::new(LoadedAudio {
            samples: vec![0.5f32; CHUNK * 2],
            sample_rate: ENGINE_RATE,
            channels: 2,
            source_id: "test.wav".to_string(),
        });

        let mut deck_a = new_deck(CHUNK);
        let mut deck_b = new_deck(CHUNK);
        deck_a.load(Arc::clone(&audio)).unwrap();
        deck_b.load(Arc::clone(&audio)).unwrap();

        assert!(Arc::ptr_eq(
            deck_a.loaded_audio().unwrap(),
            deck_b.loaded_audio().unwrap()
        ));
    }

    #[test]
    fn test_deck_pause_outputs_silence_not_stale_buffer() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.8f32; CHUNK * 2], ENGINE_RATE);
        deck.play().unwrap();

        let playing = deck.process(CHUNK).unwrap();
        assert!(playing.iter().any(|&s| s.abs() > 0.01));

        deck.pause().unwrap();
        let paused = deck.process(CHUNK).unwrap();
        assert!(
            paused.iter().all(|&s| s == 0.0),
            "paused deck must output silence, got stale samples: max={}",
            paused.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
        );
    }

    #[test]
    fn test_deck_stop_outputs_silence_not_stale_buffer() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.8f32; CHUNK * 2], ENGINE_RATE);
        deck.play().unwrap();
        deck.process(CHUNK).unwrap();

        deck.stop().unwrap();
        let stopped = deck.process(CHUNK).unwrap();
        assert!(
            stopped.iter().all(|&s| s == 0.0),
            "stopped deck must output silence, got stale samples: max={}",
            stopped.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
        );
    }
}
