use crate::entity::Entity;
use crate::entity::EventKey;
use crate::message::BusMessage;
use crate::policy::Policy;
use crate::BusError;
use omnibus::BusReceiver;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;

/// A subscription with its own pending queue.
///
/// The queue is where deduplication happens: `omnibus` fans events out in
/// publish order, and the subscriber folds whatever is still undelivered before
/// yielding the next event. Draining the channel before yielding is what makes
/// "latest" mean "latest queued at read time" rather than "latest published
/// after the previous read".
pub struct Subscriber<M: BusMessage> {
    rx: BusReceiver<Entity, M::Topic, M>,
    pending: VecDeque<Pending<M>>,
    /// Pending slot of every keyed entry, so `Update`/`Delta` rewrite in place.
    index: HashMap<EventKey<M::Topic>, usize>,
    filtered: bool,
}

struct Pending<M: BusMessage> {
    key: Option<EventKey<M::Topic>>,
    event: M,
}

impl<M: BusMessage> Subscriber<M> {
    pub(crate) fn new(rx: BusReceiver<Entity, M::Topic, M>, filtered: bool) -> Self {
        Self {
            rx,
            pending: VecDeque::new(),
            index: HashMap::new(),
            filtered,
        }
    }

    /// Non-blocking; folds everything currently queued, then yields the next event.
    pub fn try_recv(&mut self) -> Option<M> {
        self.absorb();
        self.pop_front()
    }

    /// Blocks up to `timeout` only while nothing is queued; `Ok(None)` on timeout.
    pub fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<M>, BusError> {
        if let Some(event) = self.try_recv() {
            return Ok(Some(event));
        }
        match self.rx.recv_timeout(timeout)? {
            Some(event) => {
                self.push(event.payload().clone());
                Ok(self.try_recv())
            }
            None => Ok(None),
        }
    }

    /// Blocks until one event is queued.
    pub fn recv(&mut self) -> Result<M, BusError> {
        loop {
            if let Some(event) = self.try_recv() {
                return Ok(event);
            }
            let event = self.rx.recv_blocking()?;
            self.push(event.payload().clone());
        }
    }

    /// Move everything the channel holds into the pending queue, folding as it goes.
    fn absorb(&mut self) {
        loop {
            let event = match self.rx.recv() {
                Ok(Some(event)) => event,
                // Empty, or the bus is gone and nothing more can arrive.
                Ok(None) | Err(_) => break,
            };
            let payload = event.payload().clone();
            self.push(payload);
        }
    }

    /// Fold one incoming event into the pending queue.
    fn push(&mut self, event: M) {
        let policy = event.policy();
        // `Signal` never carries a key: an occurrence cannot be folded away.
        let key = match policy {
            Policy::Signal => None,
            _ => event.key(),
        };
        let Some(key) = key else {
            self.append(None, event);
            return;
        };
        match policy {
            // Unreachable: a `Signal` event has no key and returned above.
            Policy::Signal => self.append(None, event),
            Policy::Create => {
                if !self.index.contains_key(&key) {
                    self.append(Some(key), event);
                }
            }
            Policy::Update => match self.index.get(&key) {
                Some(&slot) => self.replace(slot, event),
                None => self.append(Some(key), event),
            },
            Policy::Delta => match self.index.get(&key) {
                Some(&slot) => self.merge(slot, &event),
                None => self.append(Some(key), event),
            },
            Policy::Toggle => {
                if self.folds_toggle(&key) {
                    self.drop_tail();
                } else {
                    self.append(Some(key), event);
                }
            }
        }
    }

    /// Two adjacent toggles on one key can only mean "back where we started".
    /// A toggle separated from its pair by any other queued event cannot be
    /// folded without knowing that event's effect, so only the tail folds.
    fn folds_toggle(&self, key: &EventKey<M::Topic>) -> bool {
        if self.filtered {
            return false;
        }
        match self.pending.back() {
            Some(pending) => pending.key.as_ref() == Some(key),
            None => false,
        }
    }

    fn append(&mut self, key: Option<EventKey<M::Topic>>, event: M) {
        let slot = self.pending.len();
        if let Some(key) = &key {
            self.index.insert(key.clone(), slot);
        }
        self.pending.push_back(Pending { key, event });
    }

    fn replace(&mut self, slot: usize, event: M) {
        if let Some(pending) = self.pending.get_mut(slot) {
            pending.event = event;
        }
    }

    fn merge(&mut self, slot: usize, event: &M) {
        if let Some(pending) = self.pending.get_mut(slot) {
            pending.event.merge(event);
        }
    }

    fn drop_tail(&mut self) {
        if let Some(pending) = self.pending.pop_back() {
            if let Some(key) = pending.key {
                self.index.remove(&key);
            }
        }
    }

    fn pop_front(&mut self) -> Option<M> {
        let front = self.pending.pop_front()?;
        // Only drop the mapping when it still points at this entry: a `Toggle`
        // pair separated by another event leaves the same key queued twice, and
        // `index` tracks the newest of those.
        if let Some(key) = &front.key {
            if self.index.get(key) == Some(&0) {
                self.index.remove(key);
            }
        }
        // Every remaining slot moved down by one.
        for slot in self.index.values_mut() {
            *slot -= 1;
        }
        Some(front.event)
    }
}
