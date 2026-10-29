//! The typed cmd/evt pair shared by the engine, the library and the controller.
//!
//! One `EventBus<Event>` carries both directions, so a command and the event it
//! produces travel the same paths and Flutter subscribes once. Capacities are
//! per subscriber: overflow drops the newest event for that subscriber.

use engine_api::Event;
use event_bus::EventBus;

/// Discrete commands: drained every control tick, so a shallow queue is enough.
pub const CMD_CAPACITY: usize = 256;
/// Egress carries high-rate position/levels; headroom keeps discrete events from starving.
pub const EVT_CAPACITY: usize = 2048;

/// The process-wide bus with engine sizing.
pub fn new_event_bus() -> EventBus<Event> {
    EventBus::new(CMD_CAPACITY, EVT_CAPACITY)
}
