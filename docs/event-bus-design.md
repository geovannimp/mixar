# Event Bus Design

**Spec refs:** `docs/deck-spec.md` §7–§9; `docs/tech-spec.md`
**Status:** Shipped — one typed command/event pair for engine, library and controller

## Goal

One process-wide bus carrying two closed, typed directions: commands that move to
exactly one owning worker, and events that are shared with every observer.

Payloads are ordinary Rust structs — no wire format — and events reach Dart as a
generated sealed class. Delivery semantics are stated below and covered by
`crates/event-bus/tests/delivery.rs`; the crate does not promise more than that.

## Requirements

| Decision | Choice |
|----------|--------|
| Topology | One `EventBus<Command, Event>`: two lanes, one command inbox per target, one queue per event observer |
| Transport | `crossbeam-channel` unbounded queues; routing, lifecycle and diagnostics are owned by `crates/event-bus` |
| Payload | One concrete struct per message (`bus-api/src/messages.rs`), closed `Command` + `Event` enums |
| Direction | Separate enums: publishing an event as a command does not compile |
| Ownership | Exactly one live command inbox per `CommandTarget`; a second registration fails |
| Fan-out | Every matching event is delivered to every live observer — no folding, replacement or drops |
| Publisher contract | Enqueue only: never blocks on a full queue, never runs a handler; a refused command is returned to the caller |
| Ordering | One mutex per lane gives a total *enqueue* order inside that lane |
| Host role | Encode/decode nothing; forward typed events to Dart |
| Flutter stream | One `BusTransport.subscribeEvents()`, registered before its forwarder starts |
| Domain failures | `Event::Error` / `Event::Notice` on the event lane |
| Control path | Dedicated control thread: bounded batches on a 33 ms deadline; audio callback never touches the bus |

## Crates

| Crate | Owns |
|-------|------|
| `event-bus` | Domain-free lanes: `EventBus<C, E>`, `Subscriber<M>`, `Message`, `BusError`, `PublishReport`, `BusDiagnostics`. No knowledge of engine, library or MIDI. |
| `bus-api` | Every payload struct, the `library.rs` records, and the closed `Command`/`CommandTarget` and `Event`/`EventTopic` pairs. |
| `engine-core` | Engine command handling and event publication; control thread; library acknowledgement path. |
| `library` | Library worker command handling and event publication (analysis, stems, hot cues, loops, beat grid, history). |
| `controller` | MIDI mapping: resolves a control to a command or an event and publishes it on the matching lane; mirrors engine events into LEDs. |
| `host-flutter` | `BusTransport` (the only bus handle Flutter holds), typed transport methods, FRB mirrors. |

## Message model

`crates/bus-api/src/messages.rs` holds one ordinary struct per payload. Two
closed enums decide which direction a payload travels:

```rust
pub enum Command {                     pub enum Event {
    Play(Play),                            DeckUpdated(DeckUpdated),
    SetEqBand(SetEqBand),                  DeckPosition(DeckPosition),
    SaveTrackHotCue(SaveTrackHotCue),      HotCuesChanged(HotCuesChanged),
    …                                      …
}                                      }
```

94 commands and 19 events. `Command::target()` is an exhaustive match with no
wildcard: nine commands (`AnalyzeTrack`, `GenerateStems`, `RefreshTrack`,
`SaveBeatGrid`, `SaveTrackCue`, `SaveTrackHotCue`, `DeleteTrackHotCue`,
`SaveTrackLoop`, `DeleteTrackLoop`) belong to the library, the other 85 to the
engine. `Event::topic()` is the matching exhaustive map onto `EventTopic`.

Both worker dispatchers are exhaustive too — the engine lists the nine library
commands it will never receive, and the library worker lists the 85 engine
commands it will never receive. A new variant therefore has to be classified in
four places instead of silently falling into a catch-all.

`Navigate`, `Focus` and `Load` are *events*: they describe a UI action the host
performs, so they travel on the event lane and never on the command lane.

## Delivery semantics

- **Commands.** `publish_cmd` computes the target, then moves the command into
  that target's queue under the command lane's lock. Missing owner, closed lane,
  poisoned lane: the command comes back in `CommandPublishError` — nothing is
  dropped or retried on the caller's behalf. A second live owner of a target is
  `AlreadySubscribed`; dropping an owner frees the target, and a delayed old
  destructor cannot remove its replacement (cleanup matches its own id).
- **Events.** `publish_evt` wraps the event in one `Arc`, then, under the event
  lane's lock, enqueues that `Arc` to every matching observer. `PublishReport`
  reports `matched` / `delivered` / `disconnected`. No observers is a successful
  zero report. Topic-filtered subscriptions select by `EventTopic`; an empty
  filter is refused, duplicates collapse.
- **Ordering.** Each lane has one mutex held across matching and enqueueing, so
  concurrent publishers cannot interleave: every event observer sees the same
  sequence, and a command owner sees its target's projection of command order.
  There is no order *between* lanes, no cross-worker processing order, and no
  atomic visibility of an entire fan-out — a subscriber may consume before the
  last observer has been enqueued.
- **Lifecycle.** Queues are unbounded, so publication never fails for lack of
  space. Dropping the last `EventBus` clone (or `close()`, which is idempotent)
  detaches the senders after the queued messages stay readable; every receive
  method then reports `Closed`. `try_recv` distinguishes an open empty queue
  (`Ok(None)`) from a disconnected one (`Err(Closed)`, after draining).
- **Poisoning.** A panic inside a lane latches that lane fail-closed: the
  registry is detached outside the lock, later operations report `Poisoned`, and
  a publish that unwound is never silently retried. Rejected commands, retained
  event handles and retired registrations are dropped only after the lock is
  released, so no destructor runs under a lane lock.
- **Diagnostics.** `diagnostics()` copies labels, ids and `Sender::len` per lane.
  Depths are point-in-time approximations — they show unbounded growth, they do
  not cap it.

### What the bus does not do

- It is **not realtime-safe** and **not non-blocking**: a mutex acquisition and an
  allocation can block, and a stalled consumer grows memory until the process
  dies. Allocation failure is not recoverable here.
- It has **no replay, persistence, processing acknowledgement or retry**. An
  in-memory enqueue is not a completion, and a command that no owner receives is
  reported, never queued for a later owner.
- It does **not** cap memory or reject overload at the bus level; backpressure is
  the consumer's business.

## Control thread

- The control thread owns the engine inbox (`CommandTarget::Engine`, label
  `"engine"`) and registers it before startup reports ready; a second worker for
  the same target fails startup visibly.
- Each pass is bounded work, not "everything queued": up to
  `MAX_COMMANDS_PER_PASS` (64) commands, then up to
  `MAX_LIBRARY_EVENTS_PER_PASS` (64) library acknowledgements, each batch with its
  own 2 ms budget. Leftover work stays queued for the next pass, so a sustained
  producer cannot amplify telemetry or delay shutdown.
- One tick per 33 ms deadline — not one per command, and no catch-up burst. When
  both batches are empty the thread blocks on the command inbox until that
  deadline, so a library acknowledgement arriving while idle is serviced by the
  next tick.
- The engine's library-acknowledgement subscription is filtered to
  `HotCuesChanged` and `BeatGridChanged`; history filters to `DeckUpdated` and
  `EngineSnapshot`; the controller mirror filters to `DeckUpdated`,
  `DeckPosition`, `DeckLevels`, `EngineSnapshot` and `HotCuesChanged`. Flutter
  stays unfiltered because it renders everything.
- Worker stop stays out-of-band (an atomic flag plus joins); shutdown is never
  enqueued behind a backlog. A single expensive handler cannot be preempted —
  long library/stem work stays on its own thread.
- Audio/producer threads never publish; they hand state to the control thread,
  which turns it into events at tick rate (≤ ~60 Hz).

## Flutter

```text
engine / library / controller ──publish_evt──▶ EventBus<Command, Event>
                                                    │ subscribe_evt("flutter")
                                                    ▼
                                         BusTransport forwarder
                                                    │ fan-out (registry unlocked)
                                                    ▼
                                    Dart StreamSink<Event> per subscriber
```

`BusTransport::subscribe_events` registers the Dart sink *before* starting a new
forwarder, so there is no window where the Rust subscription is live and the
Dart stream is not: an event published after the call returns is delivered once.
A startup failure rolls back only the sink that call inserted. The forwarder
converts the shared `Arc<Event>` into an owned event only where chrome decoration
and FRB encoding need it; `Arc` never crosses the bridge.

The FRB surface is event-only: `Command`, `CommandTarget`, `EventTopic` and the
bus types are not mirrored. `crates/host-flutter/scripts/gen-event-mirrors.py`
generates the mirrors, and `--check` fails CI on a stale file.

## Removed

- `omnibus` as the production transport (kept only as a benchmark baseline dev
  dependency) and its bounded per-subscriber queues.
- Subscriber-side folding: the `bus_events!` table, `Policy`, `EventKey`,
  `EventSpec`, `BusMessage` and the pending-queue merge algebra.
- Capacity-taking constructors (`EventBus::new(cmd_cap, evt_cap)`).
- The single `Event` enum that carried both directions, and the `Topic` enum that
  went with it.
- Receive-side state that could hide a disconnection (`absorb`, indexed pending
  queue, silent `Option`-only `try_recv`).

An event and a command are now different types, so "publish an event onto the
command lane" is a compile error rather than a dispatch arm nobody executes.

## Verification

| Suite | What it establishes |
|-------|--------------------|
| `event-bus/tests/delivery.rs` | 10,000-message lossless delivery and order, no folding, per-target inbox ownership, `Arc` sharing, concurrent-publisher agreement, filtered projection, close/drain/disconnect, diagnostics |
| `event-bus/src/channel.rs` unit tests | Panic mid-fan-out fails the lane closed, a poisoned lane returns the unsent command, retired messages are dropped outside the lane locks |
| `bus-api/tests/schema.rs` | 94 commands / 19 events, nine library targets, topic mapping, `From` conversions, exhaustive owner and topic maps |
| `bus-api` doctests | `publish_cmd(Event::…)` and `publish_evt(Command::…)` do not compile |
| `engine-core` control tests | 1,000 queued commands produce 16 bounded passes and no tick storm; a due tick still runs behind a full library queue; separate per-batch budgets; shutdown stops a batch |
| `engine-core/tests/bus_control_liveness.rs` | Egress keeps flowing under a saturated command lane, and stop/join returns inside a 5 s watchdog |
| `host-flutter` tests | Play/Pause/Play ends playing, a library save reaches every observer and the engine acknowledgement path, first-sink registration, rollback of a failed forwarder start, duplicate worker startup fails, publishing before an owner registers is refused |
| `host-flutter/scripts/test_gen_event_mirrors.py` | Mirror generation rejects malformed schemas, keeps the surface event-only, and `--check` is deterministic |
| `event-bus/benches/fanout.rs` | Publish/enqueue throughput at 1/4 publishers × 1/4/8 observers × 32 B/4 KiB, targeted command delivery, topic filtering, a paused observer's 10,000-message backlog, and p50/p95/p99 end-to-end latency |

Performance is measured, not promised: the benchmark compares against the
replaced transport under equal delivery counts (its capacity raised to 16,384 so
the comparison is lossless) and reports a paused observer's queue depth instead of
hiding overload. The mutex-based ordering architecture is kept even where it costs
multi-publisher throughput; the measured cost is a result, not a reason to weaken
ordering or drop messages.
