# Event Bus Design

**Spec refs:** `docs/deck-spec.md` §7–§9; `docs/tech-spec.md`
**Status:** Shipped — one typed bus for engine, library and controller

## Goal

One process-wide command/event bus carrying one closed set of typed events. The
engine, the library worker and the controller publish on the same pair of buses,
and Flutter subscribes exactly once. Payloads are plain Rust structs — no wire
format — and reach Dart as a generated sealed class.

Earlier designs had a bus per domain, a MessagePack envelope
(`Origin` + `Kind` + body bytes) and a hand-written dedup pass in the Flutter
host. All three are gone: see [Removed](#removed).

## Requirements

| Decision | Choice |
|----------|--------|
| Topology | Two buses per process: **cmd** (ingress) and **evt** (egress) |
| Bus crate | [`omnibus`](https://docs.rs/omnibus/latest/omnibus/) `0.1`, wrapped by `crates/event-bus` |
| Payload | Concrete struct per event, one closed `Event` enum, one `Topic` enum |
| Publisher contract | Fire-and-forget, publish order preserved, never blocks |
| Dedup | Per-subscriber pending queue, folded per event **policy** |
| Host role | Encode/decode nothing; forward typed events to Dart |
| Flutter stream | One `BusTransport.subscribeEvents()` |
| Domain failures | `Event::Error` / `Event::Notice` on the evt bus |
| Control path | Dedicated control thread drains cmd; audio callback never touches the bus |

## Crates

| Crate | Owns |
|-------|------|
| `event-bus` | Domain-free wrapper: `EventBus<M>`, `Subscriber<M>`, `Policy`, `EventKey`, the `bus_events!` table macro. No knowledge of engine, library or MIDI. |
| `bus-api` | Every payload struct, the `library.rs` records the library events carry, and the single `Event`/`Topic` pair generated from the event table. |
| `engine-core` | Engine cmd handling and evt publication; control thread; library bridge. |
| `library` | Library worker cmd handling and evt publication (analysis, stems, hot cues, loops, beat grid, history). |
| `controller` | MIDI mapping: resolves a control to an `Event` and publishes it; mirrors engine evt into LEDs. |
| `host-flutter` | `BusTransport` (the only bus handle Flutter holds), typed transport methods, FRB mirrors. |

## Event model

`crates/bus-api/src/events.rs` is the single source of truth. One row per
event:

```rust
bus_events! {
    SetEqBand: Update, Entity::Deck(*deck), Part::Index(*band as u8),
        { deck: u16, band: EqBand, gain: f32, soft_takeover: bool };
    JogTurn:   Delta,  Entity::Deck(*deck), Part::Whole,
        { deck: u16, delta: i32 }, fold: delta;
}
```

The macro emits the `Topic` enum, one payload struct per row, `EventSpec` for
each payload (policy, key, fold), and the closed `Event` enum with
`BusMessage`/`From` impls. Adding an event means adding a row; there is no
second place to register it.

Commands carry the entity they target (`deck: u16`, `track_id: String`). The old
`Origin` enum existed only to say that; it is gone.

### Policies

Deduplication happens in the subscriber, on events that are still undelivered
when it next reads. Each event declares how it folds:

| Policy | Meaning | Used for |
|--------|---------|----------|
| `Create` | keep the first pending event for a key | one-time introductions |
| `Update` | newest wins, position kept | absolute targets (`Set*`, full snapshots) |
| `Delta` | pending value absorbs the new event | relative motion (`JogTurn`, `BeatJump`, `Navigate`, `Focus`) |
| `Toggle` | an adjacent pair cancels | boolean flips (`TogglePlay`) |
| `Signal` | never merged, never keyed | pad press/release edges, `Error`, `Notice` |

Classification rules, applied in order: relative amount → `Delta`; flips a
boolean → `Toggle`; introduces a named thing → `Create`; payload fully describes
the target → `Update`; an occurrence or edge → `Signal`.

### Keys

The pending-queue key is `EventKey { entity, topic, part }`. The `part`
(`Whole`, `Index(u8)`, `Pair(u8, u8)`, `Name`) is what keeps two setters for
different slots, bands or params from cancelling each other: `SetEqBand(low)`
and `SetEqBand(mid)` are two keys, so both land.

`Toggle` folds only when the pair is adjacent in the same subscriber, and only on
an unfiltered subscription: on a topic-filtered one, the events that would have
separated two toggles were filtered out, so a pair would look adjacent when it
is not.

## Control thread

- One thread drains the cmd bus (`recv_timeout(33ms)`), applies each command
  under the engine lock, and publishes the resulting `DeckUpdated`,
  `DeckPosition`, `DeckLevels`, `EngineSnapshot` or `Error`.
- Audio/producer threads never publish; they hand state to the control thread,
  which turns it into events at tick rate (≤ ~60 Hz).
- High-rate egress (`DeckPosition`, `DeckLevels`) is ordinary `Update` events: if
  a consumer falls behind, the pending queue keeps the newest one per deck.

## Flutter

```text
engine / library / controller ──publish──▶ EventBus<Event>
                                              │ subscribe_evt (one)
                                              ▼
                                   BusTransport forwarder
                                              │ fan-out
                                              ▼
                        Stream<Event> (freezed sealed class) ──▶ providers
```

- `BusTransport::open()` owns the process bus; `EngineTransport`,
  `LibraryTransport` and `ControllerTransport` are built on it.
- `BusTransport::subscribe_events()` is the only event stream; several Dart
  sinks may be live at once and a dead sink is dropped on its first send error.
- Dart `applyEngineEvent` reads a typed event: `DeckUpdated` is a full deck
  snapshot, so it replaces that deck's state instead of patching fields.
- Host-only data that is not engine state (ephemeral sampler pad chrome) is
  stamped onto `DeckUpdated` by the host forwarder, not carried through the bus.

### FRB and external types

FRB only scans `host-flutter`, so types defined in `bus-api` would arrive in
Dart as opaque handles. `crates/host-flutter/src/api/event_mirrors.rs` declares
`#[frb(mirror(...))]` for each of them — generated from the event table by
`crates/host-flutter/scripts/gen-event-mirrors.py`, and validated by codegen
(field drift is a codegen error, not a silent divergence). `mirror()` takes a
single-segment name, so the module imports the types and mirrors them by short
name.

## Testing

| Layer | Check |
|-------|-------|
| `event-bus` | `tests/merging.rs`: one test per policy, asserting delivered values and order |
| `engine-core` | Headless: publish a command, assert the typed event it produces |
| `library` | Worker tests: cmd → evt with the same payload |
| `host-flutter` | Transport tests on a real bus; `integration_test/bus_smoke_test.dart` drives the real engine through FRB |
| Flutter | Widget tests feed typed events into `engineUiProvider` through the bus bootstrap |

## Removed

- `Origin`, `Kind`, `CmdBody`, `EvtBody`, `WireMessage` and the MessagePack
  encode/decode helpers (`docs/engine-msgpack-wire-design.md`).
- The `revision` counter — an event is the unit of truth; `EngineSnapshot` is
  the full hydrate path.
- `EngineBuses` / `LibraryBuses` as bus owners, and the per-domain `EvtReceiver`
  types (`docs/engine-session-bus-design.md`).
- The three Flutter event models (`EngineEvt`, `LibraryEvt`, `ControllerEvt`)
  and the host-side coalescing forwarder.

## Non-goals

- Multi-process or network transport: the bus is in-process by construction.
- Replacing the MIDI pump: the controller keeps its 16 ms CC rate limiter
  (`session.rs`) because it filters hardware noise *before* publish; the pending
  queue is a second, generic layer.
- Cross-topic folding: the queue cannot reason about a `Toggle` separated from
  its pair by a different topic, so `Play` / `TogglePlay` fold only within their
  own keys.
