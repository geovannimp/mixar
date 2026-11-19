//! Fan-out benchmark: our lanes against the transport this crate replaced.
//!
//! `omnibus` is a dev-dependency here *only* as a baseline: it is the bounded,
//! lossy transport the old bus was built on, and the numbers are only honest if
//! it does the same work. The legacy subscriber therefore clones each payload on
//! receive (its payload was shared through the channel), and the message stream
//! uses distinct signals so no folding can make it look faster by delivering
//! fewer messages. Its capacity is deliberately large (16,384 per observer) so a
//! lossless comparison is possible; that is not a production fix.
//!
//! Publication is timed on its own: the receive/drain side, the delivery-count
//! assertions and the paused-observer depth report all happen outside the timed
//! region. Per-message receive latency is reported separately (p50/p95/p99), so
//! a fast publish never hides a slow receive.
//!
//! Run with `cargo bench -p event-bus --bench fanout`. FRB/Dart encoding is not
//! part of this transport microbenchmark.

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use event_bus::EventBus;
use event_bus::Message;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

/// Messages one publisher sends per batch.
const MESSAGES_PER_PUBLISHER: usize = 1_024;
/// The replaced bus sized every subscriber queue at this depth for the baseline.
const LEGACY_CAPACITY: usize = 16_384;
const SMALL_PAYLOAD_BYTES: usize = 32;
const BIG_PAYLOAD_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Topic {
    Signal,
    Levels,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Target {
    Engine,
}

/// An owned payload of a stated size.
#[derive(Clone, Debug, PartialEq)]
enum Payload {
    Small([u8; SMALL_PAYLOAD_BYTES]),
    Big(Box<[u8]>),
}

impl Payload {
    fn small() -> Self {
        Self::Small([7; SMALL_PAYLOAD_BYTES])
    }

    fn big() -> Self {
        Self::Big(vec![7; BIG_PAYLOAD_BYTES].into_boxed_slice())
    }

    fn bytes(&self) -> usize {
        match self {
            Self::Small(_) => SMALL_PAYLOAD_BYTES,
            Self::Big(bytes) => bytes.len(),
        }
    }
}

/// Two distinct event shapes, so nothing can be folded into one slot.
///
/// `Clone` is only here because the replaced transport stores `T: Clone` and
/// clones on receive; the owned bus itself never clones an event.
#[derive(Clone, Debug, PartialEq)]
enum Evt {
    Signal { seq: u64, payload: Payload },
    Levels { seq: u64, payload: Payload },
}

impl Evt {
    fn new(seq: u64, big: bool) -> Self {
        let payload = if big {
            Payload::big()
        } else {
            Payload::small()
        };
        if seq.is_multiple_of(2) {
            Self::Signal { seq, payload }
        } else {
            Self::Levels { seq, payload }
        }
    }
}

impl Evt {
    fn bytes(&self) -> usize {
        match self {
            Self::Signal { payload, .. } | Self::Levels { payload, .. } => payload.bytes(),
        }
    }
}

impl Message for Evt {
    type Topic = Topic;

    fn topic(&self) -> Topic {
        match self {
            Self::Signal { .. } => Topic::Signal,
            Self::Levels { .. } => Topic::Levels,
        }
    }
}

#[derive(Debug, PartialEq)]
struct Volume(f32);

impl Message for Volume {
    type Topic = Target;

    fn topic(&self) -> Target {
        Target::Engine
    }
}

/// A message that carries the instant it was published.
#[derive(Debug, PartialEq)]
struct Timed {
    seq: u64,
    sent: Instant,
}

impl Message for Timed {
    type Topic = Topic;

    fn topic(&self) -> Topic {
        Topic::Levels
    }
}

fn drain_ours(rx: &mut event_bus::Subscriber<Arc<Evt>>) -> usize {
    let mut received = 0;
    loop {
        match rx.try_recv() {
            Ok(Some(_event)) => received += 1,
            Ok(None) => return received,
            Err(error) => panic!("lane disconnected while draining: {error}"),
        }
    }
}

fn bench_ours(c: &mut Criterion) {
    let mut group = c.benchmark_group("ours/publish-enqueue");
    for publishers in [1usize, 4] {
        for observers in [1usize, 4, 8] {
            for big in [false, true] {
                let payload = if big {
                    Payload::big()
                } else {
                    Payload::small()
                };
                let bytes = payload.bytes();
                let total = publishers * MESSAGES_PER_PUBLISHER;
                group.throughput(Throughput::Bytes((bytes * total) as u64));
                group.bench_with_input(
                    BenchmarkId::new(
                        format!("observers-{observers}"),
                        format!("publishers-{publishers}/payload-{bytes}"),
                    ),
                    &(publishers, observers, big),
                    |b, &(publisher_count, observer_count, use_big)| {
                        // Only publication is timed: the drain (and the delivery
                        // count assertion) happens outside the measured region, so
                        // this is not a receive benchmark.
                        b.iter_custom(|iterations| {
                            let mut elapsed = Duration::ZERO;
                            for _ in 0..iterations {
                                let bus = EventBus::<Volume, Evt>::new();
                                let mut subscribers: Vec<_> = (0..observer_count)
                                    .map(|index| {
                                        let label: &'static str =
                                            ["ui", "history", "controller", "x"][index % 4];
                                        bus.subscribe_evt(label).expect("observer")
                                    })
                                    .collect();

                                let started = Instant::now();
                                let handles: Vec<_> = (0..publisher_count)
                                    .map(|publisher| {
                                        let bus = bus.clone();
                                        std::thread::spawn(move || {
                                            for index in 0..MESSAGES_PER_PUBLISHER {
                                                let seq = (publisher * MESSAGES_PER_PUBLISHER
                                                    + index)
                                                    as u64;
                                                bus.publish_evt(Evt::new(seq, use_big))
                                                    .expect("publish");
                                            }
                                        })
                                    })
                                    .collect();
                                for handle in handles {
                                    handle.join().expect("publisher");
                                }
                                elapsed += started.elapsed();

                                for subscriber in &mut subscribers {
                                    let received = drain_ours(subscriber);
                                    assert_eq!(
                                        received, total,
                                        "every observer receives every message"
                                    );
                                }
                            }
                            elapsed
                        });
                    },
                );
            }
        }
    }
    group.finish();
}

/// The replaced transport: bounded channel per subscriber, payload cloned on read.
fn bench_legacy(c: &mut Criterion) {
    let mut group = c.benchmark_group("legacy/publish-enqueue");
    for publishers in [1usize, 4] {
        for observers in [1usize, 4, 8] {
            for big in [false, true] {
                let payload = if big {
                    Payload::big()
                } else {
                    Payload::small()
                };
                let bytes = payload.bytes();
                let total = publishers * MESSAGES_PER_PUBLISHER;
                group.throughput(Throughput::Bytes((bytes * total) as u64));
                group.bench_with_input(
                    BenchmarkId::new(
                        format!("observers-{observers}"),
                        format!("publishers-{publishers}/payload-{bytes}"),
                    ),
                    &(publishers, observers, big),
                    |b, &(publisher_count, observer_count, use_big)| {
                        b.iter_custom(|iterations| {
                            let mut elapsed = Duration::ZERO;
                            for _ in 0..iterations {
                                let bus =
                                    omnibus::Bus::<u8, Topic, Evt>::with_capacity(LEGACY_CAPACITY);
                                let mut receivers: Vec<_> = (0..observer_count)
                                    .map(|_| {
                                        bus.subscribe(omnibus::Filter::Any, omnibus::Filter::Any)
                                            .expect("subscriber")
                                    })
                                    .collect();

                                let started = Instant::now();
                                let handles: Vec<_> = (0..publisher_count)
                                    .map(|publisher| {
                                        let bus = bus.clone();
                                        std::thread::spawn(move || {
                                            for index in 0..MESSAGES_PER_PUBLISHER {
                                                let seq = (publisher * MESSAGES_PER_PUBLISHER
                                                    + index)
                                                    as u64;
                                                let event = Evt::new(seq, use_big);
                                                let topic = event.topic();
                                                bus.publish(omnibus::Event::new(0, topic, event))
                                                    .expect("publish");
                                            }
                                        })
                                    })
                                    .collect();
                                for handle in handles {
                                    handle.join().expect("publisher");
                                }
                                elapsed += started.elapsed();

                                for receiver in &mut receivers {
                                    let mut received = 0;
                                    loop {
                                        // Zero timeout: drain without sleeping.
                                        match receiver.recv_timeout(Duration::ZERO) {
                                            Ok(Some(event)) => {
                                                // What the old subscriber did: own the payload.
                                                let cloned = event.payload().clone();
                                                assert!(cloned.bytes() >= SMALL_PAYLOAD_BYTES);
                                                received += 1;
                                            }
                                            Ok(None) => break,
                                            Err(error) => panic!("bus error: {error}"),
                                        }
                                    }
                                    assert_eq!(
                                        received, total,
                                        "the baseline is lossless at this capacity"
                                    );
                                }
                            }
                            elapsed
                        });
                    },
                );
            }
        }
    }
    group.finish();
}

fn bench_command_delivery(c: &mut Criterion) {
    let mut group = c.benchmark_group("ours/command-delivery");
    group.throughput(Throughput::Elements(MESSAGES_PER_PUBLISHER as u64));
    group.bench_function("one-owner", |b| {
        b.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                let bus = EventBus::<Volume, Evt>::new();
                let mut owner = bus
                    .subscribe_cmd(Target::Engine, "engine")
                    .expect("engine inbox");

                let started = Instant::now();
                for _ in 0..MESSAGES_PER_PUBLISHER {
                    bus.publish_cmd(Volume(0.5)).expect("owner registered");
                }
                elapsed += started.elapsed();

                let mut received = 0;
                while let Ok(Some(_command)) = owner.try_recv() {
                    received += 1;
                }
                assert_eq!(received, MESSAGES_PER_PUBLISHER);
            }
            elapsed
        });
    });
    group.finish();
}

fn bench_topic_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("ours/topic-filter");
    group.throughput(Throughput::Elements(MESSAGES_PER_PUBLISHER as u64));
    group.bench_function("one-of-two-topics", |b| {
        b.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                let bus = EventBus::<Volume, Evt>::new();
                let mut filtered = bus
                    .subscribe_evt_topics("signal-only", &[Topic::Signal])
                    .expect("observer");

                let started = Instant::now();
                for seq in 0..MESSAGES_PER_PUBLISHER as u64 {
                    bus.publish_evt(Evt::new(seq, false)).expect("publish");
                }
                elapsed += started.elapsed();

                let received = drain_ours(&mut filtered);
                assert_eq!(
                    received,
                    MESSAGES_PER_PUBLISHER / 2,
                    "only the selected topic is delivered"
                );
            }
            elapsed
        });
    });
    group.finish();
}

/// A paused observer accumulates; the queue depth is reported, not hidden.
fn bench_paused_backlog(c: &mut Criterion) {
    let mut group = c.benchmark_group("ours/paused-observer");
    group.throughput(Throughput::Elements(10_000));
    group.bench_function("accumulate-10000", |b| {
        b.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                let bus = EventBus::<Volume, Evt>::new();
                let mut paused = bus.subscribe_evt("paused").expect("observer");

                let started = Instant::now();
                for seq in 0..10_000u64 {
                    bus.publish_evt(Evt::new(seq, false)).expect("publish");
                }
                elapsed += started.elapsed();

                // Overload shows up as depth and memory growth, not as drops.
                let diagnostics = bus.diagnostics().expect("diagnostics");
                assert_eq!(
                    diagnostics.events.subscriptions[0].queued, 10_000,
                    "nothing is dropped for a stalled consumer"
                );
                assert_eq!(drain_ours(&mut paused), 10_000);
            }
            elapsed
        });
    });
    group.finish();
}

fn report_hardware() {
    println!("\nfan-out benchmark");
    println!(
        "  os:        {} ({})",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!(
        "  cores:     {}",
        std::thread::available_parallelism().map_or(0, std::num::NonZeroUsize::get)
    );
    println!("  build:     `cargo bench` release profile (opt-level 3)");
    println!("  encoding:  none — payloads are Rust values; FRB is not measured here");
}

/// Per-message end-to-end latency, not Criterion's distribution of batch times.
fn report_latency() {
    const SAMPLES: usize = 1_000;
    let bus = EventBus::<Volume, Timed>::new();
    let mut rx = bus.subscribe_evt("latency").expect("observer");

    let mut samples = Vec::with_capacity(SAMPLES);
    for seq in 0..SAMPLES as u64 {
        let sent = Instant::now();
        bus.publish_evt(Timed { seq, sent }).expect("publish");
        let event = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("open lane")
            .expect("event");
        assert_eq!(event.seq, seq, "latency samples stay in order");
        samples.push(sent.elapsed());
    }
    samples.sort_unstable();

    let percentile = |p: usize| samples[(SAMPLES - 1) * p / 100];
    println!("end-to-end latency (publish -> receive, single publisher, one observer):");
    println!("  p50: {:?}", percentile(50));
    println!("  p95: {:?}", percentile(95));
    println!("  p99: {:?}", percentile(99));
    println!("  max: {:?}", samples[SAMPLES - 1]);
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_ours(&mut criterion);
    bench_legacy(&mut criterion);
    bench_command_delivery(&mut criterion);
    bench_topic_filter(&mut criterion);
    bench_paused_backlog(&mut criterion);
    report_hardware();
    report_latency();
    criterion.final_summary();
}
