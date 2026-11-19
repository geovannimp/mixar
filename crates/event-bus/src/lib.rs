//! Owned typed command/event delivery: one inbox per command target, one queue
//! per event observer, and no silent loss.
//!
//! # Why this exists
//!
//! The bus it replaces fanned out through bounded per-subscriber channels and
//! discarded overflow: a publish could succeed while the message was dropped,
//! and release or persistence commands were as droppable as a telemetry tick.
//! Subscriber-side "keep the newest" folding then ran *after* admission, so it
//! could not prevent that overflow, and it could reorder a sequence
//! (`Play, Pause, Play` collapsed to `Play, Pause`).
//!
//! This crate owns routing and lifecycle over [`crossbeam_channel::unbounded`]
//! queues and keeps the algebra where it belongs: nowhere. Every accepted
//! message is delivered, in order, to every registered receiver.
//!
//! # Layers
//!
//! | Piece | Role |
//! |-------|------|
//! | [`EventBus`] | the cmd/evt pair; one mutex per lane |
//! | [`Message`] | what a lane can route: a topic per message |
//! | [`Subscriber`] | an owned queue with no senders and no bus |
//! | [`BusDiagnostics`] | queue depths, labels and closedness |
//!
//! # Guarantees, precisely
//!
//! - Every command is either moved into its owner's queue or returned to the
//!   caller in a [`CommandPublishError`]; nothing is dropped or retried here.
//! - Every event reaches every matching observer that is still registered, and
//!   [`PublishReport`] says how many matched, were delivered and were stale.
//! - Within a lane, the mutex defines one enqueue order: concurrent publishers
//!   cannot interleave, so observers see the same sequence.
//! - Queues are unbounded, so publication never blocks on a full queue and a
//!   stalled consumer costs memory. [`EventBus::diagnostics`] shows the depth;
//!   it does not cap it, and allocation failure is not recoverable.
//! - No replay, no persistence, no processing acknowledgement. An enqueue is not
//!   a completion, and a message consumed before fan-out finishes is simply
//!   early.
//!
//! ```
//! use event_bus::EventBus;
//! use event_bus::Message;
//!
//! #[derive(Clone, Debug, PartialEq)]
//! struct Volume {
//!     deck: u16,
//!     volume: f32,
//! }
//!
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
//! enum Target {
//!     Engine,
//! }
//!
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
//! enum Topic {
//!     Levels,
//! }
//!
//! #[derive(Debug, PartialEq)]
//! enum Cmd {
//!     Volume(Volume),
//! }
//!
//! #[derive(Debug, PartialEq)]
//! enum Evt {
//!     Levels(Volume),
//! }
//!
//! impl Message for Cmd {
//!     type Topic = Target;
//!     fn topic(&self) -> Target {
//!         Target::Engine
//!     }
//! }
//!
//! impl Message for Evt {
//!     type Topic = Topic;
//!     fn topic(&self) -> Topic {
//!         Topic::Levels
//!     }
//! }
//!
//! let bus = EventBus::<Cmd, Evt>::new();
//! let mut engine = bus.subscribe_cmd(Target::Engine, "engine").expect("engine inbox");
//! let mut ui = bus
//!     .subscribe_evt_topics("ui", &[Topic::Levels])
//!     .expect("ui observer");
//!
//! let command = Cmd::Volume(Volume { deck: 1, volume: 0.4 });
//! bus.publish_cmd(command).expect("owner registered");
//! assert_eq!(
//!     engine.try_recv(),
//!     Ok(Some(Cmd::Volume(Volume { deck: 1, volume: 0.4 })))
//! );
//!
//! let report = bus
//!     .publish_evt(Evt::Levels(Volume { deck: 1, volume: 0.4 }))
//!     .expect("publish");
//! assert_eq!((report.matched, report.delivered), (1, 1));
//! assert!(ui.try_recv().expect("open lane").is_some());
//! assert_eq!(ui.try_recv(), Ok(None));
//! ```

mod bus;
mod channel;
mod message;
mod subscriber;

use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

pub use bus::BusDiagnostics;
pub use bus::EventBus;
pub use channel::LaneDiagnostics;
pub use channel::SubscriptionDiagnostics;
pub use message::Message;
pub use subscriber::Subscriber;

/// Why an operation on a lane was refused.
///
/// Every variant is a fact about the lane, not about the payload, so a caller
/// can act on it without inspecting the message it was trying to send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusError {
    /// The lane disconnected: it was closed, or its last bus clone was dropped.
    Closed,
    /// A thread panicked while holding the lane lock; the lane is fail-closed.
    Poisoned,
    /// No live subscriber is registered for that command target.
    NoCommandHandler,
    /// A live subscriber already owns that command target.
    AlreadySubscribed,
    /// An event topic filter was empty, which could never deliver anything.
    EmptyFilter,
    /// The lane's registration ids are exhausted (`u64` overflow).
    IdExhausted,
}

impl BusError {
    /// The message this error reports, without payload details.
    const fn text(self) -> &'static str {
        match self {
            Self::Closed => "event bus is closed",
            Self::Poisoned => "event bus lane is poisoned",
            Self::NoCommandHandler => "no command handler is registered",
            Self::AlreadySubscribed => "command target already has a handler",
            Self::EmptyFilter => "event topic filter must not be empty",
            Self::IdExhausted => "event bus subscription ids exhausted",
        }
    }
}

impl Display for BusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.text())
    }
}

impl Error for BusError {}

/// A refused command, with the command handed back unsent.
///
/// `Debug`/`Display`/`Error` never format the payload, so they hold with no
/// bound on `C` — a command type need not be `Debug` to report a failure.
pub struct CommandPublishError<C> {
    /// Why the lane refused it.
    pub reason: BusError,
    /// The command the bus never accepted.
    pub command: C,
}

impl<C> fmt::Debug for CommandPublishError<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandPublishError")
            .field("reason", &self.reason)
            .field("command", &"<unsent>")
            .finish()
    }
}

impl<C> Display for CommandPublishError<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "command not published: {}", self.reason)
    }
}

impl<C> Error for CommandPublishError<C> {}

/// What one event publication reached.
///
/// `delivered` counts enqueues, not processing: an observer may consume before
/// fan-out finishes, and a queue depth is not a completion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PublishReport {
    /// Matching registrations inspected.
    pub matched: usize,
    /// Matching registrations the event was enqueued to.
    pub delivered: usize,
    /// Matching registrations that were stale or refused the event.
    pub disconnected: usize,
}
