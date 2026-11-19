//! The typed cmd/evt pair shared by the engine, the library and the controller.
//!
//! One `EventBus<Command, Event>` carries both directions: a command moves into
//! the one worker inbox that owns its target, an event is shared with every
//! matching observer. Both lanes are unbounded, so a burst is queued instead of
//! dropped — a stalled consumer costs memory, not messages.

use bus_api::Command;
use bus_api::Event;
use event_bus::EventBus;

/// The process-wide bus.
#[must_use]
pub fn new_event_bus() -> EventBus<Command, Event> {
    EventBus::new()
}
