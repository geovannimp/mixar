//! Miniaudio backend implementation
//!
//! This backend uses the miniaudio library for cross-platform audio I/O.
//! It will be implemented in Sprint 1.

use audio_core::{AudioBackend, AudioCallback, AudioStream, DeviceId, DeviceInfo, StreamParams};
use anyhow::Result;

/// Miniaudio backend (placeholder)
#[derive(Debug)]
pub struct MiniaudioBackend {
    // Implementation will be added in Sprint 1
}

impl MiniaudioBackend {
    /// Create a new miniaudio backend
    pub fn new() -> Result<Self> {
        // TODO: Implement in Sprint 1
        Err(anyhow::anyhow!("Miniaudio backend not yet implemented"))
    }
}

impl AudioBackend for MiniaudioBackend {
    fn name(&self) -> &'static str {
        "miniaudio"
    }

    fn list_output_devices(&self) -> Result<Vec<DeviceInfo>> {
        // TODO: Implement in Sprint 1
        Err(anyhow::anyhow!("Miniaudio backend not yet implemented"))
    }

    fn default_output_device(&self) -> Result<DeviceInfo> {
        // TODO: Implement in Sprint 1
        Err(anyhow::anyhow!("Miniaudio backend not yet implemented"))
    }

    fn open_output_stream(
        &mut self,
        _device: &DeviceId,
        _params: &StreamParams,
        _callback: Box<dyn AudioCallback>,
    ) -> Result<Box<dyn AudioStream>> {
        // TODO: Implement in Sprint 1
        Err(anyhow::anyhow!("Miniaudio backend not yet implemented"))
    }
}
