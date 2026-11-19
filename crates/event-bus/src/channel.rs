//! Lane state: registration, admission and lifecycle for one direction.
//!
//! A lane owns one mutex, its registration table and a poison flag. Commands get
//! exactly one inbox per target; events get one queue per observer. Both hold
//! the lane mutex across matching and enqueueing, which is what makes the
//! enqueue order total for concurrent publishers inside a lane.
//!
//! No user code runs under a lane lock: sending hands the message to a queue,
//! and everything that can run a destructor (a retired registration, a rejected
//! command, an undeliverable payload) is dropped after the guard is released.

use crate::BusError;
use crate::Message;
use crossbeam_channel::unbounded;
use crossbeam_channel::Receiver;
use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::Sender;
use crossbeam_channel::TryRecvError;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::Weak;
use std::time::Duration;

use crate::PublishReport;

/// One point-in-time view of a subscription.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionDiagnostics {
    /// Registration id; unique within its lane and increasing with registration.
    pub id: u64,
    /// The label its owner registered with.
    pub label: &'static str,
    /// Messages accepted into the queue and not yet received.
    pub queued: usize,
}

/// One point-in-time view of a lane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneDiagnostics {
    /// Whether [`crate::EventBus::close`] detached this lane's senders.
    pub closed: bool,
    /// Live registrations, ordered by id.
    pub subscriptions: Vec<SubscriptionDiagnostics>,
}

/// Consume one already queued message without blocking.
///
/// `Ok(None)` is an open, empty queue; [`BusError::Closed`] is a disconnected
/// one, so a caller can tell "nothing yet" from "no more ever".
pub(crate) fn poll<M>(rx: &Receiver<M>) -> Result<Option<M>, BusError> {
    match rx.try_recv() {
        Ok(message) => Ok(Some(message)),
        Err(TryRecvError::Empty) => Ok(None),
        Err(TryRecvError::Disconnected) => Err(BusError::Closed),
    }
}

/// Wait up to `timeout` for the next queued message.
pub(crate) fn poll_timeout<M>(rx: &Receiver<M>, timeout: Duration) -> Result<Option<M>, BusError> {
    match rx.recv_timeout(timeout) {
        Ok(message) => Ok(Some(message)),
        Err(RecvTimeoutError::Timeout) => Ok(None),
        Err(RecvTimeoutError::Disconnected) => Err(BusError::Closed),
    }
}

/// Block until the next queued message, or until the lane disconnects.
pub(crate) fn receive<M>(rx: &Receiver<M>) -> Result<M, BusError> {
    rx.recv().map_err(|_error| BusError::Closed)
}

/// Test-only breadcrumb: is a lane lock held by the current thread?
///
/// A payload or registration destructor must never run under a lane lock, and a
/// destructor cannot lock the lane to find out without deadlocking. The depth
/// counter lets a fixture's `Drop` assert instead of hang.
#[cfg(test)]
mod probe {
    use std::cell::Cell;

    thread_local! {
        static DEPTH: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) struct Held;

    impl Held {
        pub(super) fn enter() -> Self {
            DEPTH.with(|depth| depth.set(depth.get() + 1));
            Self
        }
    }

    impl Drop for Held {
        fn drop(&mut self) {
            DEPTH.with(|depth| depth.set(depth.get() - 1));
        }
    }

    pub(super) fn depth() -> usize {
        DEPTH.with(Cell::get)
    }
}

#[cfg(not(test))]
mod probe {
    pub(super) struct Held;

    impl Held {
        pub(super) fn enter() -> Self {
            Self
        }
    }
}

/// A lane lock, with the test-only breadcrumb in place.
pub(crate) struct LaneGuard<'a, T> {
    guard: MutexGuard<'a, T>,
    _held: probe::Held,
}

impl<T> Deref for LaneGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.guard
    }
}

impl<T> DerefMut for LaneGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}

fn wrap<T>(guard: MutexGuard<'_, T>) -> LaneGuard<'_, T> {
    LaneGuard {
        guard,
        _held: probe::Held::enter(),
    }
}

/// A registration whose owner is gone.
fn is_dead(live: &Weak<()>) -> bool {
    live.strong_count() == 0
}

/// Bump one registration counter, refusing to wrap.
fn next_id(next_id: &mut u64) -> Result<u64, BusError> {
    let id = *next_id;
    *next_id = next_id.checked_add(1).ok_or(BusError::IdExhausted)?;
    Ok(id)
}

/// One observer's event queue.
struct EventRegistration<M: Message> {
    id: u64,
    label: &'static str,
    /// `None` subscribes to every topic.
    filter: Option<HashSet<M::Topic>>,
    tx: Sender<Arc<M>>,
    live: Weak<()>,
}

struct EventState<M: Message> {
    closed: bool,
    next_id: u64,
    registrations: Vec<EventRegistration<M>>,
}

/// The event lane: many observers, each receiving every matching message.
pub(crate) struct EventLane<M: Message> {
    poisoned: AtomicBool,
    state: Mutex<EventState<M>>,
}

impl<M: Message + Sync> EventLane<M> {
    pub(crate) fn new() -> Self {
        Self {
            poisoned: AtomicBool::new(false),
            state: Mutex::new(EventState {
                closed: false,
                next_id: 1,
                registrations: Vec::new(),
            }),
        }
    }

    /// Lock the lane, or report and latch the failure.
    ///
    /// A poisoned lane is fail-closed: the registry is detached once, outside
    /// the lock, and every later operation reports [`BusError::Poisoned`].
    fn lock(&self) -> Result<LaneGuard<'_, EventState<M>>, BusError> {
        if self.poisoned.load(Ordering::Acquire) {
            return Err(BusError::Poisoned);
        }
        match self.state.lock() {
            Ok(guard) => Ok(wrap(guard)),
            Err(poisoned) => {
                self.poisoned.store(true, Ordering::Release);
                let mut guard = poisoned.into_inner();
                guard.closed = true;
                let detached = std::mem::take(&mut guard.registrations);
                drop(guard);
                drop(detached);
                Err(BusError::Poisoned)
            }
        }
    }

    /// Register one observer.
    pub(crate) fn subscribe(
        lane: &Arc<Self>,
        label: &'static str,
        filter: Option<HashSet<M::Topic>>,
    ) -> Result<crate::Subscriber<Arc<M>>, BusError> {
        let mut guard = lane.lock()?;
        if guard.closed {
            drop(guard);
            return Err(BusError::Closed);
        }
        let id = next_id(&mut guard.next_id)?;
        // Reserve before the channel exists, so growth cannot unwind with the
        // fresh sender owned by an in-flight push.
        guard.registrations.reserve(1);
        let (tx, rx) = unbounded();
        let token = Arc::new(());
        guard.registrations.push(EventRegistration {
            id,
            label,
            filter,
            tx,
            live: Arc::downgrade(&token),
        });
        drop(guard);

        let weak = Arc::downgrade(lane);
        Ok(crate::Subscriber::new(
            rx,
            token,
            Box::new(move || {
                if let Some(lane) = weak.upgrade() {
                    lane.remove(id);
                }
            }),
        ))
    }

    /// Forget one registration, if it is still this one.
    fn remove(&self, id: u64) {
        let Ok(mut guard) = self.lock() else {
            return;
        };
        let removed = match guard.registrations.iter().position(|entry| entry.id == id) {
            Some(index) => Some(guard.registrations.remove(index)),
            None => None,
        };
        drop(guard);
        drop(removed);
    }

    /// Fan one event out to every matching observer.
    pub(crate) fn publish(&self, message: M) -> Result<PublishReport, BusError> {
        let topic = message.topic();
        // Everything that can run a destructor is declared before the guard, so
        // an unwind releases the lane before any of it is dropped.
        let payload = Arc::new(message);
        let mut retired: Vec<EventRegistration<M>> = Vec::new();
        let mut undelivered: Vec<crossbeam_channel::SendError<Arc<M>>> = Vec::new();
        let mut dead: Vec<usize> = Vec::new();
        let mut report = PublishReport::default();

        let mut guard = self.lock()?;
        if guard.closed {
            drop(guard);
            return Err(BusError::Closed);
        }
        for (index, entry) in guard.registrations.iter().enumerate() {
            if entry
                .filter
                .as_ref()
                .is_some_and(|filter| !filter.contains(&topic))
            {
                continue;
            }
            report.matched += 1;
            if is_dead(&entry.live) {
                report.disconnected += 1;
                dead.push(index);
                continue;
            }
            match entry.tx.send(Arc::clone(&payload)) {
                Ok(()) => report.delivered += 1,
                Err(refused) => {
                    report.disconnected += 1;
                    dead.push(index);
                    undelivered.push(refused);
                }
            }
        }
        for index in dead.into_iter().rev() {
            retired.push(guard.registrations.remove(index));
        }

        drop(guard);
        drop(retired);
        drop(undelivered);
        drop(payload);
        Ok(report)
    }

    /// Detach every sender; queued messages stay readable.
    pub(crate) fn close(&self) -> Result<(), BusError> {
        let mut guard = self.lock()?;
        guard.closed = true;
        let detached = std::mem::take(&mut guard.registrations);
        drop(guard);
        drop(detached);
        Ok(())
    }

    /// A copied snapshot, ordered by id.
    pub(crate) fn diagnostics(&self) -> Result<LaneDiagnostics, BusError> {
        let guard = self.lock()?;
        let mut subscriptions: Vec<SubscriptionDiagnostics> = guard
            .registrations
            .iter()
            .map(|entry| SubscriptionDiagnostics {
                id: entry.id,
                label: entry.label,
                queued: entry.tx.len(),
            })
            .collect();
        let closed = guard.closed;
        drop(guard);
        subscriptions.sort_by_key(|entry| entry.id);
        Ok(LaneDiagnostics {
            closed,
            subscriptions,
        })
    }
}

/// One command owner's inbox.
struct CommandRegistration<M: Message> {
    id: u64,
    label: &'static str,
    tx: Sender<M>,
    live: Weak<()>,
}

struct CommandState<M: Message> {
    closed: bool,
    next_id: u64,
    inboxes: HashMap<M::Topic, CommandRegistration<M>>,
}

impl<M: Message> CommandState<M> {
    /// Retire registrations whose subscriber is gone, into `retired`.
    ///
    /// `retired` must be owned by the caller outside the lane lock: a collected
    /// `Sender` can own the channel, and dropping the last one drops whatever is
    /// still queued in it.
    fn prune_dead(&mut self, retired: &mut Vec<CommandRegistration<M>>) {
        let stale: Vec<M::Topic> = self
            .inboxes
            .iter()
            .filter(|(_, entry)| is_dead(&entry.live))
            .map(|(topic, _)| *topic)
            .collect();
        for topic in stale {
            if let Some(entry) = self.inboxes.remove(&topic) {
                retired.push(entry);
            }
        }
    }
}

/// The command lane: exactly one owned inbox per target.
pub(crate) struct CommandLane<M: Message> {
    poisoned: AtomicBool,
    state: Mutex<CommandState<M>>,
}

impl<M: Message> CommandLane<M> {
    pub(crate) fn new() -> Self {
        Self {
            poisoned: AtomicBool::new(false),
            state: Mutex::new(CommandState {
                closed: false,
                next_id: 1,
                inboxes: HashMap::new(),
            }),
        }
    }

    /// Lock the lane, or report and latch the failure.
    fn lock(&self) -> Result<LaneGuard<'_, CommandState<M>>, BusError> {
        if self.poisoned.load(Ordering::Acquire) {
            return Err(BusError::Poisoned);
        }
        match self.state.lock() {
            Ok(guard) => Ok(wrap(guard)),
            Err(poisoned) => {
                self.poisoned.store(true, Ordering::Release);
                let mut guard = poisoned.into_inner();
                guard.closed = true;
                let detached = std::mem::take(&mut guard.inboxes);
                drop(guard);
                drop(detached);
                Err(BusError::Poisoned)
            }
        }
    }

    /// Claim the inbox for one target.
    pub(crate) fn subscribe(
        lane: &Arc<Self>,
        target: M::Topic,
        label: &'static str,
    ) -> Result<crate::Subscriber<M>, BusError> {
        // Declared before the guard so every early return can unlock first.
        let mut retired: Vec<CommandRegistration<M>> = Vec::new();
        let mut guard = lane.lock()?;
        if guard.closed {
            drop(guard);
            drop(retired);
            return Err(BusError::Closed);
        }
        // A stale registration must not block its replacement.
        guard.prune_dead(&mut retired);
        if guard.inboxes.contains_key(&target) {
            drop(guard);
            drop(retired);
            return Err(BusError::AlreadySubscribed);
        }
        let id = match next_id(&mut guard.next_id) {
            Ok(id) => id,
            Err(reason) => {
                drop(guard);
                drop(retired);
                return Err(reason);
            }
        };
        // Reserve before the channel exists: growth cannot unwind with the
        // sender (and the receiver) owned by an in-flight insertion.
        guard.inboxes.reserve(1);
        let (tx, rx) = unbounded();
        let token = Arc::new(());
        guard.inboxes.insert(
            target,
            CommandRegistration {
                id,
                label,
                tx,
                live: Arc::downgrade(&token),
            },
        );
        drop(guard);
        drop(retired);

        let weak = Arc::downgrade(lane);
        Ok(crate::Subscriber::new(
            rx,
            token,
            Box::new(move || {
                if let Some(lane) = weak.upgrade() {
                    lane.remove(target, id);
                }
            }),
        ))
    }

    /// Forget one registration, if it is still this one.
    fn remove(&self, target: M::Topic, id: u64) {
        let Ok(mut guard) = self.lock() else {
            return;
        };
        let current = guard
            .inboxes
            .get(&target)
            .is_some_and(|entry| entry.id == id);
        let removed = if current {
            guard.inboxes.remove(&target)
        } else {
            None
        };
        drop(guard);
        drop(removed);
    }

    /// Move one command into its owner's queue.
    pub(crate) fn publish(&self, message: M) -> Result<(), crate::CommandPublishError<M>> {
        let target = message.topic();
        // Everything that can run a destructor is declared before the guard, so
        // an unwind (or the explicit drops below) releases the lane first: a
        // cloned sender or a retired registration can own the channel, and
        // dropping the last sender drops whatever is still queued in it.
        let mut payload = Some(message);
        let mut retired: Vec<CommandRegistration<M>> = Vec::new();
        let mut sender: Option<Sender<M>> = None;
        let mut rejected: Option<M> = None;

        let outcome = {
            let mut guard = match self.lock() {
                Ok(guard) => guard,
                Err(reason) => {
                    return Err(crate::CommandPublishError {
                        reason,
                        command: payload.take().expect("the command is still unsent"),
                    });
                }
            };
            if guard.closed {
                Err(BusError::Closed)
            } else {
                // A stale registration must not block its replacement.
                guard.prune_dead(&mut retired);
                match guard.inboxes.get(&target).map(|entry| entry.tx.clone()) {
                    None => Err(BusError::NoCommandHandler),
                    Some(tx) => {
                        // The sender lives outside the guard before it is used,
                        // so a panic anywhere below cannot drop it under the lock.
                        sender = Some(tx);
                        let tx = sender.as_ref().expect("the sender was just stored");
                        match tx.send(payload.take().expect("the command is still unsent")) {
                            Ok(()) => Ok(()),
                            Err(undelivered) => {
                                // The owner is gone: keep the command, then retire
                                // its registration (the lookup can panic).
                                rejected = Some(undelivered.into_inner());
                                if let Some(entry) = guard.inboxes.remove(&target) {
                                    retired.push(entry);
                                }
                                Err(BusError::NoCommandHandler)
                            }
                        }
                    }
                }
            }
        };

        // The lane lock is released here; the values below cannot re-enter it.
        drop(sender);
        drop(retired);
        match outcome {
            Ok(()) => Ok(()),
            Err(reason) => {
                let command = rejected
                    .or_else(|| payload.take())
                    .expect("a rejected command is retained");
                Err(crate::CommandPublishError { reason, command })
            }
        }
    }

    /// Detach every sender; queued commands stay readable.
    pub(crate) fn close(&self) -> Result<(), BusError> {
        let mut guard = self.lock()?;
        guard.closed = true;
        let detached = std::mem::take(&mut guard.inboxes);
        drop(guard);
        drop(detached);
        Ok(())
    }

    /// A copied snapshot, ordered by id.
    pub(crate) fn diagnostics(&self) -> Result<LaneDiagnostics, BusError> {
        let guard = self.lock()?;
        let mut subscriptions: Vec<SubscriptionDiagnostics> = guard
            .inboxes
            .values()
            .map(|entry| SubscriptionDiagnostics {
                id: entry.id,
                label: entry.label,
                queued: entry.tx.len(),
            })
            .collect();
        let closed = guard.closed;
        drop(guard);
        subscriptions.sort_by_key(|entry| entry.id);
        Ok(LaneDiagnostics {
            closed,
            subscriptions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventBus;
    use std::hash::Hash;
    use std::hash::Hasher;
    use std::panic;
    use std::panic::AssertUnwindSafe;
    use std::sync::atomic::AtomicUsize;

    /// The fixture tests share these panic controls and counters, so they run
    /// one at a time: nextest runs tests in parallel threads of one process.
    static FIXTURE_LOCK: Mutex<()> = Mutex::new(());

    fn fixture() -> std::sync::MutexGuard<'static, ()> {
        // A test that panics inside `catch_unwind` poisons the lock on purpose;
        // recover it so the next fixture test still runs.
        FIXTURE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// How many topic comparisons pass before `PanicTopic` panics.
    static EQ_LIMIT: AtomicUsize = AtomicUsize::new(usize::MAX);
    static EQ_CALLS: AtomicUsize = AtomicUsize::new(0);
    /// Drops that ran while a lane lock was held (must stay zero).
    static VIOLATIONS: AtomicUsize = AtomicUsize::new(0);

    /// A topic whose comparison can be made to panic inside the lane lock.
    #[derive(Clone, Copy, Debug)]
    struct PanicTopic(u8);

    impl PartialEq for PanicTopic {
        fn eq(&self, other: &Self) -> bool {
            let limit = EQ_LIMIT.load(Ordering::Relaxed);
            if limit != usize::MAX && EQ_CALLS.fetch_add(1, Ordering::Relaxed) >= limit {
                panic!("topic comparison panicked");
            }
            self.0 == other.0
        }
    }

    impl Eq for PanicTopic {}

    /// Every topic lands in one bucket, so `contains` always compares and the
    /// panic above is reachable instead of being skipped by a hash mismatch.
    impl Hash for PanicTopic {
        fn hash<H: Hasher>(&self, state: &mut H) {
            0u8.hash(state);
        }
    }

    fn arm(limit: usize) {
        EQ_CALLS.store(0, Ordering::Relaxed);
        EQ_LIMIT.store(limit, Ordering::Relaxed);
    }

    fn disarm() {
        EQ_LIMIT.store(usize::MAX, Ordering::Relaxed);
        EQ_CALLS.store(0, Ordering::Relaxed);
    }

    /// An event that records a destructor running under a lane lock.
    #[derive(Debug, PartialEq)]
    struct ProbeEvent(u8);

    impl Drop for ProbeEvent {
        fn drop(&mut self) {
            if probe::depth() != 0 {
                VIOLATIONS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    impl Message for ProbeEvent {
        type Topic = PanicTopic;

        fn topic(&self) -> PanicTopic {
            PanicTopic(0)
        }
    }

    /// A command that records a destructor running under a lane lock.
    #[derive(Debug, PartialEq)]
    struct ProbeCommand(u8);

    impl Drop for ProbeCommand {
        fn drop(&mut self) {
            if probe::depth() != 0 {
                VIOLATIONS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    impl Message for ProbeCommand {
        type Topic = PanicTopic;

        fn topic(&self) -> PanicTopic {
            PanicTopic(7)
        }
    }

    #[test]
    fn a_panic_after_one_enqueue_fails_the_event_lane_closed() {
        let _fixture = fixture();
        VIOLATIONS.store(0, Ordering::Relaxed);
        let bus = EventBus::<ProbeCommand, ProbeEvent>::new();
        let mut first = bus
            .subscribe_evt_topics("first", &[PanicTopic(0)])
            .expect("observer");
        let mut second = bus
            .subscribe_evt_topics("second", &[PanicTopic(1)])
            .expect("observer");

        // The first observer matches, so its message is already queued when the
        // second comparison unwinds inside the lane lock.
        arm(1);
        let panicked = panic::catch_unwind(AssertUnwindSafe(|| {
            bus.publish_evt(ProbeEvent(1)).expect("publish")
        }));
        disarm();
        assert!(
            panicked.is_err(),
            "the fixture panic must escape the publish"
        );
        drop(panicked);

        // Fail-closed: the lane reports poisoned and stays that way.
        assert_eq!(bus.publish_evt(ProbeEvent(2)), Err(BusError::Poisoned));
        assert_eq!(
            bus.diagnostics().err(),
            Some(BusError::Poisoned),
            "diagnostics refuse a poisoned lane"
        );

        // The outcome was partial, and both senders are gone now.
        assert_eq!(
            first.try_recv().expect("drains"),
            Some(Arc::new(ProbeEvent(1)))
        );
        assert_eq!(first.try_recv(), Err(BusError::Closed));
        assert_eq!(
            second.try_recv(),
            Err(BusError::Closed),
            "the observer that never received is disconnected, not starved"
        );
        assert_eq!(
            VIOLATIONS.load(Ordering::Relaxed),
            0,
            "unwinding must not drop a payload under the lane lock"
        );
    }

    #[test]
    fn a_poisoned_command_lane_returns_the_unsent_command() {
        let _fixture = fixture();
        VIOLATIONS.store(0, Ordering::Relaxed);
        let bus = EventBus::<ProbeCommand, ProbeEvent>::new();
        let _owner = bus
            .subscribe_cmd(PanicTopic(7), "engine")
            .expect("engine inbox");

        arm(0);
        let panicked = panic::catch_unwind(AssertUnwindSafe(|| {
            let _ = bus.publish_cmd(ProbeCommand(1));
        }));
        disarm();
        assert!(
            panicked.is_err(),
            "the fixture panic must escape the publish"
        );
        drop(panicked);

        let error = bus
            .publish_cmd(ProbeCommand(2))
            .expect_err("a poisoned lane refuses new commands");
        assert_eq!(error.reason, BusError::Poisoned);
        let returned = error.command;
        assert_eq!(returned, ProbeCommand(2), "ownership is preserved");
        // Release it before the probe, so its destructor is part of what is checked.
        drop(returned);
        assert_eq!(
            bus.subscribe_cmd(PanicTopic(7), "engine").err(),
            Some(BusError::Poisoned)
        );
        assert_eq!(
            VIOLATIONS.load(Ordering::Relaxed),
            0,
            "a refused command is dropped outside the lane lock"
        );
    }

    #[test]
    fn a_disconnected_owner_returns_the_unsent_command() {
        let _fixture = fixture();
        VIOLATIONS.store(0, Ordering::Relaxed);
        let lane = Arc::new(CommandLane::<ProbeCommand>::new());
        // A registration whose receiver is already gone while its liveness token
        // is still held: cleanup normally removes the entry first, so this is the
        // lane's defensive path, seeded directly here.
        let token = Arc::new(());
        {
            let mut guard = lane.lock().expect("lane open");
            let (tx, rx) = unbounded();
            drop(rx);
            let id = guard.next_id;
            guard.next_id = id + 1;
            guard.inboxes.insert(
                PanicTopic(7),
                CommandRegistration {
                    id,
                    label: "stale",
                    tx,
                    live: Arc::downgrade(&token),
                },
            );
        }

        let error = lane
            .publish(ProbeCommand(1))
            .expect_err("the owner is gone");
        assert_eq!(error.reason, BusError::NoCommandHandler);
        let returned = error.command;
        assert_eq!(returned, ProbeCommand(1), "the unsent command comes back");
        drop(returned);

        assert_eq!(
            lane.diagnostics().expect("lane open").subscriptions.len(),
            0,
            "the stale registration is retired, not left to fail again"
        );
        assert_eq!(
            VIOLATIONS.load(Ordering::Relaxed),
            0,
            "the rejected command is dropped outside the lane lock"
        );
        drop(token);
    }

    #[test]
    fn an_undeliverable_event_is_retired_and_the_live_observer_still_receives() {
        let _fixture = fixture();
        let lane = Arc::new(EventLane::<ProbeEvent>::new());
        let mut live = EventLane::subscribe(&lane, "live", None).expect("observer");

        // The same defensive state on the event lane: a queued observer whose
        // receiver is gone but whose cleanup has not run.
        let token = Arc::new(());
        {
            let mut guard = lane.lock().expect("lane open");
            let (tx, rx) = unbounded();
            drop(rx);
            let id = guard.next_id;
            guard.next_id = id + 1;
            guard.registrations.push(EventRegistration {
                id,
                label: "stale",
                filter: None,
                tx,
                live: Arc::downgrade(&token),
            });
        }

        let report = lane.publish(ProbeEvent(3)).expect("publish");
        assert_eq!(report.matched, 2);
        assert_eq!(report.delivered, 1, "the live observer still receives");
        assert_eq!(report.disconnected, 1, "the refused observer is counted");
        assert_eq!(
            live.try_recv().expect("lane open"),
            Some(Arc::new(ProbeEvent(3)))
        );
        assert_eq!(
            lane.diagnostics().expect("lane open").subscriptions.len(),
            1,
            "the stale observer is retired"
        );
        drop(token);
    }

    #[test]
    fn a_retired_cleanup_cannot_remove_its_replacement() {
        // `PanicTopic` consults the shared panic controls, so this fixture test
        // takes the same lock as the others.
        let _fixture = fixture();
        let lane = Arc::new(CommandLane::<ProbeCommand>::new());
        let first = CommandLane::subscribe(&lane, PanicTopic(7), "first").expect("inbox");
        let first_id = lane.diagnostics().expect("diagnostics").subscriptions[0].id;
        drop(first);

        let replacement =
            CommandLane::subscribe(&lane, PanicTopic(7), "replacement").expect("replacement");
        let replacement_id = lane.diagnostics().expect("diagnostics").subscriptions[0].id;
        assert_ne!(first_id, replacement_id, "a replacement gets a new id");

        // The old registration's destructor running late: same target, old id.
        lane.remove(PanicTopic(7), first_id);

        let diagnostics = lane.diagnostics().expect("diagnostics");
        assert_eq!(
            diagnostics.subscriptions.len(),
            1,
            "a stale cleanup must not unregister the replacement"
        );
        assert_eq!(diagnostics.subscriptions[0].id, replacement_id);
        assert_eq!(diagnostics.subscriptions[0].label, "replacement");
        drop(replacement);
    }

    #[test]
    fn retired_messages_are_dropped_outside_the_lane_locks() {
        let _fixture = fixture();
        VIOLATIONS.store(0, Ordering::Relaxed);
        let bus = EventBus::<ProbeCommand, ProbeEvent>::new();

        // A rejected command (no owner) is handed back and dropped by the caller.
        let error = bus
            .publish_cmd(ProbeCommand(1))
            .expect_err("no command handler");
        drop(error);

        // An undeliverable event: the observer is gone, so the payload is
        // retired under the lane lock and must be released after it.
        let observer = bus.subscribe_evt("stale").expect("observer");
        drop(observer);
        bus.publish_evt(ProbeEvent(2)).expect("publish");

        // A closed lane retires its registrations and then returns the command.
        let owner = bus
            .subscribe_cmd(PanicTopic(7), "engine")
            .expect("engine inbox");
        bus.close().expect("close");
        drop(owner);
        let rejected = bus
            .publish_cmd(ProbeCommand(3))
            .expect_err("the lane is closed");
        drop(rejected);

        assert_eq!(
            VIOLATIONS.load(Ordering::Relaxed),
            0,
            "a destructor re-entered a lane lock"
        );
    }
}
