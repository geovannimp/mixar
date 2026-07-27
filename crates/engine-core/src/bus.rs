//! Omnibus cmd/evt buses for engine control and egress.

use engine_api::{Kind, Origin};
use omnibus::Filter;
use std::sync::Arc;

/// Shared omnibus type for cmd and evt buses (payload = nested MessagePack body bytes).
pub type EngineBus = omnibus::Bus<Origin, Kind, Arc<[u8]>>;

/// Host-facing evt subscription (all origins/kinds). Keeps omnibus types out of Tauri.
pub type EvtReceiver = omnibus::BusReceiver<Origin, Kind, Arc<[u8]>>;

/// Shared evt handle type for host bridges that drain/coalesce without depending on omnibus.
pub type Evt = omnibus::Event<Origin, Kind, Arc<[u8]>>;

/// Create a matched cmd/evt bus pair. Evt capacity is large so high-rate
/// position/levels do not starve discrete Updated/Status when a host is slow.
pub fn new_buses() -> (EngineBus, EngineBus) {
    (
        EngineBus::with_capacity(256),
        EngineBus::with_capacity(2048),
    )
}

/// Subscribe to every egress event (UI / MIDI / bridge).
pub fn subscribe_evt_all(bus: &EngineBus) -> Result<EvtReceiver, omnibus::OmnibusError> {
    bus.subscribe(Filter::Any, Filter::Any)
}
