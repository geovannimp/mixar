//! Omnibus cmd/evt buses for engine control and egress.

use engine_api::{Kind, Origin};
use std::sync::Arc;

/// Shared omnibus type for cmd and evt buses (payload = nested postcard body bytes).
pub type EngineBus = omnibus::Bus<Origin, Kind, Arc<[u8]>>;

/// Create a matched cmd/evt bus pair with room for high-rate evt coalescing (Task 3).
pub fn new_buses() -> (EngineBus, EngineBus) {
    (EngineBus::with_capacity(256), EngineBus::with_capacity(256))
}
