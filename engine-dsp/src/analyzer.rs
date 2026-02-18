//! Audio analysis components for BPM detection and other analysis
//!
//! This module provides audio analysis capabilities including BPM detection,
//! beat tracking, and other audio analysis features.

use anyhow::Result;
use audio_core::Sample;

/// BPM analyzer for tempo detection
#[derive(Debug)]
pub struct BpmAnalyzer {
    /// Sample rate
    sample_rate: u32,
    /// Analysis buffer
    buffer: Vec<Sample>,
    /// Current BPM estimate
    current_bpm: f32,
    /// Confidence in the current BPM estimate
    confidence: f32
}

impl BpmAnalyzer {
    /// Create a new BPM analyzer
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            buffer: Vec::new(),
            current_bpm: 120.0, // Default BPM
            confidence: 0.0,
        }
    }

    /// Get the current BPM estimate
    pub fn current_bpm(&self) -> f32 {
        self.current_bpm
    }

    /// Get the confidence in the current BPM estimate
    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    /// Set the sample rate
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
    }

    /// Analyze audio for BPM detection
    ///
    /// # Arguments
    /// * `audio` - Audio buffer to analyze (interleaved stereo)
    /// * `frames` - Number of frames in the audio buffer
    ///
    /// # Returns
    /// The estimated BPM
    pub fn analyze(&mut self, audio: &[Sample], frames: u32) -> Result<f32> {
        // Ensure buffer is large enough
        let buffer_size = frames as usize * 2; // Stereo
        self.buffer.resize(buffer_size, 0.0);

        // Copy audio to analysis buffer
        for (i, &sample) in audio.iter().enumerate() {
            if i < self.buffer.len() {
                self.buffer[i] = sample;
            }
        }

        // Simple BPM detection algorithm (placeholder)
        // In a real implementation, this would use more sophisticated
        // algorithms like autocorrelation or FFT-based beat detection
        let estimated_bpm = self.detect_bpm_simple(frames)?;

        // Update current BPM with some smoothing
        self.current_bpm = 0.9 * self.current_bpm + 0.1 * estimated_bpm;

        // Calculate confidence based on signal strength
        self.confidence = self.calculate_confidence()?;

        Ok(self.current_bpm)
    }

    /// Simple BPM detection algorithm (placeholder)
    fn detect_bpm_simple(&self, frames: u32) -> Result<f32> {
        // This is a very basic implementation that just returns a
        // constant BPM. In a real implementation, this would analyze
        // the audio for rhythmic patterns.

        // For now, return a random BPM between 80 and 160
        let base_bpm = 120.0;
        let variation = (frames as f32 / 1000.0).sin() * 20.0;
        Ok(base_bpm + variation)
    }

    /// Calculate confidence in the current BPM estimate
    fn calculate_confidence(&self) -> Result<f32> {
        // Simple confidence calculation based on signal strength
        let signal_strength = self.buffer.iter().map(|&s| s.abs()).fold(0.0, f32::max);

        // Normalize to 0.0-1.0 range
        let confidence = (signal_strength * 10.0).min(1.0);
        Ok(confidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpm_analyzer_creation() {
        let analyzer = BpmAnalyzer::new(48000);
        assert_eq!(analyzer.current_bpm(), 120.0);
        assert_eq!(analyzer.confidence(), 0.0);
    }

    #[test]
    fn test_bpm_analyzer_analysis() {
        let mut analyzer = BpmAnalyzer::new(48000);

        // Create test audio (sine wave)
        let frames = 1024;
        let mut audio = vec![0.0; frames * 2]; // Stereo

        for i in 0..frames {
            let sample = 0.1 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin();
            audio[i * 2] = sample; // Left channel
            audio[i * 2 + 1] = sample; // Right channel
        }

        let bpm = analyzer.analyze(&audio, frames as u32).unwrap();
        assert!(bpm > 0.0);
        assert!(bpm < 300.0); // Reasonable BPM range
    }

    #[test]
    fn test_bpm_analyzer_confidence() {
        let mut analyzer = BpmAnalyzer::new(48000);

        // Test with silence
        let frames = 1024;
        let audio = vec![0.0; frames * 2];
        analyzer.analyze(&audio, frames as u32).unwrap();
        assert_eq!(analyzer.confidence(), 0.0);

        // Test with audio
        let mut audio = vec![0.0; frames * 2];
        for i in 0..frames {
            audio[i * 2] = 0.5; // Strong signal
        }
        analyzer.analyze(&audio, frames as u32).unwrap();
        assert!(analyzer.confidence() > 0.0);
    }

    #[test]
    fn test_bpm_analyzer_smoothing() {
        let mut analyzer = BpmAnalyzer::new(48000);

        let frames = 1024;
        let audio = vec![0.1; frames * 2];

        // Analyze multiple times to test smoothing
        let bpm1 = analyzer.analyze(&audio, frames as u32).unwrap();
        let bpm2 = analyzer.analyze(&audio, frames as u32).unwrap();

        // BPM should be smoothed between calls
        assert!(bpm1 != bpm2); // Should be different due to smoothing
    }
}
