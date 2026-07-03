//! Audio resampler for rust-dj-engine
//!
//! This crate provides audio resampling capabilities using rubato 3 (FFT synchronous resampler).
//! Uses `InterleavedSlice` from `audioadapter-buffers` for zero-copy
//! adapter-based I/O — no manual deinterleave/interleave needed.

use anyhow::Result;
use audio_core::Sample;
use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Fft, FixedSync, Indexing, Resampler as RubatoResamplerTrait,
};

/// Resampler trait
pub trait Resampler: Send {
    /// Process interleaved samples.
    ///
    /// Returns `(output_samples_written, input_frames_consumed)`.
    fn process(
        &mut self,
        in_buf: &[Sample],
        out_buf: &mut [Sample],
        channels: usize,
    ) -> (usize, usize);

    /// Set the sample rate
    fn set_rate(&mut self, input_sr: u32, output_sr: u32);

    /// Input frames required for the next resampling step.
    fn input_frames_next(&self) -> usize;

    /// Output frames produced per resampling step.
    fn output_frames_next(&self) -> usize;

    /// Rebuild the resampler so each step produces `frames` output samples.
    fn set_output_chunk_frames(&mut self, frames: usize);

    /// Clear internal filter/history state (e.g. after seek or stop).
    fn reset(&mut self);
}

/// Rubato FFT resampler using fixed **output** chunks so input consumption tracks playback.
pub struct RubatoResampler {
    resampler: Option<Fft<f32>>,
    input_sample_rate: u32,
    output_sample_rate: u32,
    channels: usize,
    output_chunk_frames: usize,
}

impl RubatoResampler {
    /// Create a new rubato resampler.
    ///
    /// `output_chunk_frames` must match the audio callback / engine buffer size so
    /// rubato input consumption tracks real-time playback (see CPAL buffer size docs).
    pub fn new(
        input_sr: u32,
        output_sr: u32,
        channels: usize,
        output_chunk_frames: usize,
    ) -> Result<Self> {
        let mut resampler = Self {
            resampler: None,
            input_sample_rate: input_sr,
            output_sample_rate: output_sr,
            channels,
            output_chunk_frames: output_chunk_frames.max(1),
        };

        resampler.update_resampler()?;
        Ok(resampler)
    }

    fn update_resampler(&mut self) -> Result<()> {
        if self.input_sample_rate == self.output_sample_rate {
            self.resampler = None;
            return Ok(());
        }

        // sub_chunks=2 → ~256-frame FFT blocks at 512-frame output chunks (rubato docs recommend ~100–1000).
        let resampler = Fft::<f32>::new(
            self.input_sample_rate as usize,
            self.output_sample_rate as usize,
            self.output_chunk_frames,
            2,
            self.channels,
            FixedSync::Output,
        )?;

        self.resampler = Some(resampler);
        Ok(())
    }
}

impl Resampler for RubatoResampler {
    fn process(
        &mut self,
        in_buf: &[Sample],
        out_buf: &mut [Sample],
        channels: usize,
    ) -> (usize, usize) {
        if self.input_sample_rate == self.output_sample_rate {
            let copy_len = in_buf.len().min(out_buf.len());
            out_buf[..copy_len].copy_from_slice(&in_buf[..copy_len]);
            return (copy_len, copy_len / channels);
        }

        let Some(ref mut resampler) = self.resampler else {
            return (0, 0);
        };

        let input_frames = in_buf.len() / channels;
        let output_frames_cap = out_buf.len() / channels;

        let input_adapter = match InterleavedSlice::new(in_buf, channels, input_frames) {
            Ok(a) => a,
            Err(_) => return (0, 0),
        };
        let mut output_adapter =
            match InterleavedSlice::new_mut(out_buf, channels, output_frames_cap) {
                Ok(a) => a,
                Err(_) => return (0, 0),
            };

        let mut input_offset = 0usize;
        let mut output_offset = 0usize;

        while output_offset < output_frames_cap {
            let need_in = resampler.input_frames_next();
            let need_out = resampler
                .output_frames_next()
                .min(output_frames_cap - output_offset);

            if input_offset + need_in > input_frames {
                let remaining = input_frames - input_offset;
                if remaining == 0 {
                    break;
                }
                let indexing = Indexing {
                    input_offset,
                    output_offset,
                    active_channels_mask: None,
                    partial_len: Some(remaining),
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
                break;
            }

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
                    if n_in == 0 && n_out == 0 {
                        break;
                    }
                    input_offset += n_in;
                    output_offset += n_out;
                    if n_out < need_out {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        (output_offset * channels, input_offset)
    }

    fn set_rate(&mut self, input_sr: u32, output_sr: u32) {
        self.input_sample_rate = input_sr;
        self.output_sample_rate = output_sr;

        if let Err(e) = self.update_resampler() {
            eprintln!("Failed to update resampler: {}", e);
        }
    }

    fn input_frames_next(&self) -> usize {
        self.resampler
            .as_ref()
            .map(|r| r.input_frames_next())
            .unwrap_or(0)
    }

    fn output_frames_next(&self) -> usize {
        self.resampler
            .as_ref()
            .map(|r| r.output_frames_next())
            .unwrap_or(self.output_chunk_frames)
    }

    fn set_output_chunk_frames(&mut self, frames: usize) {
        let frames = frames.max(1);
        if frames == self.output_chunk_frames {
            return;
        }
        self.output_chunk_frames = frames;
        if let Err(e) = self.update_resampler() {
            eprintln!("Failed to resize resampler chunk: {}", e);
        }
    }

    fn reset(&mut self) {
        if let Err(e) = self.update_resampler() {
            eprintln!("Failed to reset resampler: {}", e);
        }
    }
}

/// Create a new resampler instance sized for one engine callback.
pub fn create_resampler(
    input_sr: u32,
    output_sr: u32,
    channels: usize,
    output_chunk_frames: usize,
) -> Result<Box<dyn Resampler>> {
    Ok(Box::new(RubatoResampler::new(
        input_sr,
        output_sr,
        channels,
        output_chunk_frames,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_OUTPUT_CHUNK_FRAMES: usize = 512;

    #[test]
    fn test_resampler_creation() {
        let resampler = RubatoResampler::new(44100, 48000, 2, DEFAULT_OUTPUT_CHUNK_FRAMES);
        assert!(resampler.is_ok());
    }

    #[test]
    fn test_resampler_no_resampling() {
        let mut resampler =
            RubatoResampler::new(44100, 44100, 2, DEFAULT_OUTPUT_CHUNK_FRAMES).unwrap();

        let input = vec![0.1, 0.2, 0.3, 0.4];
        let mut output = vec![0.0; 4];

        let (processed, input_frames) = resampler.process(&input, &mut output, 2);
        assert_eq!(processed, 4);
        assert_eq!(input_frames, 2);
        assert_eq!(input, output);
    }

    #[test]
    fn test_fft_stream_consumption_over_many_callbacks() {
        let mut resampler = RubatoResampler::new(44100, 48000, 2, 512).unwrap();
        let mut total_in = 0usize;
        let mut total_out = 0usize;
        let callbacks = 480000 / 512;
        let source = vec![0.5f32; 600_000 * 2];
        let mut src_pos = 0usize;

        for _ in 0..callbacks {
            let need_in = resampler.input_frames_next();
            let step_out = resampler.output_frames_next();
            let chunk = &source[src_pos..src_pos + need_in * 2];
            let mut out = vec![0.0f32; step_out * 2];
            let (out_samples, in_frames) = resampler.process(chunk, &mut out, 2);
            total_in += in_frames;
            total_out += out_samples / 2;
            src_pos += in_frames * 2;
        }

        let expected_in = (total_out as f64 * 44100.0 / 48000.0) as usize;
        let ratio = total_in as f64 / expected_in as f64;
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "total_in={total_in}, expected~{expected_in}, ratio={ratio:.4}"
        );
    }

    #[test]
    fn test_output_mode_512_frames() {
        let resampler = RubatoResampler::new(44100, 48000, 2, DEFAULT_OUTPUT_CHUNK_FRAMES).unwrap();
        assert_eq!(resampler.output_frames_next(), 512);
        let need_in = resampler.input_frames_next();
        assert!(need_in > 0);
        // FFT blocks align to rate-ratio multiples (147 in / 160 out for 44100→48000).
        assert_eq!(need_in % 147, 0, "input chunk should align to FFT block size");
    }

    #[test]
    fn test_512_output_consumes_proportional_input() {
        let mut resampler =
            RubatoResampler::new(44100, 48000, 2, DEFAULT_OUTPUT_CHUNK_FRAMES).unwrap();
        let mut total_in = 0usize;
        let mut total_out = 0usize;
        let source = vec![0.5f32; 600_000 * 2];
        let mut src_pos = 0usize;

        for _ in 0..100 {
            let need_in = resampler.input_frames_next();
            let step_out = resampler.output_frames_next();
            let chunk = &source[src_pos..src_pos + need_in * 2];
            let mut out = vec![0.0f32; step_out * 2];
            let (out_samples, in_frames) = resampler.process(chunk, &mut out, 2);
            total_in += in_frames;
            total_out += out_samples / 2;
            src_pos += in_frames * 2;
        }

        let expected_in = (total_out as f64 * 44100.0 / 48000.0) as usize;
        let ratio = total_in as f64 / expected_in as f64;
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "total_in={total_in}, expected~{expected_in}, ratio={ratio:.4}"
        );
    }

    #[test]
    fn test_create_resampler_function() {
        let resampler = create_resampler(44100, 48000, 2, DEFAULT_OUTPUT_CHUNK_FRAMES);
        assert!(resampler.is_ok());
    }

    #[test]
    fn test_480_output_consumes_proportional_input() {
        let mut resampler =
            RubatoResampler::new(44100, 48000, 2, DEFAULT_OUTPUT_CHUNK_FRAMES).unwrap();
        resampler.set_output_chunk_frames(480);
        assert_eq!(resampler.output_frames_next(), 480);

        let mut total_in = 0usize;
        let mut total_out = 0usize;
        let source = vec![0.5f32; 600_000 * 2];
        let mut src_pos = 0usize;

        for _ in 0..100 {
            let need_in = resampler.input_frames_next();
            let step_out = resampler.output_frames_next();
            let chunk = &source[src_pos..src_pos + need_in * 2];
            let mut out = vec![0.0f32; step_out * 2];
            let (out_samples, in_frames) = resampler.process(chunk, &mut out, 2);
            total_in += in_frames;
            total_out += out_samples / 2;
            src_pos += in_frames * 2;
        }

        let expected_in = (total_out as f64 * 44100.0 / 48000.0) as usize;
        let ratio = total_in as f64 / expected_in as f64;
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "total_in={total_in}, expected~{expected_in}, ratio={ratio:.4}"
        );
    }
}
