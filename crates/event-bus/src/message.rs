use crate::entity::EventKey;
use crate::policy::Policy;
use std::hash::Hash;

/// Contract of the closed event enum that travels on the bus.
///
/// A single enum cannot carry one `TOPIC`/`POLICY` constant per variant, so the
/// enum delegates each question to the payload it wraps.
pub trait BusMessage: Clone + Send + Sync + 'static {
    type Topic: Copy + Eq + Hash + Send + Sync + 'static;
    fn topic(&self) -> Self::Topic;
    fn policy(&self) -> Policy;
    fn key(&self) -> Option<EventKey<Self::Topic>>;
    /// `Update`/`Delta` only: fold `later` into `self` according to policy.
    fn merge(&mut self, later: &Self);
}
