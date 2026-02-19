//! Audio resampler for rust-dj-engine
//!
//! This crate provides audio resampling capabilities using rubato 1.0.
//! Uses `InterleavedSlice` from `audioadapter-buffers` for zero-copy
//! adapter-based I/O — no manual deinterleave/interleave needed.

use anyhow::Result;
use audio_core::Sample;
use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Indexing, Resampler as RubatoResamplerTrait,
    SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

/// Resampler trait
pub trait Resampler: Send {
    /// Process audio samples
    fn process(&mut self, in_buf: &[Sample], out_buf: &mut [Sample], channels: usize) -> usize;

    /// Set the sample rate
    fn set_rate(&mut self, input_sr: u32, output_sr: u32);
}

/// Rubato resampler implementation using `rubato::Async` (sinc, fixed-input).
pub struct RubatoResampler {
    resampler: Option<Async<f32>>,
    input_sample_rate: u32,
    output_sample_rate: u32,
    channels: usize,
}

impl RubatoResampler {
    /// Create a new rubato resampler
    pub fn new(input_sr: u32, output_sr: u32, channels: usize) -> Result<Self> {
        let mut resampler = Self {
            resampler: None,
            input_sample_rate: input_sr,
            output_sample_rate: output_sr,
            channels,
        };

        resampler.update_resampler()?;
        Ok(resampler)
    }

    /// Update the internal resampler when sample rates change
    fn update_resampler(&mut self) -> Result<()> {
        if self.input_sample_rate == self.output_sample_rate {
            self.resampler = None;
            return Ok(());
        }

        let ratio = self.output_sample_rate as f64 / self.input_sample_rate as f64;

        let params = SincInterpolationParameters {
            sinc_len: 32,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 16,
            window: WindowFunction::BlackmanHarris2,
        };

        let resampler = Async::<f32>::new_sinc(
            ratio,
            2.0,
            &params,
            1024,
            self.channels,
            FixedAsync::Input,
        )?;

        self.resampler = Some(resampler);
        Ok(())
    }
}

impl Resampler for RubatoResampler {
    fn process(&mut self, in_buf: &[Sample], out_buf: &mut [Sample], channels: usize) -> usize {
        if self.input_sample_rate == self.output_sample_rate {
            let copy_len = in_buf.len().min(out_buf.len());
            out_buf[..copy_len].copy_from_slice(&in_buf[..copy_len]);
            return copy_len;
        }

        let Some(ref mut resampler) = self.resampler else {
            return 0;
        };

        let input_frames = in_buf.len() / channels;
        let output_frames_cap = out_buf.len() / channels;

        let input_adapter = match InterleavedSlice::new(in_buf, channels, input_frames) {
            Ok(a) => a,
            Err(_) => return 0,
        };
        let mut output_adapter =
            match InterleavedSlice::new_mut(out_buf, channels, output_frames_cap) {
                Ok(a) => a,
                Err(_) => return 0,
            };

        let chunk_in = resampler.input_frames_next();
        let mut input_offset = 0usize;
        let mut output_offset = 0usize;

        while input_offset + chunk_in <= input_frames && output_offset < output_frames_cap {
            let indexing = Indexing {
                input_offset,
                output_offset,
                active_channels_mask: None,
                partial_len: None,
            };
            match resampler.process_into_buffer(
                &input_adapter,
                &mut output_adapter,
                Some(&indexing),
            ) {
                Ok((n_in, n_out)) => {
                    input_offset += n_in;
                    output_offset += n_out;
                }
                Err(_) => break,
            }
        }

        // Handle remaining input (partial chunk)
        if input_offset < input_frames && output_offset < output_frames_cap {
            let remaining = input_frames - input_offset;
            let indexing = Indexing {
                input_offset,
                output_offset,
                active_channels_mask: None,
                partial_len: Some(remaining),
            };
            if let Ok((_, n_out)) = resampler.process_into_buffer(
                &input_adapter,
                &mut output_adapter,
                Some(&indexing),
            ) {
                output_offset += n_out;
            }
        }

        output_offset * channels
    }

    fn set_rate(&mut self, input_sr: u32, output_sr: u32) {
        self.input_sample_rate = input_sr;
        self.output_sample_rate = output_sr;

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

        resampler.set_rate(48000, 44100);
        assert_eq!(resampler.input_sample_rate, 48000);
        assert_eq!(resampler.output_sample_rate, 44100);
    }

    #[test]
    fn test_resampler_44100_to_48000() {
        let mut resampler = RubatoResampler::new(44100, 48000, 2).unwrap();

        // Generate a reasonable chunk of silence (1024 frames, stereo interleaved)
        let input = vec![0.0f32; 1024 * 2];
        let mut output = vec![0.0f32; 2048 * 2];

        let processed = resampler.process(&input, &mut output, 2);
        assert!(processed > 0, "Resampler should produce output samples");
    }

    #[test]
    fn test_create_resampler_function() {
        let resampler = create_resampler(44100, 48000, 2);
        assert!(resampler.is_ok());
    }
}
