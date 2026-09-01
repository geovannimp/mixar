use audio_core::{AudioCallback, Sample};
use rtrb::chunks::ChunkError;
use rtrb::Consumer;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Consumer audio callback implementation for the engine.
pub(crate) struct ConsumerCallback {
    consumer: Consumer<Sample>,
    callback_count: Arc<AtomicU64>,
}

impl ConsumerCallback {
    pub(crate) fn new(consumer: Consumer<Sample>, callback_count: Arc<AtomicU64>) -> Self {
        Self {
            consumer,
            callback_count,
        }
    }
}

impl AudioCallback for ConsumerCallback {
    fn render(&mut self, out: &mut [Sample], _frames: u32, _sample_rate: u32) {
        self.callback_count.fetch_add(1, Ordering::Relaxed);

        let chunk = match self.consumer.read_chunk(out.len()) {
            Ok(chunk) => chunk,
            Err(ChunkError::TooFewSlots(0)) => {
                out.fill(0.0);
                return;
            }
            Err(ChunkError::TooFewSlots(n)) => match self.consumer.read_chunk(n) {
                Ok(chunk) => chunk,
                Err(_) => {
                    out.fill(0.0);
                    return;
                }
            },
        };

        let take = chunk.len().min(out.len());
        let (first, second) = chunk.as_slices();
        let first_take = first.len().min(take);
        let second_take = take - first_take;
        out[..first_take].copy_from_slice(&first[..first_take]);
        out[first_take..take].copy_from_slice(&second[..second_take]);
        chunk.commit(take);
        out[take..].fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtrb::RingBuffer;

    fn make_callback(capacity: usize) -> (ConsumerCallback, rtrb::Producer<Sample>) {
        let (producer, consumer) = RingBuffer::new(capacity);
        let callback = ConsumerCallback::new(consumer, Arc::new(AtomicU64::new(0)));
        (callback, producer)
    }

    #[test]
    fn render_bulk_pop_fills_output_from_ring() {
        let (mut callback, mut producer) = make_callback(16);
        let samples: [Sample; 4] = [0.1, 0.2, 0.3, 0.4];
        for &sample in &samples {
            producer.push(sample).unwrap();
        }

        let mut out = [0.0; 4];
        callback.render(&mut out, 2, 48_000);

        assert_eq!(out, samples);
    }

    #[test]
    fn render_underrun_fills_silence() {
        let (mut callback, _producer) = make_callback(16);

        let mut out = [1.0; 4];
        callback.render(&mut out, 2, 48_000);

        assert_eq!(out, [0.0; 4]);
    }

    #[test]
    fn render_partial_underrun_fills_remainder_with_silence() {
        let (mut callback, mut producer) = make_callback(16);
        producer.push(0.5).unwrap();
        producer.push(0.6).unwrap();

        let mut out = [1.0; 4];
        callback.render(&mut out, 2, 48_000);

        assert_eq!(out, [0.5, 0.6, 0.0, 0.0]);
    }

    /// Mirrors `create_device_ring_buffer` prefill: capacity 24×N, prefill ~22×N, callback out.len() = N.
    #[test]
    fn render_prefilled_ring_reads_one_callback_slice() {
        const RING_BUFFER_MULTIPLIER: usize = 24;
        let samples_per_buffer = 4;
        let capacity = samples_per_buffer * RING_BUFFER_MULTIPLIER;
        let prefill = capacity - 2 * samples_per_buffer;

        let (mut producer, consumer) = RingBuffer::new(capacity);
        for _ in 0..prefill {
            producer.push(0.0).unwrap();
        }
        let mut callback = ConsumerCallback::new(consumer, Arc::new(AtomicU64::new(0)));

        let mut out = [1.0; 4];
        callback.render(&mut out, 2, 48_000);

        assert_eq!(out, [0.0; 4]);
        // Writable headroom grows by one consumed callback slice.
        assert_eq!(
            producer.slots(),
            2 * samples_per_buffer + samples_per_buffer
        );
    }
}
