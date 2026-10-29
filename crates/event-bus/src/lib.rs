//! Domain-free typed event bus with per-subscriber merge policies.
//!
//! `omnibus` fans an event out into one bounded channel per subscriber and
//! drops on overflow. That leaves two problems Mixar had solved in the host
//! instead: the payload had to be encoded to bytes and decoded per consumer,
//! and every consumer re-implemented its own "keep only the newest of these"
//! filter. This crate owns both.
//!
//! # Layers
//!
//! | Piece | Role |
//! |-------|------|
//! | [`EventBus`] | the cmd/evt pair; typed payloads, no wire format |
//! | [`BusMessage`] | the closed event enum travelling on it |
//! | [`EventSpec`] | one payload's topic, policy, key and fold |
//! | [`Policy`] | how a pending slot folds a colliding event |
//! | [`Subscriber`] | per-subscription pending queue applying that policy |
//!
//! # Merging
//!
//! Publishers keep publish order and never block; merging happens in the
//! subscriber, on the events that are still undelivered when it next reads.
//! [`EventKey`] is `{ entity, topic, part }`, so two setters for different
//! slots, bands or params own different pending slots and are never folded into
//! each other.
//!
//! `Toggle` is the one policy that reasons about adjacency, and it does so only
//! on an unfiltered subscription: on a topic-filtered one the events that would
//! separate two toggles never arrive, so a pair looks adjacent when it is not.
//!
//! # Example
//!
//! ```
//! use event_bus::bus_events;
//! use event_bus::Entity;
//! use event_bus::EventBus;
//! use event_bus::Part;
//!
//! bus_events! {
//!     Volume: Update, Entity::Deck(*deck), Part::Whole, { deck: u16, volume: f32 };
//!     Jog:    Delta,  Entity::Deck(*deck), Part::Whole, { deck: u16, delta: i32 }, fold: delta;
//! }
//!
//! fn main() {
//!     let bus = EventBus::<Event>::new(64, 64);
//!     let mut sub = bus.subscribe_cmd().expect("first subscriber");
//!     bus.publish_cmd(Event::Volume(Volume { deck: 1, volume: 0.2 })).expect("publish");
//!     bus.publish_cmd(Event::Volume(Volume { deck: 1, volume: 0.9 })).expect("publish");
//!     bus.publish_cmd(Event::Jog(Jog { deck: 1, delta: 3 })).expect("publish");
//!     bus.publish_cmd(Event::Jog(Jog { deck: 1, delta: 4 })).expect("publish");
//!
//!     // The two volumes collapse to the newest, the two jogs sum.
//!     assert_eq!(sub.try_recv(), Some(Event::Volume(Volume { deck: 1, volume: 0.9 })));
//!     assert_eq!(sub.try_recv(), Some(Event::Jog(Jog { deck: 1, delta: 7 })));
//!     assert_eq!(sub.try_recv(), None);
//! }
//! ```

mod bus;
mod entity;
mod macros;
mod message;
mod policy;
mod spec;
mod subscriber;

/// Errors returned by the underlying bus: a poisoned lock or a closed channel.
pub use omnibus::OmnibusError as BusError;

pub use bus::EventBus;
pub use entity::Entity;
pub use entity::EventKey;
pub use entity::Part;
pub use message::BusMessage;
pub use policy::Policy;
pub use spec::EventSpec;
pub use subscriber::Subscriber;
