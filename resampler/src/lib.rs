//! Audio resampler for rust-dj-engine
//!
//! This crate provides audio resampling capabilities using rubato.

use anyhow::Result;
use audio_core::Sample;
use rubato::{
    Resampler as RubatoResamplerTrait, SincFixedIn, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};

/// Resampler trait
pub trait Resampler: Send {
    /// Process audio samples
    fn process(&mut self, in_buf: &[Sample], out_buf: &mut [Sample], channels: usize) -> usize;

    /// Set the sample rate
    fn set_rate(&mut self, input_sr: u32, output_sr: u32);
}

/// Rubato resampler implementation
pub struct RubatoResampler {
    resampler: Option<SincFixedIn<f32>>,
    input_sample_rate: u32,
    output_sample_rate: u32,
    channels: usize,
    input_buffer: Vec<Vec<f32>>,
    output_buffer: Vec<Vec<f32>>,
}

impl RubatoResampler {
    /// Create a new rubato resampler
    pub fn new(input_sr: u32, output_sr: u32, channels: usize) -> Result<Self> {
        let mut resampler = Self {
            resampler: None,
            input_sample_rate: input_sr,
            output_sample_rate: output_sr,
            channels,
            input_buffer: vec![Vec::new(); channels],
            output_buffer: vec![Vec::new(); channels],
        };

        resampler.update_resampler()?;
        Ok(resampler)
    }

    /// Update the internal resampler when sample rates change
    fn update_resampler(&mut self) -> Result<()> {
        if self.input_sample_rate == self.output_sample_rate {
            // No resampling needed
            self.resampler = None;
            return Ok(());
        }

        let ratio = self.output_sample_rate as f64 / self.input_sample_rate as f64;

        // Calculate buffer sizes
        let max_input_frames = 1024;
        let max_output_frames = ((max_input_frames as f64) * ratio).ceil() as usize;

        // Create interpolation parameters - FAST settings for testing
        let params = SincInterpolationParameters {
            sinc_len: 32, // Much smaller for speed (was 256)
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 16, // Much smaller for speed (was 256)
            window: WindowFunction::BlackmanHarris2,
        };

        // Create the resampler
        let resampler = SincFixedIn::<f32>::new(
            ratio,
            2.0, // Max ratio change
            params,
            max_input_frames,
            self.channels,
        )?;

        self.resampler = Some(resampler);
        Ok(())
    }

    /// Deinterleave audio samples into separate channel buffers
    fn deinterleave(interleaved: &[Sample], channels: usize) -> Vec<Vec<f32>> {
        let mut channel_buffers = vec![Vec::new(); channels];
        let samples_per_channel = interleaved.len() / channels;

        for (i, &sample) in interleaved.iter().enumerate() {
            let channel = i % channels;
            if channel_buffers[channel].len() < samples_per_channel {
                channel_buffers[channel].push(sample);
            }
        }

        channel_buffers
    }

    /// Interleave separate channel buffers into a single buffer
    fn interleave(channel_buffers: &[Vec<f32>], channels: usize) -> Vec<Sample> {
        let max_samples = channel_buffers
            .iter()
            .map(|buf| buf.len())
            .max()
            .unwrap_or(0);
        let mut interleaved = Vec::with_capacity(max_samples * channels);

        for i in 0..max_samples {
            for channel in 0..channels {
                if let Some(&sample) = channel_buffers[channel].get(i) {
                    interleaved.push(sample);
                } else {
                    interleaved.push(0.0);
                }
            }
        }

        interleaved
    }
}

impl Resampler for RubatoResampler {
    fn process(&mut self, in_buf: &[Sample], out_buf: &mut [Sample], channels: usize) -> usize {
        if self.input_sample_rate == self.output_sample_rate {
            // No resampling needed, just copy
            let copy_len = in_buf.len().min(out_buf.len());
            out_buf[..copy_len].copy_from_slice(&in_buf[..copy_len]);
            return copy_len;
        }

        let Some(ref mut resampler) = self.resampler else {
            return 0;
        };

        // Deinterleave input
        let input_channels = Self::deinterleave(in_buf, channels);
        let input_frames = input_channels[0].len();

        // Rubato SincFixedIn consumes exactly input_frames_next() frames per call.
        // Process in chunks so we don't drop 3/4 of the audio.
        let chunk_in = resampler.input_frames_next();
        let max_out_per_chunk = resampler.output_frames_max() * channels;

        // Ensure per-channel output buffers for rubato
        let out_frames_cap = resampler.output_frames_max();
        for buf in self.output_buffer.iter_mut() {
            if buf.len() < out_frames_cap {
                buf.resize(out_frames_cap, 0.0);
            }
        }

        let mut input_offset = 0_usize;
        let mut output_offset = 0_usize;

        while input_offset + chunk_in <= input_frames {
            let space_left = out_buf.len().saturating_sub(output_offset);
            if space_left < max_out_per_chunk {
                break;
            }

            // Slices for this chunk: one per channel
            let in_slices: Vec<&[f32]> = (0..channels)
                .map(|c| &input_channels[c][input_offset..input_offset + chunk_in])
                .collect();

            let mut out_buffers: Vec<&mut [f32]> = self
                .output_buffer
                .iter_mut()
                .map(|b| &mut b[..out_frames_cap])
                .collect();

            let (n_in, n_out) = match resampler.process_into_buffer(&in_slices, &mut out_buffers, None) {
                Ok(t) => t,
                Err(_) => break,
            };

            // Interleave this chunk into out_buf
            for f in 0..n_out {
                for c in 0..channels {
                    if output_offset < out_buf.len() {
                        out_buf[output_offset] = self.output_buffer[c][f];
                        output_offset += 1;
                    }
                }
            }
            input_offset += n_in;
        }

        // Process remaining input (partial chunk) so we don't drop tail samples
        if input_offset < input_frames {
            let in_slices: Vec<&[f32]> = (0..channels)
                .map(|c| &input_channels[c][input_offset..])
                .collect();
            let mut out_buffers: Vec<&mut [f32]> = self
                .output_buffer
                .iter_mut()
                .map(|b| &mut b[..out_frames_cap])
                .collect();
            let space_left = out_buf.len().saturating_sub(output_offset);
            if space_left >= max_out_per_chunk {
                if let Ok((_n_in, n_out)) = resampler.process_partial_into_buffer(
                    Some(&in_slices),
                    &mut out_buffers,
                    None,
                ) {
                    for f in 0..n_out {
                        for c in 0..channels {
                            if output_offset < out_buf.len() {
                                out_buf[output_offset] = self.output_buffer[c][f];
                                output_offset += 1;
                            }
                        }
                    }
                }
            }
        }

        output_offset
    }

    fn set_rate(&mut self, input_sr: u32, output_sr: u32) {
        self.input_sample_rate = input_sr;
        self.output_sample_rate = output_sr;

        // Update the resampler with new rates
        if let Err(e) = self.update_resampler() {
            eprintln!("Failed to update resampler: {}", e);
        }
    }
}

/// Create a new resampler instance
pub fn create_resampler(
    input_sr: u32,
    output_sr: u32,
    channels: usize,
) -> Result<Box<dyn Resampler>> {
    Ok(Box::new(RubatoResampler::new(
        input_sr, output_sr, channels,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_creation() {
        let resampler = RubatoResampler::new(44100, 48000, 2);
        assert!(resampler.is_ok());
    }

    #[test]
    fn test_resampler_no_resampling() {
        let mut resampler = RubatoResampler::new(44100, 44100, 2).unwrap();

        let input = vec![0.1, 0.2, 0.3, 0.4]; // 2 channels, 2 frames
        let mut output = vec![0.0; 4];

        let processed = resampler.process(&input, &mut output, 2);
        assert_eq!(processed, 4);
        assert_eq!(input, output);
    }

    #[test]
    fn test_resampler_rate_change() {
        let mut resampler = RubatoResampler::new(44100, 48000, 2).unwrap();

        // Change sample rate
        resampler.set_rate(48000, 44100);
        assert_eq!(resampler.input_sample_rate, 48000);
        assert_eq!(resampler.output_sample_rate, 44100);
    }

    #[test]
    fn test_deinterleave() {
        let resampler = RubatoResampler::new(44100, 48000, 2).unwrap();
        let input = vec![0.1, 0.2, 0.3, 0.4]; // L, R, L, R
        let channels = RubatoResampler::deinterleave(&input, 2);

        assert_eq!(channels[0], vec![0.1, 0.3]); // Left channel
        assert_eq!(channels[1], vec![0.2, 0.4]); // Right channel
    }

    #[test]
    fn test_interleave() {
        let resampler = RubatoResampler::new(44100, 48000, 2).unwrap();
        let channels = vec![vec![0.1, 0.3], vec![0.2, 0.4]];
        let interleaved = RubatoResampler::interleave(&channels, 2);

        assert_eq!(interleaved, vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn test_create_resampler_function() {
        let resampler = create_resampler(44100, 48000, 2);
        assert!(resampler.is_ok());
    }
}
