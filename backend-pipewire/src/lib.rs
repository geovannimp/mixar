//! PipeWire backend implementation
//!
//! This backend uses PipeWire for low-latency audio on Linux.
//! It will be implemented in Sprint 4.

use audio_core::{AudioBackend, AudioCallback, AudioStream, DeviceId, DeviceInfo, StreamParams};
use anyhow::Result;

/// PipeWire backend (placeholder)
#[derive(Debug)]
pub struct PipewireBackend {
    // Implementation will be added in Sprint 4
}

impl PipewireBackend {
    /// Create a new PipeWire backend
    pub fn new() -> Result<Self> {
        // TODO: Implement in Sprint 4
        Err(anyhow::anyhow!("PipeWire backend not yet implemented"))
    }
}

impl AudioBackend for PipewireBackend {
    fn name(&self) -> &'static str {
        "pipewire"
    }

    fn list_output_devices(&self) -> Result<Vec<DeviceInfo>> {
        // TODO: Implement in Sprint 4
        Err(anyhow::anyhow!("PipeWire backend not yet implemented"))
    }

    fn default_output_device(&self) -> Result<DeviceInfo> {
        // TODO: Implement in Sprint 4
        Err(anyhow::anyhow!("PipeWire backend not yet implemented"))
    }

    fn open_output_stream(
        &mut self,
        _device: &DeviceId,
        _params: &StreamParams,
        _callback: Box<dyn AudioCallback>,
    ) -> Result<Box<dyn AudioStream>> {
        // TODO: Implement in Sprint 4
        Err(anyhow::anyhow!("PipeWire backend not yet implemented"))
    }
}
