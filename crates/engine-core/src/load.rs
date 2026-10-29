use anyhow::Result;
use bus_api::SyncMode;
use library::{LibraryManager, PreparedTrackPlayback};
use library_core::{AudioSource, LoadableAudio, TrackId, TrackMetadata};
use std::sync::Arc;

use crate::engine::Engine;

impl Engine {
    /// Load a library-indexed track into a deck through the shared library manager.
    pub fn load_track_from_library(&mut self, deck_id: usize, track_id: &TrackId) -> Result<()> {
        let library = self
            .library
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine has no attached LibraryManager"))?;
        let mut prepared = LibraryManager::prepare_track_for_playback(library.as_ref(), track_id)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let stems = prepared.stems.take();
        self.load_prepared_track(deck_id, prepared)?;
        if let Some(stems) = stems {
            self.attach_deck_stems(deck_id, stems)?;
        }
        Ok(())
    }

    /// Load a track from an [`AudioSource`] into a deck.
    ///
    /// Decodes via [`LoadableAudio`] on demand. Channel auto gain is derived from
    /// source metadata (ReplayGain preferred, else `loudness_lufs`) together with
    /// [`Self::set_normalizer_target`].
    pub fn load_track(&mut self, deck_id: usize, source: AudioSource) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        {
            let dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
            if dsp.deck(deck_id).is_none() || dsp.mixer().channel(deck_id).is_none() {
                return Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id));
            }
        }

        let track_id = source.id().clone();
        let metadata = source.metadata().clone();
        let track_path = source.source_ref();
        let loudness_lufs = loudness_from_metadata(&metadata);
        let bpm = metadata.bpm;
        let audio = Arc::new(source.load()?);

        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        {
            let deck = dsp.deck_mut(deck_id).expect("validated above");
            deck.load(audio)?;
            deck.set_track_bpm(bpm);
        }
        dsp.mixer_mut()
            .channel_mut(deck_id)
            .expect("validated above")
            .set_loudness_lufs(loudness_lufs);
        if let Some(control) = self.deck_control.get_mut(deck_id) {
            control.apply_loaded_metadata(track_id, track_path, &metadata);
        }
        drop(dsp);
        self.hydrate_hot_cues(deck_id)?;
        self.hydrate_beat_grid(deck_id)?;
        self.resync_followers_after_load(deck_id)?;
        tracing::info!("Track loaded into deck {}", deck_id);
        Ok(())
    }

    /// Load a track prepared by [`library::LibraryManager`] into a deck.
    ///
    /// Consumes the library-owned bounded decode cache output directly; the engine holds no
    /// cache of its own. Callers that may receive `stems` must take them before calling this
    /// method and attach them with [`Self::attach_deck_stems`] — decoding, phase selection,
    /// and stem routing stay library/host-owned.
    pub fn load_prepared_track(
        &mut self,
        deck_id: usize,
        prepared: PreparedTrackPlayback,
    ) -> Result<()> {
        // Stems must be taken by the caller and attached afterwards (see the doc comment);
        // catching a Some here keeps the drop loud in test builds instead of silent.
        debug_assert!(
            prepared.stems.is_none(),
            "load_prepared_track drops prepared.stems; take them before calling and attach via attach_deck_stems"
        );
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        // Phase A (locked, short): validate the deck and snapshot the params needed to build.
        let spec = {
            let dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
            if dsp.deck(deck_id).is_none() || dsp.mixer().channel(deck_id).is_none() {
                return Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id));
            }
            dsp.deck(deck_id).expect("validated above").load_spec()
        };

        let bpm = prepared.source.metadata().bpm;
        let track_id = prepared.track_id.clone();
        let loudness_lufs =
            loudness_from_metadata(prepared.source.metadata()).or(prepared.loudness_lufs);

        // Phase B (unlocked): resampler/stretcher construction so the producer can keep filling.
        let deck_load = spec.prepare(prepared.audio)?;

        // Phase C (locked, short): swap in prepared resources only.
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        {
            let deck = dsp.deck_mut(deck_id).expect("validated above");
            deck.apply_prepared_load(deck_load);
            deck.set_track_bpm(bpm);
        }
        dsp.mixer_mut()
            .channel_mut(deck_id)
            .expect("validated above")
            .set_loudness_lufs(loudness_lufs);
        if let Some(control) = self.deck_control.get_mut(deck_id) {
            control.apply_source_load(&prepared.source, track_id);
        }
        drop(dsp);
        self.hydrate_hot_cues(deck_id)?;
        self.hydrate_beat_grid(deck_id)?;
        self.resync_followers_after_load(deck_id)?;
        tracing::info!("Library-prepared track loaded into deck {}", deck_id);
        Ok(())
    }

    /// When the master deck's BPM changes on load, refresh synced slaves' ratios.
    fn resync_followers_after_load(&mut self, loaded_deck_id: usize) -> Result<()> {
        if loaded_deck_id != self.master_deck {
            return Ok(());
        }
        for slave_id in 0..self.deck_control.len() {
            if slave_id == loaded_deck_id {
                continue;
            }
            if self.deck_control[slave_id].sync_mode == SyncMode::Off {
                continue;
            }
            // BPM may be missing on a follower; skip rather than fail the load.
            if self.apply_tempo_sync(slave_id, loaded_deck_id).is_err() {
                continue;
            }
        }
        Ok(())
    }

    /// Set loudness-normalization target for all mixer channels (`None` = off).
    pub fn set_normalizer_target(&mut self, target_lufs: Option<f32>) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        dsp.mixer_mut().set_normalizer_target(target_lufs);
        Ok(())
    }

    /// Cached auto-gain (dB) for a deck's mixer channel, when the engine is running.
    pub fn deck_auto_gain_db(&self, deck_id: usize) -> Option<f32> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.mixer().channel(deck_id)?.auto_gain_db())
    }

    /// Unload the track from a deck. Clears channel loudness (auto gain → 0) while
    /// preserving manual mixer controls (volume, EQ, filter, trim, headphone cue).
    pub fn unload_deck(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");

        if dsp.deck(deck_id).is_none() || dsp.mixer().channel(deck_id).is_none() {
            return Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id));
        }

        dsp.deck_mut(deck_id).expect("validated above").unload()?;
        dsp.mixer_mut()
            .channel_mut(deck_id)
            .expect("validated above")
            .set_loudness_lufs(None);
        if let Some(control) = self.deck_control.get_mut(deck_id) {
            control.clear_loaded_track();
        }
        Ok(())
    }

    fn hydrate_hot_cues(&mut self, deck_id: usize) -> Result<()> {
        let Some(track_id) = self
            .deck_control
            .get(deck_id)
            .and_then(|c| c.track_id.clone())
        else {
            return Ok(());
        };
        let Some(library) = self.library.as_ref() else {
            return Ok(());
        };
        let lib = library
            .lock()
            .map_err(|_| anyhow::anyhow!("library lock poisoned"))?;
        let cues = lib
            .list_track_hot_cues(&track_id)
            .map_err(|e| anyhow::anyhow!("list hot cues: {e}"))?;
        drop(lib);
        let Some(control) = self.deck_control.get_mut(deck_id) else {
            return Ok(());
        };
        control.hot_cues = [None; crate::pads::HOT_CUE_SLOT_COUNT];
        for cue in cues {
            let idx = usize::from(cue.slot_index);
            if idx < control.hot_cues.len() {
                control.hot_cues[idx] = Some(cue.position_ms);
            }
        }
        Ok(())
    }

    /// Load persisted beat-grid BPM + first-beat phase for quantize snap.
    fn hydrate_beat_grid(&mut self, deck_id: usize) -> Result<()> {
        let Some(track_id) = self
            .deck_control
            .get(deck_id)
            .and_then(|c| c.track_id.clone())
        else {
            return Ok(());
        };
        let Some(library) = self.library.as_ref() else {
            return Ok(());
        };
        let lib = library
            .lock()
            .map_err(|_| anyhow::anyhow!("library lock poisoned"))?;
        let grid = lib
            .get_track_beat_grid(&track_id)
            .map_err(|e| anyhow::anyhow!("get beat grid: {e}"))?;
        drop(lib);
        let Some(control) = self.deck_control.get_mut(deck_id) else {
            return Ok(());
        };
        let Some(grid) = grid else {
            control.first_beat_ms = 0.0;
            return Ok(());
        };
        control.first_beat_ms = grid
            .beats
            .first()
            .copied()
            .map(|s| f64::from(s) * 1000.0)
            .unwrap_or(0.0);
        let grid_bpm = grid.bpm.filter(|b| b.is_finite() && *b > 0.0);
        if let Some(b) = grid_bpm {
            control.bpm = Some(b);
        }
        // Keep DSP track_bpm aligned with control when the grid overrides metadata BPM.
        // Safe on load: ratio_override is already cleared by the earlier set_track_bpm.
        if let Some(b) = grid_bpm {
            if let Some(dsp_engine) = self.dsp_engine.as_ref() {
                if let Ok(mut dsp) = dsp_engine.lock() {
                    if let Some(deck) = dsp.deck_mut(deck_id) {
                        deck.set_track_bpm(Some(b));
                    }
                }
            }
        }
        Ok(())
    }
}

fn loudness_from_metadata(metadata: &TrackMetadata) -> Option<f64> {
    if let Some(gain_db) = metadata.replaygain_track_gain_db.filter(|g| g.is_finite()) {
        return Some(analyzer_core::loudness_lufs_from_replaygain_track_gain_db(
            gain_db,
        ));
    }
    metadata.loudness_lufs.filter(|l| l.is_finite())
}
