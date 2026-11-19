//! The contract of a message that travels on one lane.

use std::hash::Hash;

/// A message that a [`crate::EventBus`] can route.
///
/// One lane routes by [`Message::topic`]: commands by owner, events by topic
/// (so an observer can select the events it handles). There
/// is deliberately no `Clone` bound — a command moves into its owner's queue
/// without being copied, and an event is shared through `Arc` instead.
pub trait Message: Send + 'static {
    /// Lane discriminator: the command owner, or the event topic.
    type Topic: Copy + Eq + Hash + Send + Sync + 'static;

    /// The discriminator this message routes on.
    fn topic(&self) -> Self::Topic;
}
