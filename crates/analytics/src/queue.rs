use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use crate::CaptureEvent;

pub(crate) const QUEUE_CAPACITY: usize = 64;
pub(crate) const BATCH_SIZE: usize = 20;
const FLUSH_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct Queue {
    events: Arc<(Mutex<QueueState>, Condvar)>,
}

#[derive(Default)]
struct QueueState {
    pending: VecDeque<CaptureEvent>,
    active: bool,
}

impl Queue {
    pub(crate) fn new() -> Self {
        Self {
            events: Arc::new((
                Mutex::new(QueueState {
                    active: true,
                    ..QueueState::default()
                }),
                Condvar::new(),
            )),
        }
    }

    /// Queue state stays consistent across a panic elsewhere, so recover from a
    /// poisoned lock instead of silently dropping every later event.
    fn lock(&self) -> MutexGuard<'_, QueueState> {
        self.events
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn push(&self, event: CaptureEvent) {
        let mut state = self.lock();
        if !state.active {
            return;
        }
        if state.pending.len() == QUEUE_CAPACITY {
            state.pending.pop_front();
        }
        state.pending.push_back(event);
        drop(state);
        self.events.1.notify_one();
    }

    pub(crate) fn clear(&self) {
        let mut state = self.lock();
        state.pending.clear();
        state.active = false;
        drop(state);
        self.events.1.notify_all();
    }

    /// False once the user opted out; the flush thread must not post a batch it
    /// already drained.
    pub(crate) fn is_active(&self) -> bool {
        self.lock().active
    }

    #[cfg(test)]
    pub(crate) fn drain(&self) -> Vec<CaptureEvent> {
        let mut state = self.lock();
        Self::drain_batch(&mut state)
    }

    fn drain_batch(state: &mut QueueState) -> Vec<CaptureEvent> {
        (0..BATCH_SIZE)
            .filter_map(|_| state.pending.pop_front())
            .collect()
    }

    pub(crate) fn wait_batch(&self) -> Option<Vec<CaptureEvent>> {
        let condition = &self.events.1;
        let mut state = self.lock();
        if state.pending.is_empty() && state.active {
            state = condition
                .wait_timeout(state, FLUSH_INTERVAL)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
        if !state.active {
            return None;
        }
        Some(Self::drain_batch(&mut state))
    }
}
