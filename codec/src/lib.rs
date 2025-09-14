//! Audio codec wrapper for rust-dj-engine
//!
//! This crate provides audio decoding capabilities using symphonia.
//! It will be implemented in Sprint 1.

use audio_core::Sample;
use anyhow::Result;

/// Audio decoder (placeholder)
pub struct Decoder {
    // Implementation will be added in Sprint 1
}

impl Decoder {
    /// Create a new decoder
    pub fn new() -> Result<Self> {
        // TODO: Implement in Sprint 1
        Err(anyhow::anyhow!("Codec not yet implemented"))
    }

    /// Read frames from the decoder
    pub fn read_frames(&mut self, _buffer: &mut [Sample]) -> Result<usize> {
        // TODO: Implement in Sprint 1
        Err(anyhow::anyhow!("Codec not yet implemented"))
    }
}
