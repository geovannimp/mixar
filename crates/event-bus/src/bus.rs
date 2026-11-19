//! The cmd/evt pair: two independent lanes with their own ordering.

use crate::channel::CommandLane;
use crate::channel::EventLane;
use crate::channel::LaneDiagnostics;
use crate::BusError;
use crate::CommandPublishError;
use crate::Message;
use crate::PublishReport;
use crate::Subscriber;
use std::collections::HashSet;
use std::sync::Arc;

/// A point-in-time view of both lanes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BusDiagnostics {
    /// The command lane: one owned inbox per target.
    pub commands: LaneDiagnostics,
    /// The event lane: one queue per observer.
    pub events: LaneDiagnostics,
}

/// One process-wide command/event pair over a closed command and event enum.
///
/// Commands move: [`EventBus::publish_cmd`] hands the message to the single
/// owner registered for that target. Events are shared: every matching observer
/// gets the same `Arc<Event>`. Both lanes are unbounded — a stalled consumer
/// grows its queue instead of losing messages.
///
/// # Ordering
///
/// Each lane has one mutex, held across matching and enqueueing, so publishers
/// race only for entry to the lane: every observer sees the same projection of
/// event order, and a command owner sees its target's projection of command
/// order. There is no order *between* lanes and no cross-worker processing
/// order — an enqueue is not a completion.
///
/// # Lifetime
///
/// The bus owns the senders. Dropping the last clone disconnects the lanes after
/// their queues drain; subscribers and diagnostics keep no senders of their own.
/// [`EventBus::close`] does the same on demand, and is idempotent.
pub struct EventBus<C: Message, E: Message + Sync> {
    cmd: Arc<CommandLane<C>>,
    evt: Arc<EventLane<E>>,
}

impl<C: Message, E: Message + Sync> Clone for EventBus<C, E> {
    fn clone(&self) -> Self {
        Self {
            cmd: Arc::clone(&self.cmd),
            evt: Arc::clone(&self.evt),
        }
    }
}

impl<C: Message, E: Message + Sync> Default for EventBus<C, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Message, E: Message + Sync> EventBus<C, E> {
    /// Create a matched, empty cmd/evt pair.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cmd: Arc::new(CommandLane::new()),
            evt: Arc::new(EventLane::new()),
        }
    }

    /// Move one command into its owner's queue.
    ///
    /// The error carries the unsent command back: nothing is retried for the
    /// caller and nothing is silently dropped.
    pub fn publish_cmd(&self, command: C) -> Result<(), CommandPublishError<C>> {
        self.cmd.publish(command)
    }

    /// Share one event with every matching observer.
    pub fn publish_evt(&self, event: E) -> Result<PublishReport, BusError> {
        self.evt.publish(event)
    }

    /// Claim the single owned inbox for `target`.
    ///
    /// Fails with [`BusError::AlreadySubscribed`] while an earlier owner is
    /// alive; dropping that owner (or its bus) frees the target.
    pub fn subscribe_cmd(
        &self,
        target: C::Topic,
        label: &'static str,
    ) -> Result<Subscriber<C>, BusError> {
        CommandLane::subscribe(&self.cmd, target, label)
    }

    /// Observe every event.
    pub fn subscribe_evt(&self, label: &'static str) -> Result<Subscriber<Arc<E>>, BusError> {
        EventLane::subscribe(&self.evt, label, None)
    }

    /// Observe the events whose topic is in `topics`.
    ///
    /// An empty list is [`BusError::EmptyFilter`] rather than a subscription
    /// that can never deliver; duplicates collapse.
    pub fn subscribe_evt_topics(
        &self,
        label: &'static str,
        topics: &[E::Topic],
    ) -> Result<Subscriber<Arc<E>>, BusError> {
        if topics.is_empty() {
            return Err(BusError::EmptyFilter);
        }
        let filter: HashSet<E::Topic> = topics.iter().copied().collect();
        EventLane::subscribe(&self.evt, label, Some(filter))
    }

    /// Detach both lanes' senders, leaving queued messages readable.
    ///
    /// Not atomic across lanes: if one lane is poisoned it is still detached and
    /// the first failure is returned. Call it only once every user has stopped.
    pub fn close(&self) -> Result<(), BusError> {
        let commands = self.cmd.close();
        let events = self.evt.close();
        commands.and(events)
    }

    /// Copy both lanes' registration state.
    pub fn diagnostics(&self) -> Result<BusDiagnostics, BusError> {
        Ok(BusDiagnostics {
            commands: self.cmd.diagnostics()?,
            events: self.evt.diagnostics()?,
        })
    }
}
