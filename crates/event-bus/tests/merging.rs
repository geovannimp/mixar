//! Pending-queue merging, asserted on delivered values and order.

#![allow(clippy::unwrap_used)]

use event_bus::bus_events;
use event_bus::Entity;
use event_bus::EventBus;
use event_bus::Part;
use event_bus::Subscriber;
use std::time::Duration;

bus_events! {
    Volume:   Update, Entity::Deck(*deck),        Part::Whole,           { deck: u16, volume: f32 };
    Created:  Create, Entity::Library,                Part::Whole,           { name: String };
    Jog:      Delta,  Entity::Deck(*deck),        Part::Whole,           { deck: u16, delta: i32 }, fold: delta;
    Play:     Toggle, Entity::Deck(*deck),        Part::Whole,           { deck: u16 };
    Notice:   Signal, Entity::Ui,                     Part::Whole,           { message: String };
    Band:     Update, Entity::Deck(*deck),        Part::Index(*band as u8), { deck: u16, band: EqBand, gain: f32 };
    FxParam:  Update, Entity::Mixer,                  Part::Pair(*slot, *param), { slot: u8, param: u8, value: f32 };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EqBand {
    Low,
    Mid,
}

fn bus() -> EventBus<Event> {
    // Large enough that no test is measuring channel overflow.
    EventBus::<Event>::new(512, 512)
}

fn drain(sub: &mut Subscriber<Event>) -> Vec<Event> {
    let mut out = Vec::new();
    while let Some(event) = sub.try_recv() {
        out.push(event);
    }
    out
}

fn volume(deck: u16, volume: f32) -> Event {
    Event::Volume(Volume { deck, volume })
}

fn jog(deck: u16, delta: i32) -> Event {
    Event::Jog(Jog { deck, delta })
}

fn play(deck: u16) -> Event {
    Event::Play(Play { deck })
}

fn notice(message: &str) -> Event {
    Event::Notice(Notice {
        message: message.to_string(),
    })
}

#[test]
fn create_keeps_the_first_event() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    bus.publish_cmd(Event::Created(Created {
        name: "bank".to_string(),
    }))
    .unwrap();
    bus.publish_cmd(Event::Created(Created {
        name: "bank".to_string(),
    }))
    .unwrap();

    let delivered = drain(&mut sub);
    assert_eq!(delivered.len(), 1, "second create must be dropped");
    assert_eq!(
        delivered[0],
        Event::Created(Created {
            name: "bank".to_string()
        })
    );
}

#[test]
fn update_keeps_the_newest_event() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    bus.publish_cmd(volume(1, 0.1)).unwrap();
    bus.publish_cmd(volume(1, 0.2)).unwrap();
    bus.publish_cmd(volume(1, 0.3)).unwrap();

    assert_eq!(drain(&mut sub), vec![volume(1, 0.3)]);
}

#[test]
fn delta_sums_into_one_event() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    bus.publish_cmd(jog(0, 1)).unwrap();
    bus.publish_cmd(jog(0, 3)).unwrap();

    assert_eq!(drain(&mut sub), vec![jog(0, 4)]);
}

#[test]
fn adjacent_toggles_cancel() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    bus.publish_cmd(play(2)).unwrap();
    bus.publish_cmd(play(2)).unwrap();

    assert_eq!(drain(&mut sub), Vec::new());
}

#[test]
fn odd_toggle_run_leaves_one() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    bus.publish_cmd(play(2)).unwrap();
    bus.publish_cmd(play(2)).unwrap();
    bus.publish_cmd(play(2)).unwrap();

    assert_eq!(drain(&mut sub), vec![play(2)]);
}

#[test]
fn toggles_separated_by_another_event_do_not_cancel() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    bus.publish_cmd(play(2)).unwrap();
    bus.publish_cmd(notice("between")).unwrap();
    bus.publish_cmd(play(2)).unwrap();

    assert_eq!(drain(&mut sub), vec![play(2), notice("between"), play(2)]);
}

#[test]
fn signals_are_never_folded() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    bus.publish_cmd(notice("one")).unwrap();
    bus.publish_cmd(notice("two")).unwrap();

    assert_eq!(drain(&mut sub), vec![notice("one"), notice("two")]);
}

#[test]
fn eq_bands_keep_separate_slots() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    bus.publish_cmd(Event::Band(Band {
        deck: 1,
        band: EqBand::Low,
        gain: 0.5,
    }))
    .unwrap();
    bus.publish_cmd(Event::Band(Band {
        deck: 1,
        band: EqBand::Mid,
        gain: -0.5,
    }))
    .unwrap();

    assert_eq!(
        drain(&mut sub),
        vec![
            Event::Band(Band {
                deck: 1,
                band: EqBand::Low,
                gain: 0.5
            }),
            Event::Band(Band {
                deck: 1,
                band: EqBand::Mid,
                gain: -0.5
            }),
        ]
    );
}

#[test]
fn fx_params_key_on_slot_and_param() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    let param = |slot, param, value| Event::FxParam(FxParam { slot, param, value });
    bus.publish_cmd(param(0, 1, 0.4)).unwrap();
    bus.publish_cmd(param(0, 2, 0.7)).unwrap();
    bus.publish_cmd(param(1, 1, 0.9)).unwrap();

    // Same slot, different param: separate slots. Different slot, same param: separate too.
    assert_eq!(
        drain(&mut sub),
        vec![param(0, 1, 0.4), param(0, 2, 0.7), param(1, 1, 0.9)]
    );
}

#[test]
fn update_keeps_position_and_takes_newest_value() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    bus.publish_cmd(volume(0, 0.1)).unwrap();
    bus.publish_cmd(notice("x")).unwrap();
    bus.publish_cmd(volume(0, 0.9)).unwrap();

    assert_eq!(
        drain(&mut sub),
        vec![volume(0, 0.9), notice("x")],
        "the update rewrites slot 0 in place, so it stays ahead of the signal"
    );
}

#[test]
fn decks_are_independent_keys() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    bus.publish_cmd(volume(0, 0.1)).unwrap();
    bus.publish_cmd(volume(1, 0.2)).unwrap();

    assert_eq!(drain(&mut sub), vec![volume(0, 0.1), volume(1, 0.2)]);
}

#[test]
fn hundred_updates_across_four_keys_collapse() {
    let bus = bus();
    let mut sub = bus.subscribe_cmd().unwrap();
    for round in 0..25 {
        for deck in 0..4u16 {
            bus.publish_cmd(volume(deck, round as f32)).unwrap();
        }
    }

    assert_eq!(
        drain(&mut sub),
        vec![
            volume(0, 24.0),
            volume(1, 24.0),
            volume(2, 24.0),
            volume(3, 24.0),
        ]
    );
}

#[test]
fn filtered_subscription_never_folds_toggles() {
    let bus = bus();
    let mut sub = bus.subscribe_evt_topic(Topic::Play).unwrap();
    bus.publish_evt(play(3)).unwrap();
    bus.publish_evt(play(3)).unwrap();

    assert_eq!(
        drain(&mut sub),
        vec![play(3), play(3)],
        "the events separating two toggles are filtered out, so adjacency is unknowable"
    );
}

#[test]
fn recv_timeout_reports_an_empty_queue_as_none() {
    let bus = bus();
    let mut sub = bus.subscribe_evt().unwrap();

    assert_eq!(
        sub.recv_timeout(Duration::from_millis(5)).unwrap(),
        None,
        "a timeout is not an error"
    );
}
