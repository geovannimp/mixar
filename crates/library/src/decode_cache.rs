//! Bounded decoded-PCM cache for [`crate::LibraryManager`].

use audio_core::LoadedAudio;
use library_core::TrackId;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, OnceLock};

/// Soft ceiling on retained interleaved f32 PCM (~512 MiB).
pub(crate) const DEFAULT_MAX_BYTES: usize = 512 * 1024 * 1024;
/// Caps entry count so many short clips cannot grow the map without bound.
pub(crate) const DEFAULT_MAX_ENTRIES: usize = 8;

/// One retained track: its decoded PCM plus the track-wide waveform amplitude
/// reference, which lives exactly as long as this PCM identity.
pub(crate) struct DecodedAudio {
    pub(crate) audio: Arc<LoadedAudio>,
    /// Filled on the first nonempty detail request; shared by every later
    /// crop/zoom of the same decoded track.
    pub(crate) waveform_peak: OnceLock<f32>,
}

pub(crate) struct DecodeCache {
    map: HashMap<TrackId, Arc<DecodedAudio>>,
    /// Oldest at front; newest / most-recently-used at back.
    order: VecDeque<TrackId>,
    bytes: usize,
    max_bytes: usize,
    max_entries: usize,
}

fn pcm_bytes(audio: &LoadedAudio) -> usize {
    audio.samples.len() * std::mem::size_of::<f32>()
}

impl DecodeCache {
    pub(crate) fn new() -> Self {
        Self::with_limits(DEFAULT_MAX_BYTES, DEFAULT_MAX_ENTRIES)
    }

    /// Build a cache with custom limits (tests).
    ///
    /// `get`/`touch` are O(n) in `max_entries`; keep that small (production uses 8).
    pub(crate) fn with_limits(max_bytes: usize, max_entries: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            max_bytes,
            max_entries: max_entries.max(1),
        }
    }

    /// PCM-only lookup: playback / stems / export need nothing else.
    pub(crate) fn get(&mut self, id: &TrackId) -> Option<Arc<LoadedAudio>> {
        self.get_for_waveform(id).map(|entry| entry.audio.clone())
    }

    /// Lookup that keeps the entry (and its cached waveform reference) shared.
    pub(crate) fn get_for_waveform(&mut self, id: &TrackId) -> Option<Arc<DecodedAudio>> {
        let entry = self.map.get(id).cloned()?;
        self.touch(id);
        Some(entry)
    }

    /// Insert or replace. Evicts least-recently-used entries to stay within limits.
    ///
    /// A single oversized track is kept (decks still need the PCM); other entries
    /// are dropped first. Library has no live-deck set — engine holds its own `Arc`s.
    pub(crate) fn insert(&mut self, id: TrackId, audio: Arc<LoadedAudio>) -> Arc<LoadedAudio> {
        self.insert_for_waveform(id, audio).audio.clone()
    }

    /// Insert or replace, returning the entry every other caller must share.
    ///
    /// Each insertion (including a replacement) starts a fresh, empty reference
    /// cell, so a stale scalar can never outlive the PCM it was computed from.
    pub(crate) fn insert_for_waveform(
        &mut self,
        id: TrackId,
        audio: Arc<LoadedAudio>,
    ) -> Arc<DecodedAudio> {
        let new_bytes = pcm_bytes(&audio);
        let entry = Arc::new(DecodedAudio {
            audio,
            waveform_peak: OnceLock::new(),
        });
        if let Some(old) = self.map.insert(id.clone(), Arc::clone(&entry)) {
            self.bytes = self
                .bytes
                .saturating_sub(pcm_bytes(&old.audio))
                .saturating_add(new_bytes);
            self.touch(&id);
        } else {
            self.bytes = self.bytes.saturating_add(new_bytes);
            self.order.push_back(id);
        }
        self.evict_if_needed();
        entry
    }

    fn evict_if_needed(&mut self) {
        while self.map.len() > self.max_entries
            || (self.bytes > self.max_bytes && self.map.len() > 1)
        {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(removed) = self.map.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(pcm_bytes(&removed.audio));
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.bytes = 0;
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    #[cfg(test)]
    pub(crate) fn contains(&self, id: &TrackId) -> bool {
        self.map.contains_key(id)
    }

    #[cfg(test)]
    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }

    fn touch(&mut self, id: &TrackId) {
        // shortcut: O(n) scan/remove is fine while DEFAULT_MAX_ENTRIES is tiny.
        // Upgrade to an intrusive/index map if the entry cap grows large.
        if let Some(pos) = self.order.iter().position(|x| x == id) {
            if let Some(moved) = self.order.remove(pos) {
                self.order.push_back(moved);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio(samples: usize) -> Arc<LoadedAudio> {
        Arc::new(LoadedAudio {
            samples: vec![0.0; samples],
            sample_rate: 48_000,
            channels: 2,
            source_id: "test".into(),
        })
    }

    fn id(n: u8) -> TrackId {
        TrackId::new(format!("t{n}"))
    }

    #[test]
    fn insert_evicts_oldest_when_over_entry_cap() {
        let mut cache = DecodeCache::with_limits(usize::MAX, 2);
        cache.insert(id(1), audio(4));
        cache.insert(id(2), audio(4));
        cache.insert(id(3), audio(4));

        assert_eq!(cache.len(), 2);
        assert!(!cache.contains(&id(1)));
        assert!(cache.contains(&id(2)));
        assert!(cache.contains(&id(3)));
    }

    #[test]
    fn insert_evicts_until_under_byte_cap() {
        // 100 interleaved f32 samples = 400 bytes each entry.
        let mut cache = DecodeCache::with_limits(500, 8);
        cache.insert(id(1), audio(100));
        cache.insert(id(2), audio(100));
        assert_eq!(cache.len(), 1);
        assert!(!cache.contains(&id(1)));
        assert!(cache.contains(&id(2)));
        assert!(cache.bytes() <= 500);
    }

    #[test]
    fn get_refreshes_lru_order() {
        let mut cache = DecodeCache::with_limits(usize::MAX, 2);
        cache.insert(id(1), audio(4));
        cache.insert(id(2), audio(4));
        assert!(cache.get(&id(1)).is_some());
        cache.insert(id(3), audio(4));

        assert!(cache.contains(&id(1)), "touched entry should survive");
        assert!(!cache.contains(&id(2)));
        assert!(cache.contains(&id(3)));
    }

    #[test]
    fn clear_drops_all_entries() {
        let mut cache = DecodeCache::with_limits(usize::MAX, 8);
        cache.insert(id(1), audio(4));
        cache.insert(id(2), audio(4));
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.bytes(), 0);
        assert!(!cache.contains(&id(1)));
    }

    #[test]
    fn oversized_single_entry_is_kept() {
        let mut cache = DecodeCache::with_limits(100, 8);
        cache.insert(id(1), audio(100)); // 400 bytes > 100
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(&id(1)));
    }

    #[test]
    fn waveform_gets_share_one_entry_and_reference_cell() {
        let mut cache = DecodeCache::with_limits(usize::MAX, 8);
        cache.insert_for_waveform(id(1), audio(4));

        let first = cache.get_for_waveform(&id(1)).expect("entry");
        let second = cache.get_for_waveform(&id(1)).expect("entry");
        assert!(Arc::ptr_eq(&first, &second), "same entry identity");
        assert!(std::ptr::eq(&first.waveform_peak, &second.waveform_peak));

        // PCM callers keep getting the same decoded Arc.
        let pcm = cache.get(&id(1)).expect("pcm");
        assert!(Arc::ptr_eq(&pcm, &first.audio));
        let inserted = cache.insert(id(1), Arc::clone(&pcm));
        assert!(Arc::ptr_eq(&inserted, &pcm), "re-insert keeps the PCM Arc");
    }

    #[test]
    fn concurrent_initializers_fill_the_cell_once() {
        let mut cache = DecodeCache::with_limits(usize::MAX, 8);
        let entry = cache.insert_for_waveform(id(1), audio(4));
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let entry = Arc::clone(&entry);
                let calls = Arc::clone(&calls);
                std::thread::spawn(move || {
                    *entry.waveform_peak.get_or_init(|| {
                        calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        0.25
                    })
                })
            })
            .collect();
        let values: Vec<f32> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(values.iter().all(|v| *v == 0.25));
    }

    #[test]
    fn cached_zero_is_a_value_not_a_sentinel() {
        let mut cache = DecodeCache::with_limits(usize::MAX, 8);
        let entry = cache.insert_for_waveform(id(1), audio(4));
        assert_eq!(*entry.waveform_peak.get_or_init(|| 0.0), 0.0);
        assert_eq!(entry.waveform_peak.get(), Some(&0.0));
        // Already initialized, so a later initializer never runs.
        assert_eq!(*entry.waveform_peak.get_or_init(|| 1.0), 0.0);
    }

    #[test]
    fn replacement_starts_a_fresh_reference_cell() {
        let mut cache = DecodeCache::with_limits(usize::MAX, 8);
        let first = cache.insert_for_waveform(id(1), audio(4));
        assert!(first.waveform_peak.set(0.5).is_ok());
        let replacement = cache.insert_for_waveform(id(1), audio(8));

        assert!(!Arc::ptr_eq(&first, &replacement));
        assert_eq!(replacement.waveform_peak.get(), None);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.bytes(), 8 * 4);
    }

    #[test]
    fn clear_or_evict_then_reinsert_never_reuses_the_old_cell() {
        let mut cache = DecodeCache::with_limits(usize::MAX, 2);
        let first = cache.insert_for_waveform(id(1), audio(4));
        assert!(first.waveform_peak.set(0.75).is_ok());
        cache.clear();
        assert!(cache.get_for_waveform(&id(1)).is_none());
        let after_clear = cache.insert_for_waveform(id(1), audio(4));
        assert_eq!(after_clear.waveform_peak.get(), None);

        cache.insert_for_waveform(id(2), audio(4));
        cache.insert_for_waveform(id(3), audio(4)); // evicts id(1)
        assert!(!cache.contains(&id(1)));
        let after_evict = cache.insert_for_waveform(id(1), audio(4));
        assert!(!Arc::ptr_eq(&after_evict, &first));
        assert_eq!(after_evict.waveform_peak.get(), None);
    }
}
