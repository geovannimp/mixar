//! Omnibus cmd/evt buses for library control and egress.

use library_api::{Kind, Origin};
use omnibus::Filter;
use std::sync::Arc;

/// Shared omnibus type for cmd and evt buses (payload = nested MessagePack body bytes).
pub type LibraryBus = omnibus::Bus<Origin, Kind, Arc<[u8]>>;

/// Host-facing evt subscription (all origins/kinds).
pub type EvtReceiver = omnibus::BusReceiver<Origin, Kind, Arc<[u8]>>;

/// Shared evt handle type for host bridges.
pub type Evt = omnibus::Event<Origin, Kind, Arc<[u8]>>;

/// Create a matched cmd/evt bus pair.
pub fn new_buses() -> (LibraryBus, LibraryBus) {
    (
        LibraryBus::with_capacity(256),
        LibraryBus::with_capacity(512),
    )
}

/// Subscribe to every egress event (UI / bridge).
pub fn subscribe_evt_all(bus: &LibraryBus) -> Result<EvtReceiver, omnibus::OmnibusError> {
    bus.subscribe(Filter::Any, Filter::Any)
}
