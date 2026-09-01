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
            Err(ChunkError::TooFewSlots(n)) => self.consumer.read_chunk(n).unwrap(),
        };

        let (first, second) = chunk.as_slices();
        let mid = first.len();
        let filled = chunk.len();
        out[..mid].copy_from_slice(first);
        out[mid..filled].copy_from_slice(second);
        chunk.commit_all();
        out[filled..].fill(0.0);
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
}
