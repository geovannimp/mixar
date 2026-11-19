//! One subscription: an owned queue, read by exactly one thread.

use crate::channel::poll;
use crate::channel::poll_timeout;
use crate::channel::receive;
use crate::BusError;
use crossbeam_channel::Receiver;
use std::sync::Arc;
use std::time::Duration;

/// Runs when the subscriber goes away, to forget its registration.
pub(crate) type Cleanup = Box<dyn FnOnce() + Send + 'static>;

/// The receiving end of one registration.
///
/// Commands yield an owned message; events yield `Arc<Event>` so a fan-out
/// shares one payload. The subscriber owns no bus, lane or sender, so a
/// receiver that outlives every publisher still sees the lane disconnect.
pub struct Subscriber<M> {
    rx: Receiver<M>,
    /// Held while this subscriber is alive; the lane judges liveness from its
    /// `Weak` half, so it is released first in `Drop`.
    token: Option<Arc<()>>,
    cleanup: Option<Cleanup>,
}

impl<M> Subscriber<M> {
    pub(crate) fn new(rx: Receiver<M>, token: Arc<()>, cleanup: Cleanup) -> Self {
        Self {
            rx,
            token: Some(token),
            cleanup: Some(cleanup),
        }
    }

    /// Take the next already queued message without blocking.
    ///
    /// `Ok(None)` means the lane is open and nothing is queued;
    /// [`BusError::Closed`] means it disconnected and the queue is empty.
    pub fn try_recv(&mut self) -> Result<Option<M>, BusError> {
        poll(&self.rx)
    }

    /// Wait up to `timeout` for the next message; `Ok(None)` on timeout.
    pub fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<M>, BusError> {
        poll_timeout(&self.rx, timeout)
    }

    /// Block until the next message, or until the lane disconnects.
    pub fn recv(&mut self) -> Result<M, BusError> {
        receive(&self.rx)
    }
}

impl<M> Drop for Subscriber<M> {
    fn drop(&mut self) {
        // Release liveness before cleanup: a registration that is already
        // pruned must not be pruned twice, and a replacement registered under
        // the same target in the meantime must survive this destructor.
        self.token = None;
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}
