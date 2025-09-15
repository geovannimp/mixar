//! Audio codec wrapper for rust-dj-engine
//!
//! This crate provides audio decoding capabilities using symphonia.

use anyhow::Result;
use audio_core::Sample;
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Audio decoder for various formats
pub struct AudioDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
}

/// Audio metadata
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<u64>, // Duration in samples
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: u16,
}

impl AudioDecoder {
    /// Create a new decoder from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(&path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a probe hint using the file's extension.
        let mut hint = Hint::new();
        if let Some(extension) = path.as_ref().extension().and_then(|ext| ext.to_str()) {
            hint.with_extension(extension);
        }

        // Use the default options for metadata and format readers.
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        // Probe the media source.
        let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;

        // Get the instantiated format reader.
        let format = probed.format;

        // Find the first audio track with a known (decodeable) codec.
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow::anyhow!("No supported audio tracks"))?;

        let track_id = track.id;

        // Create a decoder for the track.
        let dec_opts: DecoderOptions = Default::default();
        let decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

        // Get the audio parameters
        let codec_params = &track.codec_params;
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params.channels.map(|c| c.count()).unwrap_or(2) as u16;

        Ok(Self {
            format,
            decoder,
            track_id,
            sample_rate,
            channels,
        })
    }

    /// Read frames from the decoder
    pub fn read_frames(&mut self, buffer: &mut [Sample]) -> Result<usize> {
        let mut total_samples = 0;
        let buffer_len = buffer.len();

        while total_samples < buffer_len {
            // Read the next packet from the media source.
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    // The track list has changed and the user must seek to a valid position.
                    return Ok(total_samples);
                }
                Err(symphonia::core::errors::Error::IoError(_)) => {
                    // End of stream
                    return Ok(total_samples);
                }
                Err(e) => return Err(e.into()),
            };

            // If the packet does not belong to the selected track, skip it.
            if packet.track_id() != self.track_id {
                continue;
            }

            // Decode the packet into audio samples.
            let audio_buf = match self.decoder.decode(&packet) {
                Ok(audio_buf) => audio_buf,
                Err(symphonia::core::errors::Error::IoError(_)) => {
                    // End of stream
                    return Ok(total_samples);
                }
                Err(e) => return Err(e.into()),
            };

            // Convert the audio buffer to f32 samples
            let samples = Self::audio_buffer_to_samples(&audio_buf);
            let samples_to_copy = (buffer_len - total_samples).min(samples.len());

            buffer[total_samples..total_samples + samples_to_copy]
                .copy_from_slice(&samples[..samples_to_copy]);

            total_samples += samples_to_copy;

            // If we've filled the buffer, we're done
            if total_samples >= buffer_len {
                break;
            }
        }

        Ok(total_samples)
    }

    /// Get metadata for the audio file
    pub fn metadata(&mut self) -> Result<AudioMetadata> {
        // Simplified metadata extraction for now
        // TODO: Implement proper metadata reading when symphonia API is stable

        // Get duration from the first track
        let mut duration = None;
        if let Some(track) = self.format.tracks().first() {
            if let Some(time_base) = track.codec_params.time_base {
                if let Some(n_frames) = track.codec_params.n_frames {
                    duration = Some(n_frames);
                }
            }
        }

        Ok(AudioMetadata {
            title: None,
            artist: None,
            album: None,
            duration,
            sample_rate: self.sample_rate,
            channels: self.channels,
            bit_depth: 32, // We always convert to f32
        })
    }

    /// Get the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the number of channels
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Convert symphonia audio buffer to f32 samples
    fn audio_buffer_to_samples(audio_buf: &AudioBufferRef) -> Vec<Sample> {
        match audio_buf {
            AudioBufferRef::F32(buf) => buf.chan(0).to_vec(),
            AudioBufferRef::U8(buf) => buf
                .chan(0)
                .iter()
                .map(|&sample| (sample as f32 - 128.0) / 128.0)
                .collect(),
            AudioBufferRef::U16(buf) => buf
                .chan(0)
                .iter()
                .map(|&sample| sample as f32 / 32768.0)
                .collect(),
            AudioBufferRef::U24(buf) => buf
                .chan(0)
                .iter()
                .map(|&sample| sample.inner() as f32 / 8388608.0)
                .collect(),
            AudioBufferRef::U32(buf) => buf
                .chan(0)
                .iter()
                .map(|&sample| sample as f32 / 2147483648.0)
                .collect(),
            AudioBufferRef::S8(buf) => buf
                .chan(0)
                .iter()
                .map(|&sample| sample as f32 / 128.0)
                .collect(),
            AudioBufferRef::S16(buf) => buf
                .chan(0)
                .iter()
                .map(|&sample| sample as f32 / 32768.0)
                .collect(),
            AudioBufferRef::S24(buf) => buf
                .chan(0)
                .iter()
                .map(|&sample| sample.inner() as f32 / 8388608.0)
                .collect(),
            AudioBufferRef::S32(buf) => buf
                .chan(0)
                .iter()
                .map(|&sample| sample as f32 / 2147483648.0)
                .collect(),
            AudioBufferRef::F64(buf) => buf.chan(0).iter().map(|&sample| sample as f32).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_wav_file() -> Result<NamedTempFile> {
        // Create a simple WAV file for testing
        let mut file = NamedTempFile::new()?;

        // WAV header (simplified)
        let header = [
            0x52, 0x49, 0x46, 0x46, // "RIFF"
            0x24, 0x00, 0x00, 0x00, // File size - 8
            0x57, 0x41, 0x56, 0x45, // "WAVE"
            0x66, 0x6D, 0x74, 0x20, // "fmt "
            0x10, 0x00, 0x00, 0x00, // Format chunk size
            0x01, 0x00, // Audio format (PCM)
            0x02, 0x00, // Number of channels
            0x44, 0xAC, 0x00, 0x00, // Sample rate (44100)
            0x10, 0xB1, 0x02, 0x00, // Byte rate
            0x04, 0x00, // Block align
            0x10, 0x00, // Bits per sample
            0x64, 0x61, 0x74, 0x61, // "data"
            0x00, 0x00, 0x00, 0x00, // Data size
        ];

        file.write_all(&header)?;
        file.flush()?;
        Ok(file)
    }

    #[test]
    fn test_decoder_creation() {
        // This test might fail if no test file is available, which is ok
        // In a real implementation, we'd create a proper test file
        let result = AudioDecoder::from_file("nonexistent.wav");
        assert!(result.is_err());
    }

    #[test]
    fn test_metadata_structure() {
        let metadata = AudioMetadata {
            title: Some("Test Song".to_string()),
            artist: Some("Test Artist".to_string()),
            album: Some("Test Album".to_string()),
            duration: Some(44100),
            sample_rate: 44100,
            channels: 2,
            bit_depth: 32,
        };

        assert_eq!(metadata.title, Some("Test Song".to_string()));
        assert_eq!(metadata.artist, Some("Test Artist".to_string()));
        assert_eq!(metadata.sample_rate, 44100);
        assert_eq!(metadata.channels, 2);
    }
}
