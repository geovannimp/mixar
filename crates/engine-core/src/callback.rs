use audio_core::{AudioCallback, Sample};
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

        for sample in out.iter_mut() {
            match self.consumer.pop() {
                Ok(value) => {
                    *sample = value;
                }
                Err(_) => {
                    *sample = 0.0;
                }
            }
        }
    }
}
