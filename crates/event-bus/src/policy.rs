/// How a subscriber's pending queue folds a new event that collides with a pending one.
///
/// The policy is a property of the event, not of the subscription: every event
/// declares one through `EventSpec::POLICY`, and the pending queue applies it
/// when an event arrives while another event with the same
/// [`crate::EventKey`] is still undelivered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// First event for a key is kept; later events for that key are dropped.
    Create,
    /// Pending value is replaced in place by the newest event for that key.
    Update,
    /// Pending value absorbs the newest event for that key (`EventSpec::merge`).
    Delta,
    /// An adjacent pair of events with the same key cancels; an odd run leaves one.
    /// Only folds on unfiltered subscriptions (see `Subscriber`).
    Toggle,
    /// Never merged, never keyed; appended in publish order.
    Signal,
}
