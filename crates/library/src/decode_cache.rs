//! Bounded decoded-PCM cache for [`crate::LibraryManager`].

use audio_core::LoadedAudio;
use library_core::TrackId;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Soft ceiling on retained interleaved f32 PCM (~512 MiB).
pub(crate) const DEFAULT_MAX_BYTES: usize = 512 * 1024 * 1024;
/// Caps entry count so many short clips cannot grow the map without bound.
pub(crate) const DEFAULT_MAX_ENTRIES: usize = 8;

pub(crate) struct DecodeCache {
    map: HashMap<TrackId, Arc<LoadedAudio>>,
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

    pub(crate) fn with_limits(max_bytes: usize, max_entries: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            max_bytes,
            max_entries: max_entries.max(1),
        }
    }

    pub(crate) fn get(&mut self, id: &TrackId) -> Option<Arc<LoadedAudio>> {
        let audio = self.map.get(id).cloned()?;
        self.touch(id);
        Some(audio)
    }

    /// Insert or replace. Evicts least-recently-used entries to stay within limits.
    ///
    /// A single oversized track is kept (decks still need the PCM); other entries
    /// are dropped first. Library has no live-deck set — engine holds its own `Arc`s.
    pub(crate) fn insert(&mut self, id: TrackId, audio: Arc<LoadedAudio>) -> Arc<LoadedAudio> {
        let new_bytes = pcm_bytes(&audio);
        if let Some(old) = self.map.insert(id.clone(), Arc::clone(&audio)) {
            self.bytes = self
                .bytes
                .saturating_sub(pcm_bytes(&old))
                .saturating_add(new_bytes);
            self.touch(&id);
        } else {
            self.bytes = self.bytes.saturating_add(new_bytes);
            self.order.push_back(id);
        }
        self.evict_if_needed();
        audio
    }

    fn evict_if_needed(&mut self) {
        while self.map.len() > self.max_entries
            || (self.bytes > self.max_bytes && self.map.len() > 1)
        {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(removed) = self.map.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(pcm_bytes(&removed));
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
        // 100 f32 samples = 400 bytes each entry.
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
}
