use crate::entity::EventKey;
use crate::policy::Policy;
use std::hash::Hash;

/// Contract implemented by one concrete event payload.
///
/// `bus_events!` derives this impl from the event table, so payloads never write
/// it by hand; it exists so a payload's merge behaviour travels with its fields.
pub trait EventSpec: Clone + Send + Sync + 'static {
    const POLICY: Policy;
    type Topic: Copy + Eq + Hash + Send + Sync + 'static;
    /// Filter discriminator; one value per concrete event type.
    const TOPIC: Self::Topic;
    /// Pending-queue key. `None` disables merging for this value (falls back to `Signal`).
    fn key(&self) -> Option<EventKey<Self::Topic>>;
    /// Merge `later` into a pending `self`. Called for `Update` and `Delta` only.
    /// Default (used by `Update`) is whole-value replacement.
    fn merge(&mut self, later: &Self) {
        *self = later.clone();
    }
}
