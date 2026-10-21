//! Bounded single-producer/single-consumer ring for live stem playback.
//!
//! A background separator writes finalized stem windows ahead of the deck's
//! playhead; the audio callback reads them without locking or allocating. Frames
//! are absolute (track position at [`LiveStemRing::rate`]), so a deck seek can
//! fall back to the main mix until the producer covers the new position.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// Four stems in NI/engine order: drums, bass, other, vocals.
pub const LIVE_STEM_COUNT: usize = 4;

/// Smallest allowed ring (frames): one HTDemucs segment is ~344k frames.
const MIN_CAPACITY_FRAMES: usize = 1 << 19;

/// Ring capacity used by the host: 2^20 frames ≈ 23.8 s at 44.1 kHz.
pub const DEFAULT_LIVE_STEM_CAPACITY: usize = 1 << 20;

/// Absolute-frame provenance shared between producer and consumer.
///
/// `start..end` is the continuous range that is currently published and still
/// resident (old frames are evicted as the producer wraps). Both halves fit in
/// `u32`; a track longer than ~27 hours at 44.1 kHz is out of scope.
struct Span {
    start: AtomicU64,
    end: AtomicU64,
}

impl Span {
    fn get(&self) -> (u64, u64) {
        // `end` is published after `start` for a reset (start first) and after
        // the samples for an append. A stale start paired with a new end can
        // only under-report coverage, which is safe.
        let start = self.start.load(Ordering::Acquire);
        let end = self.end.load(Ordering::Acquire);
        (start, end)
    }
}

/// Live stem ring shared by an [`analyzer_stems`]-style producer and a deck.
pub struct LiveStemRing {
    /// Stem sample rate (the separation model rate).
    rate: u32,
    /// Capacity in frames; power of two.
    capacity: usize,
    mask: u64,
    /// Frame-major `[frame][stem][channel]` sample bits.
    ///
    /// One stereo pair for all four stems is 8 × 4 B = 32 B, so a frame and
    /// the next (needed for interpolation) share one or two cache lines in the
    /// audio callback.
    ///
    /// `AtomicU32` (relaxed) gives the producer write access through `&self`
    /// with no runtime cost on the supported targets; the `Release` store of
    /// `span.end` publishes the preceding sample stores to the reader's
    /// `Acquire` load.
    samples: Box<[AtomicU32]>,
    span: Span,
    /// Deck position (frames at [`Self::rate`]) published by the consumer.
    playhead: AtomicU64,
    closed: AtomicBool,
}

impl LiveStemRing {
    /// Allocate a ring for `rate` Hz stems.
    ///
    /// # Panics
    /// If `capacity_frames` is not a power of two or is smaller than 2^19.
    pub fn new(rate: u32, capacity_frames: usize) -> Self {
        assert!(rate > 0, "live stem rate must be > 0");
        assert!(
            capacity_frames.is_power_of_two() && capacity_frames >= MIN_CAPACITY_FRAMES,
            "live stem capacity must be a power of two >= {MIN_CAPACITY_FRAMES}"
        );
        Self {
            rate,
            capacity: capacity_frames,
            mask: (capacity_frames - 1) as u64,
            samples: (0..LIVE_STEM_COUNT * capacity_frames * 2)
                .map(|_| AtomicU32::new(0))
                .collect(),
            span: Span {
                start: AtomicU64::new(0),
                end: AtomicU64::new(0),
            },
            playhead: AtomicU64::new(0),
            closed: AtomicBool::new(false),
        }
    }

    /// Stem sample rate (frames are counted at this rate).
    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Ring capacity in frames (power of two).
    pub fn capacity_frames(&self) -> usize {
        self.capacity
    }

    /// Published and resident frame range, if any.
    pub fn coverage(&self) -> Option<(u64, u64)> {
        let (start, end) = self.span.get();
        (end > start).then_some((start, end))
    }

    /// True when the whole interpolation pair `frame, frame + 1` is covered.
    pub fn covers_interpolated(&self, frame: u64) -> bool {
        let (start, end) = self.span.get();
        frame >= start && frame.saturating_add(1) < end
    }

    /// Read one stereo frame for `stem`; the caller must have checked coverage.
    ///
    /// # Panics
    /// If `stem >= 4`.
    pub fn frame(&self, stem: usize, frame: u64) -> (f32, f32) {
        assert!(stem < LIVE_STEM_COUNT, "stem index out of range");
        let slot = (frame & self.mask) as usize * LIVE_STEM_COUNT * 2 + stem * 2;
        (
            f32::from_bits(self.samples[slot].load(Ordering::Relaxed)),
            f32::from_bits(self.samples[slot + 1].load(Ordering::Relaxed)),
        )
    }

    /// Deck position, in stem frames.
    pub fn playhead(&self) -> u64 {
        self.playhead.load(Ordering::Acquire)
    }

    /// Publish the deck position so the producer can pace ahead of it.
    pub fn set_playhead(&self, frame: u64) {
        self.playhead.store(frame, Ordering::Release);
    }

    /// Stop the producer and invalidate coverage.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        // Shrink `end` first: an intermediate `[0, old_end)` span would claim
        // frames that were never written under the new epoch.
        self.span.end.store(0, Ordering::Release);
        self.span.start.store(0, Ordering::Release);
    }

    /// True after [`Self::close`].
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// Start a new epoch at `frame`, discarding prior coverage.
    pub fn reset(&self, frame: u64) {
        let frame = frame.min(u64::from(u32::MAX));
        let (start, _) = self.span.get();
        if frame < start {
            // Backward: empty the span before moving its start up.
            self.span.end.store(frame, Ordering::Release);
            self.span.start.store(frame, Ordering::Release);
        } else {
            // Forward: move the start first so no unwritten gap is published.
            self.span.start.store(frame, Ordering::Release);
            self.span.end.store(frame, Ordering::Release);
        }
    }

    /// Append one finalized window starting at `start`.
    ///
    /// `stems[s]` is interleaved stereo PCM for [`LIVE_STEM_COUNT`] stems and
    /// must contain at least `frames * 2` samples. The publication is release
    /// ordered after the sample writes; readers that observe the new `end` see
    /// fully written frames.
    ///
    /// # Panics
    /// If a stem buffer is shorter than `frames * 2`.
    pub fn publish(&self, start: u64, frames: usize, stems: &[&[f32]; LIVE_STEM_COUNT]) {
        if frames == 0 || self.is_closed() {
            return;
        }
        let end = start.saturating_add(frames as u64);
        let capacity = self.capacity as u64;
        for (stem, data) in stems.iter().enumerate() {
            assert!(
                data.len() >= frames * 2,
                "stem {stem} window too short: {} < {}",
                data.len(),
                frames * 2
            );
        }
        for i in 0..frames {
            let base = ((start + i as u64) & self.mask) as usize * LIVE_STEM_COUNT * 2;
            for (stem, data) in stems.iter().enumerate() {
                self.samples[base + stem * 2].store(data[i * 2].to_bits(), Ordering::Relaxed);
                self.samples[base + stem * 2 + 1]
                    .store(data[i * 2 + 1].to_bits(), Ordering::Relaxed);
            }
        }

        let (old_start, old_end) = self.span.get();
        let contiguous = start <= old_end;
        let new_start = if contiguous { old_start } else { start };
        // Only frames within the last `capacity` remain resident.
        let new_start = new_start.max(end.saturating_sub(capacity));
        if new_start != old_start {
            self.span.start.store(new_start, Ordering::Release);
        }
        self.span.end.store(end.max(old_end), Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(frames: usize, value: f32) -> Vec<f32> {
        vec![value; frames * 2]
    }

    #[test]
    fn empty_until_published() {
        let ring = LiveStemRing::new(44_100, 1 << 19);
        assert_eq!(ring.coverage(), None);
        ring.reset(0);
        assert_eq!(ring.coverage(), None);
    }

    #[test]
    fn publish_maps_absolute_frames_and_wraps() {
        let cap = 1 << 19;
        let ring = LiveStemRing::new(44_100, cap);
        ring.set_playhead(0);
        let a = window(4, 0.25);
        let b = window(4, 0.5);
        ring.publish(cap as u64 - 2, 4, &[&a, &a, &a, &a]);
        assert_eq!(ring.coverage(), Some((cap as u64 - 2, cap as u64 + 2)));
        assert_eq!(ring.frame(0, cap as u64), (0.25, 0.25));
        // After wrapping, frame `cap` aliases slot 0; writing b overwrites only
        // frames that were outside the published prefix.
        ring.publish(cap as u64, 4, &[&b, &b, &b, &b]);
        assert_eq!(ring.coverage(), Some((cap as u64 - 2, cap as u64 + 4)));
        assert_eq!(ring.frame(3, cap as u64 + 3), (0.5, 0.5));
    }

    #[test]
    fn publish_evicts_frames_outside_capacity() {
        let cap = 1 << 19;
        let ring = LiveStemRing::new(44_100, cap);
        let w = window(cap, 0.1);
        ring.publish(0, cap, &[&w, &w, &w, &w]);
        assert_eq!(ring.coverage(), Some((0, cap as u64)));
        let w2 = window(8, 0.2);
        ring.publish(cap as u64, 8, &[&w2, &w2, &w2, &w2]);
        assert_eq!(ring.coverage(), Some((8, cap as u64 + 8)));
    }

    #[test]
    fn close_invalidates_coverage() {
        let ring = LiveStemRing::new(44_100, 1 << 19);
        let w = window(4, 1.0);
        ring.publish(0, 4, &[&w, &w, &w, &w]);
        assert!(ring.coverage().is_some());
        ring.close();
        assert!(ring.is_closed());
        assert_eq!(ring.coverage(), None);
    }

    #[test]
    fn interpolated_requires_the_next_frame() {
        let ring = LiveStemRing::new(44_100, 1 << 19);
        let w = window(10, 0.0);
        ring.publish(0, 10, &[&w, &w, &w, &w]);
        assert_eq!(ring.coverage(), Some((0, 10)));
        assert!(!ring.covers_interpolated(9));
        assert!(ring.covers_interpolated(8));
    }
}
