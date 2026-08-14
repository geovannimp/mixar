//! Omnibus cmd/evt buses for engine control and egress.

use anyhow::Result;
use engine_api::{encode_evt_body, EvtBody, Kind, Origin};
use omnibus::{Event, Filter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Shared omnibus type for cmd and evt buses (payload = nested MessagePack body bytes).
pub type EngineBus = omnibus::Bus<Origin, Kind, Arc<[u8]>>;

/// Host-facing evt subscription (all origins/kinds). Keeps omnibus types out of Tauri.
pub type EvtReceiver = omnibus::BusReceiver<Origin, Kind, Arc<[u8]>>;

/// Shared evt handle type for host bridges that drain/coalesce without depending on omnibus.
pub type Evt = omnibus::Event<Origin, Kind, Arc<[u8]>>;

/// Host-owned cloneable cmd/evt bus bundle + shared revision.
///
/// Hosts create this once, inject clones into [`crate::Engine`] via `set_buses`,
/// and keep a handle so controller/UI can `publish_evt` without locking the engine.
#[derive(Clone)]
pub struct EngineBuses {
    cmd: EngineBus,
    evt: EngineBus,
    revision: Arc<AtomicU64>,
}

impl EngineBuses {
    /// Create a matched cmd/evt pair with shared revision.
    pub fn new() -> Self {
        let (cmd, evt) = new_buses();
        Self {
            cmd,
            evt,
            revision: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Fire-and-forget publish of a command (nested `CmdBody` bytes in payload).
    pub fn publish_cmd(&self, origin: Origin, kind: Kind, body: impl AsRef<[u8]>) -> Result<()> {
        self.cmd
            .publish(Event::new(origin, kind, Arc::from(body.as_ref())))?;
        Ok(())
    }

    /// Encode `EvtBody`, bump revision, publish on the evt bus.
    pub fn publish_evt(&self, origin: Origin, kind: Kind, body: EvtBody) -> Result<()> {
        let bytes = encode_evt_body(&body)?;
        self.revision.fetch_add(1, Ordering::Relaxed);
        self.evt
            .publish(Event::new(origin, kind, Arc::from(bytes)))?;
        Ok(())
    }

    /// Subscribe to every egress event (UI / MIDI / bridge).
    pub fn subscribe_evt_all(&self) -> Result<EvtReceiver> {
        Ok(self.evt.subscribe(Filter::Any, Filter::Any)?)
    }

    /// Monotonic revision bumped when discrete engine state changes.
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    /// Clone handle to the command ingress bus.
    pub fn cmd_bus(&self) -> EngineBus {
        self.cmd.clone()
    }

    /// Clone handle to the event egress bus.
    pub fn evt_bus(&self) -> EngineBus {
        self.evt.clone()
    }

    pub(crate) fn revision_arc(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.revision)
    }
}

impl Default for EngineBuses {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a matched cmd/evt bus pair. Evt capacity is large so high-rate
/// position/levels do not starve discrete Updated/Status when a host is slow.
pub fn new_buses() -> (EngineBus, EngineBus) {
    (
        EngineBus::with_capacity(256),
        EngineBus::with_capacity(2048),
    )
}
