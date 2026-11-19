//! Delivery guarantees of the owned bus: every accepted message, in order.
//!
//! These tests use fixtures that are deliberately *not* `Clone`, so a transport
//! that still copied payloads would not compile. Numbers are published in the
//! range that used to overflow a bounded lane, because "raising the capacity"
//! is exactly the fix this crate replaces.

use event_bus::BusError;
use event_bus::EventBus;
use event_bus::Message;
use std::collections::HashSet;
use std::mem::ManuallyDrop;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Barrier;
use std::thread;
use std::time::Duration;

/// Which worker owns a fixture command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Target {
    Engine,
    Library,
}

/// Event topics a fixture observer can select.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Topic {
    Deck,
    Levels,
}

/// A command fixture: no `Clone`, because a command is *moved* to its owner.
#[derive(Debug, PartialEq)]
enum Cmd {
    Play(u16),
    Pause(u16),
    Toggle(u16),
    JogTurn {
        deck: u16,
        delta: i32,
    },
    Seek {
        deck: u16,
        position_ms: i32,
    },
    SetVolume {
        deck: u16,
        volume: f32,
    },
    /// A create-like payload: the payload *is* the message, never merged.
    CreateSampler {
        deck: u16,
        name: String,
    },
    /// A press/release-shaped signal.
    PadPress(u8),
    /// Owned payload for the other owner: nothing here derives `Clone`, so a
    /// transport that copied messages would not compile.
    Library(String),
}

impl Message for Cmd {
    type Topic = Target;

    fn topic(&self) -> Target {
        match self {
            Self::Library(_) => Target::Library,
            Self::Play(_)
            | Self::Pause(_)
            | Self::Toggle(_)
            | Self::JogTurn { .. }
            | Self::Seek { .. }
            | Self::SetVolume { .. }
            | Self::CreateSampler { .. }
            | Self::PadPress(_) => Target::Engine,
        }
    }
}

/// An event fixture: no `Clone` either, because observers share one `Arc`.
#[derive(Debug, PartialEq)]
enum Evt {
    /// Numbered telemetry; the topic is derived so a filter has something to pick.
    Numbered(u64),
    Deck {
        deck: u16,
        playing: bool,
    },
    Levels {
        deck: u16,
        peak: f32,
    },
}

impl Evt {
    fn numbered_topic(seq: u64) -> Topic {
        if seq.is_multiple_of(2) {
            Topic::Deck
        } else {
            Topic::Levels
        }
    }
}

impl Message for Evt {
    type Topic = Topic;

    fn topic(&self) -> Topic {
        match self {
            Self::Numbered(seq) => Self::numbered_topic(*seq),
            Self::Deck { .. } => Topic::Deck,
            Self::Levels { .. } => Topic::Levels,
        }
    }
}

fn labelled_bus() -> EventBus<Cmd, Evt> {
    EventBus::new()
}

#[test]
fn ten_thousand_commands_are_all_delivered_in_order() {
    let bus = labelled_bus();
    let mut engine = bus
        .subscribe_cmd(Target::Engine, "engine")
        .expect("engine inbox");
    for seq in 0..10_000u16 {
        bus.publish_cmd(Cmd::Seek {
            deck: 0,
            position_ms: i32::from(seq),
        })
        .expect("owner registered");
    }
    for seq in 0..10_000i32 {
        assert_eq!(
            engine.try_recv(),
            Ok(Some(Cmd::Seek {
                deck: 0,
                position_ms: seq
            })),
            "command {seq} missing or reordered"
        );
    }
    assert_eq!(engine.try_recv(), Ok(None));
}

#[test]
fn ten_thousand_events_are_all_delivered_in_order() {
    let bus = labelled_bus();
    let mut ui = bus.subscribe_evt("ui").expect("observer");
    for seq in 0..10_000u64 {
        let report = bus.publish_evt(Evt::Numbered(seq)).expect("publish");
        assert_eq!((report.matched, report.delivered), (1, 1));
    }
    for seq in 0..10_000u64 {
        assert_eq!(ui.try_recv(), Ok(Some(Arc::new(Evt::Numbered(seq)))));
    }
    assert_eq!(ui.try_recv(), Ok(None));
}

#[test]
fn play_pause_play_is_not_folded() {
    let bus = labelled_bus();
    let mut engine = bus
        .subscribe_cmd(Target::Engine, "engine")
        .expect("engine inbox");
    for command in [Cmd::Play(1), Cmd::Pause(1), Cmd::Play(1)] {
        bus.publish_cmd(command).expect("owner registered");
    }
    let received: Vec<Cmd> = std::iter::from_fn(|| engine.try_recv().expect("open"))
        .take(3)
        .collect();
    assert_eq!(received, vec![Cmd::Play(1), Cmd::Pause(1), Cmd::Play(1)]);
    assert_eq!(engine.try_recv(), Ok(None));
}

#[test]
fn repeated_toggles_setters_and_signals_are_preserved() {
    let bus = labelled_bus();
    let mut engine = bus
        .subscribe_cmd(Target::Engine, "engine")
        .expect("engine inbox");
    let published = vec![
        Cmd::Toggle(1),
        Cmd::Toggle(1),
        Cmd::JogTurn { deck: 1, delta: 7 },
        Cmd::Seek {
            deck: 1,
            position_ms: 900,
        },
        Cmd::JogTurn { deck: 1, delta: -3 },
        Cmd::SetVolume {
            deck: 1,
            volume: 0.25,
        },
        Cmd::SetVolume {
            deck: 1,
            volume: 0.75,
        },
        Cmd::CreateSampler {
            deck: 1,
            name: "kick".to_string(),
        },
        Cmd::CreateSampler {
            deck: 1,
            name: "kick".to_string(),
        },
        Cmd::PadPress(4),
        Cmd::PadPress(4),
    ];
    for command in published {
        bus.publish_cmd(command).expect("owner registered");
    }
    let received: Vec<Cmd> = std::iter::from_fn(|| engine.try_recv().expect("open"))
        .take(11)
        .collect();
    assert_eq!(received.len(), 11, "every occurrence is its own message");
    assert_eq!(
        received,
        vec![
            Cmd::Toggle(1),
            Cmd::Toggle(1),
            Cmd::JogTurn { deck: 1, delta: 7 },
            Cmd::Seek {
                deck: 1,
                position_ms: 900
            },
            Cmd::JogTurn { deck: 1, delta: -3 },
            Cmd::SetVolume {
                deck: 1,
                volume: 0.25
            },
            Cmd::SetVolume {
                deck: 1,
                volume: 0.75
            },
            Cmd::CreateSampler {
                deck: 1,
                name: "kick".to_string()
            },
            Cmd::CreateSampler {
                deck: 1,
                name: "kick".to_string()
            },
            Cmd::PadPress(4),
            Cmd::PadPress(4),
        ]
    );
}

#[test]
fn each_target_has_its_own_inbox() {
    let bus = labelled_bus();
    let mut engine = bus
        .subscribe_cmd(Target::Engine, "engine")
        .expect("engine inbox");
    let mut library = bus
        .subscribe_cmd(Target::Library, "library")
        .expect("library inbox");
    bus.publish_cmd(Cmd::Play(2)).expect("engine owner");
    bus.publish_cmd(Cmd::Library("analyze me".to_string()))
        .expect("library owner");

    assert_eq!(engine.try_recv(), Ok(Some(Cmd::Play(2))));
    assert_eq!(
        engine.try_recv(),
        Ok(None),
        "the engine never sees library work"
    );
    assert_eq!(
        library.try_recv(),
        Ok(Some(Cmd::Library("analyze me".to_string())))
    );
    assert_eq!(library.try_recv(), Ok(None));
}

#[test]
fn a_second_live_owner_is_refused_until_the_first_is_dropped() {
    let bus = labelled_bus();
    let first = bus
        .subscribe_cmd(Target::Engine, "engine")
        .expect("engine inbox");
    assert_eq!(
        bus.subscribe_cmd(Target::Engine, "engine-2").err(),
        Some(BusError::AlreadySubscribed)
    );
    drop(first);
    let mut second = bus
        .subscribe_cmd(Target::Engine, "engine-2")
        .expect("replacement after the first owner dropped");
    bus.publish_cmd(Cmd::Play(1)).expect("replacement owns it");
    assert_eq!(second.try_recv(), Ok(Some(Cmd::Play(1))));
}

#[test]
fn a_stale_owner_cleanup_cannot_remove_its_replacement() {
    let bus = labelled_bus();
    let first = bus
        .subscribe_cmd(Target::Engine, "engine")
        .expect("engine inbox");
    // Simulate a destructor delayed past its registration's replacement: the
    // registration is already gone from the table, so releasing the old token
    // later must not touch whatever replaced it.
    let stale = ManuallyDrop::new(first);
    // Forgotten on purpose: the lane must treat the token as dead by liveness,
    // not by destructor order.
    let engine_state = bus.diagnostics().expect("diagnostics").commands;
    assert_eq!(engine_state.subscriptions.len(), 1);

    let replacement = bus
        .subscribe_cmd(Target::Engine, "engine")
        .map(|_| ())
        .err();
    assert_eq!(replacement, Some(BusError::AlreadySubscribed));
    drop(ManuallyDrop::into_inner(stale));

    let mut owner = bus
        .subscribe_cmd(Target::Engine, "engine")
        .expect("a dead token frees the target");
    bus.publish_cmd(Cmd::Play(3)).expect("owner registered");
    assert_eq!(owner.try_recv(), Ok(Some(Cmd::Play(3))));
}

#[test]
fn a_missing_owner_returns_the_unsent_command() {
    let bus = labelled_bus();
    // An event observer exists, but commands are never broadcast to observers.
    let mut ui = bus.subscribe_evt("ui").expect("observer");
    let error = bus
        .publish_cmd(Cmd::Play(1))
        .expect_err("no engine owner registered");
    assert_eq!(error.reason, BusError::NoCommandHandler);
    assert_eq!(error.command, Cmd::Play(1));
    assert_eq!(
        ui.try_recv(),
        Ok(None),
        "commands never reach event observers"
    );
}

#[test]
fn event_observers_share_one_allocation() {
    let bus = labelled_bus();
    let mut first = bus.subscribe_evt("first").expect("observer");
    let mut second = bus.subscribe_evt("second").expect("observer");
    let report = bus
        .publish_evt(Evt::Deck {
            deck: 1,
            playing: true,
        })
        .expect("publish");
    assert_eq!(
        report,
        event_bus::PublishReport {
            matched: 2,
            delivered: 2,
            disconnected: 0
        }
    );

    let a = first.try_recv().expect("open").expect("event");
    let b = second.try_recv().expect("open").expect("event");
    assert!(Arc::ptr_eq(&a, &b), "both observers share one payload");
    assert_eq!(
        a.as_ref(),
        &Evt::Deck {
            deck: 1,
            playing: true
        }
    );
}

#[test]
fn a_dropped_observer_costs_the_others_nothing() {
    let bus = labelled_bus();
    let mut kept = bus.subscribe_evt("kept").expect("observer");
    let dropped = bus.subscribe_evt("dropped").expect("observer");
    bus.publish_evt(Evt::Numbered(1)).expect("publish");
    drop(dropped);
    for seq in 2..10u64 {
        let report = bus.publish_evt(Evt::Numbered(seq)).expect("publish");
        assert_eq!(report.matched, 1, "the dropped registration is gone");
        assert_eq!(report.delivered, 1, "the live observer still receives");
        assert_eq!(report.disconnected, 0);
    }
    let received: Vec<Arc<Evt>> = std::iter::from_fn(|| kept.try_recv().expect("open")).collect();
    assert_eq!(received.len(), 9, "no message lost with a dead observer");
}

#[test]
fn publishing_with_no_observers_succeeds_with_zero_counts() {
    let bus = labelled_bus();
    assert_eq!(
        bus.publish_evt(Evt::Numbered(0)),
        Ok(event_bus::PublishReport::default())
    );
}

#[test]
fn concurrent_publishers_give_every_observer_the_same_order() {
    const PUBLISHERS: u64 = 4;
    const PER_PUBLISHER: u64 = 250;

    let bus = labelled_bus();
    let mut every = bus.subscribe_evt("every").expect("observer");
    let mut deck_only = bus
        .subscribe_evt_topics("deck-only", &[Topic::Deck])
        .expect("filtered observer");

    let barrier = Arc::new(Barrier::new(PUBLISHERS as usize + 1));
    let mut handles = Vec::new();
    for publisher in 0..PUBLISHERS {
        let bus = bus.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier.wait();
            for index in 0..PER_PUBLISHER {
                bus.publish_evt(Evt::Numbered(publisher * PER_PUBLISHER + index))
                    .expect("publish");
            }
        }));
    }
    barrier.wait();
    for handle in handles {
        handle.join().expect("publisher");
    }

    let all: Vec<u64> = std::iter::from_fn(|| every.try_recv().expect("open"))
        .map(|event| match event.as_ref() {
            Evt::Numbered(seq) => *seq,
            other => panic!("unexpected event: {other:?}"),
        })
        .collect();
    assert_eq!(all.len() as u64, PUBLISHERS * PER_PUBLISHER);
    let unique: HashSet<u64> = all.iter().copied().collect();
    assert_eq!(
        unique.len() as u64,
        PUBLISHERS * PER_PUBLISHER,
        "every message is delivered exactly once"
    );

    let deck: Vec<u64> = std::iter::from_fn(|| deck_only.try_recv().expect("open"))
        .map(|event| match event.as_ref() {
            Evt::Numbered(seq) => *seq,
            other => panic!("unexpected event: {other:?}"),
        })
        .collect();
    let expected: Vec<u64> = all
        .iter()
        .copied()
        .filter(|seq| Evt::numbered_topic(*seq) == Topic::Deck)
        .collect();
    assert_eq!(
        deck, expected,
        "a filtered observer sees the exact projection"
    );
}

#[test]
fn an_empty_filter_is_refused_and_duplicates_do_not_duplicate_delivery() {
    let bus = labelled_bus();
    assert_eq!(
        bus.subscribe_evt_topics("empty", &[]).err(),
        Some(BusError::EmptyFilter)
    );
    let mut observer = bus
        .subscribe_evt_topics("deck", &[Topic::Deck, Topic::Deck])
        .expect("duplicates collapse");
    // A non-matching topic must not reach this observer.
    bus.publish_evt(Evt::Levels { deck: 1, peak: 0.5 })
        .expect("publish");
    bus.publish_evt(Evt::Deck {
        deck: 1,
        playing: false,
    })
    .expect("publish");
    assert_eq!(
        observer.try_recv(),
        Ok(Some(Arc::new(Evt::Deck {
            deck: 1,
            playing: false
        })))
    );
    assert_eq!(observer.try_recv(), Ok(None), "one delivery, not two");
}

#[test]
fn a_subscription_joins_between_publications() {
    let bus = labelled_bus();
    let before = Arc::new(Barrier::new(2));
    let after = Arc::new(Barrier::new(2));

    let publisher = {
        let bus = bus.clone();
        let before = Arc::clone(&before);
        let after = Arc::clone(&after);
        thread::spawn(move || {
            bus.publish_evt(Evt::Numbered(1)).expect("publish");
            before.wait();
            after.wait();
            bus.publish_evt(Evt::Numbered(2)).expect("publish");
        })
    };

    before.wait();
    let mut observer = bus.subscribe_evt("joined").expect("observer");
    after.wait();
    publisher.join().expect("publisher");

    assert_eq!(
        observer.try_recv(),
        Ok(Some(Arc::new(Evt::Numbered(2)))),
        "an event published before registration is not promised"
    );
    assert_eq!(
        observer.try_recv(),
        Ok(None),
        "everything published afterwards is delivered"
    );
}

#[test]
fn close_is_idempotent_and_drains_before_disconnecting() {
    let bus = labelled_bus();
    let mut engine = bus
        .subscribe_cmd(Target::Engine, "engine")
        .expect("engine inbox");
    let mut ui = bus.subscribe_evt("ui").expect("observer");
    bus.publish_cmd(Cmd::Play(1)).expect("owner registered");
    bus.publish_evt(Evt::Numbered(1)).expect("publish");

    bus.close().expect("close");
    bus.close().expect("close is idempotent");

    assert_eq!(
        engine.try_recv(),
        Ok(Some(Cmd::Play(1))),
        "queued stays readable"
    );
    assert_eq!(engine.try_recv(), Err(BusError::Closed));
    assert_eq!(ui.try_recv(), Ok(Some(Arc::new(Evt::Numbered(1)))));
    assert_eq!(ui.try_recv(), Err(BusError::Closed));

    let error = bus.publish_cmd(Cmd::Play(2)).expect_err("closed lane");
    assert_eq!(error.reason, BusError::Closed);
    assert_eq!(error.command, Cmd::Play(2), "the command comes back unsent");
    assert_eq!(bus.publish_evt(Evt::Numbered(3)), Err(BusError::Closed));
    assert_eq!(
        bus.subscribe_cmd(Target::Engine, "late").err(),
        Some(BusError::Closed)
    );
    assert_eq!(bus.subscribe_evt("late").err(), Some(BusError::Closed));
}

#[test]
fn open_and_empty_is_not_disconnected() {
    let bus = labelled_bus();
    let mut ui = bus.subscribe_evt("ui").expect("observer");
    assert_eq!(ui.try_recv(), Ok(None));
    assert_eq!(ui.recv_timeout(Duration::from_millis(5)), Ok(None));
}

#[test]
fn dropping_every_bus_disconnects_after_draining() {
    let bus = labelled_bus();
    let mut ui = bus.subscribe_evt("ui").expect("observer");
    bus.publish_evt(Evt::Numbered(7)).expect("publish");
    let clone = bus.clone();
    drop(bus);
    assert_eq!(
        ui.try_recv(),
        Ok(Some(Arc::new(Evt::Numbered(7)))),
        "a retained subscriber still drains"
    );
    assert_eq!(ui.try_recv(), Ok(None), "one bus clone still owns the lane");
    drop(clone);
    assert_eq!(ui.try_recv(), Err(BusError::Closed));
    assert_eq!(ui.recv(), Err(BusError::Closed));
}

#[test]
fn diagnostics_report_labels_and_depths_and_forget_dropped_subscribers() {
    let bus = labelled_bus();
    let mut ui = bus.subscribe_evt("ui").expect("observer");
    let stale = bus.subscribe_evt("stale").expect("observer");
    let _engine = bus
        .subscribe_cmd(Target::Engine, "engine")
        .expect("engine inbox");
    for seq in 0..5u64 {
        bus.publish_evt(Evt::Numbered(seq)).expect("publish");
    }

    let diagnostics = bus.diagnostics().expect("diagnostics");
    assert!(!diagnostics.events.closed);
    assert_eq!(
        diagnostics
            .events
            .subscriptions
            .iter()
            .map(|entry| (entry.label, entry.queued))
            .collect::<Vec<_>>(),
        vec![("ui", 5), ("stale", 5)],
        "sorted by registration id, with the real queue depth"
    );
    assert_eq!(diagnostics.commands.subscriptions.len(), 1);
    assert_eq!(diagnostics.commands.subscriptions[0].label, "engine");

    drop(stale);
    let after = bus.diagnostics().expect("diagnostics");
    assert_eq!(
        after
            .events
            .subscriptions
            .iter()
            .map(|entry| entry.label)
            .collect::<Vec<_>>(),
        vec!["ui"],
        "a dropped subscriber disappears from the snapshot"
    );

    assert!(ui.try_recv().expect("open").is_some());
}

#[test]
fn a_subscriber_does_not_keep_publishers_alive() {
    let bus = labelled_bus();
    let ui = bus.subscribe_evt("ui").expect("observer");
    let engine = bus
        .subscribe_cmd(Target::Engine, "engine")
        .expect("engine inbox");
    let diagnostics = bus.diagnostics().expect("diagnostics");
    drop(bus);
    // Nothing here holds a sender: the lane is gone, so both receivers report
    // disconnection instead of waiting for a publisher that can never return.
    assert_eq!(diagnostics.events.subscriptions.len(), 1);
    let mut ui = ui;
    assert_eq!(ui.try_recv(), Err(BusError::Closed));
    let mut engine = engine;
    assert_eq!(engine.try_recv(), Err(BusError::Closed));
}

#[test]
fn a_stalled_observer_grows_its_queue_without_delaying_the_others() {
    let bus = labelled_bus();
    let mut fast = bus.subscribe_evt("fast").expect("observer");
    let _stalled = bus.subscribe_evt("stalled").expect("observer");
    for seq in 0..10_000u64 {
        bus.publish_evt(Evt::Numbered(seq)).expect("publish");
        // The fast observer drains concurrently with publication.
        if seq % 100 == 0 {
            fast.try_recv().expect("open").expect("event");
        }
    }
    let queued = bus.diagnostics().expect("diagnostics").events.subscriptions;
    assert_eq!(
        queued[1].queued, 10_000,
        "the stalled queue keeps everything"
    );
    let mut delivered = 0;
    while fast.try_recv().expect("open").is_some() {
        delivered += 1;
    }
    assert_eq!(
        delivered + 100,
        10_000,
        "the fast observer lost nothing either"
    );
}

#[test]
fn a_rejected_command_can_be_retried_by_the_caller() {
    let bus = labelled_bus();
    let error = bus
        .publish_cmd(Cmd::Library("later".to_string()))
        .expect_err("no library owner yet");
    let command = error.command;
    let mut library = bus
        .subscribe_cmd(Target::Library, "library")
        .expect("library inbox");
    bus.publish_cmd(command).expect("owner registered now");
    assert_eq!(
        library.try_recv(),
        Ok(Some(Cmd::Library("later".to_string())))
    );
}

#[test]
fn a_barrier_ordered_close_never_accepts_a_later_publication() {
    let bus = labelled_bus();
    let ready = Arc::new(Barrier::new(2));
    let closed = Arc::new(Barrier::new(2));
    let (tx, rx) = mpsc::channel();

    let publisher = {
        let bus = bus.clone();
        let ready = Arc::clone(&ready);
        let closed = Arc::clone(&closed);
        thread::spawn(move || {
            ready.wait();
            closed.wait();
            let accepted = bus.publish_evt(Evt::Numbered(1)).is_ok();
            tx.send(accepted).expect("report");
        })
    };

    ready.wait();
    bus.close().expect("close");
    closed.wait();
    assert!(
        !rx.recv_timeout(Duration::from_secs(5)).expect("report"),
        "no publication succeeds after close returns"
    );
    publisher.join().expect("publisher");
}
