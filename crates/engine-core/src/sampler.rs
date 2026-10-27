use anyhow::Result;
use audio_core::LoadedAudio;
use library::PreparedTrackPlayback;
use library_core::{AudioSource, LoadableAudio};
use std::sync::Arc;

use crate::engine::Engine;

impl Engine {
    /// Assign a decoded sample to a sampler pad slot on a deck.
    pub fn assign_sampler_slot(
        &mut self,
        deck_id: usize,
        slot: usize,
        source: AudioSource,
        label: String,
        loudness_lufs: Option<f64>,
    ) -> Result<()> {
        let audio = Arc::new(source.load()?);
        self.assign_sampler_audio(deck_id, slot, audio, label, loudness_lufs)
    }

    /// Assign already-decoded library audio to a sampler pad (prepare off the engine lock).
    pub fn assign_prepared_sampler(
        &mut self,
        deck_id: usize,
        slot: usize,
        prepared: PreparedTrackPlayback,
    ) -> Result<()> {
        let label = sampler_slot_label(&prepared.source);
        let loudness = prepared.loudness_lufs;
        self.assign_sampler_audio(deck_id, slot, prepared.audio, label, loudness)
    }

    fn assign_sampler_audio(
        &mut self,
        deck_id: usize,
        slot: usize,
        audio: Arc<LoadedAudio>,
        label: String,
        loudness_lufs: Option<f64>,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        let sample_rate = dsp.sample_rate();
        let quality = dsp.mixer().resampler_quality().to_string();
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .assign_slot(slot, audio, label, sample_rate, &quality, loudness_lufs)
    }

    /// Clear a sampler pad slot on a deck.
    pub fn clear_sampler_slot(&mut self, deck_id: usize, slot: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .clear_slot(slot)
    }

    /// Clear all sampler slots on a deck (e.g. before loading a bank).
    pub fn clear_all_sampler_slots(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .clear_all_slots();
        Ok(())
    }

    /// Set effective sampler play mode for a deck.
    pub fn set_sampler_play_mode(
        &mut self,
        deck_id: usize,
        mode: engine_dsp::SamplerPlayMode,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .set_play_mode(mode);
        Ok(())
    }

    /// Trigger a sample from a pad slot on a deck.
    pub fn trigger_sampler(&mut self, deck_id: usize, slot: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .trigger(slot)
    }

    /// End hold/loop for a sampler pad slot on a deck.
    pub fn end_sampler(&mut self, deck_id: usize, slot: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .end(slot)
    }

    /// Whether a sampler slot has assigned audio on a deck.
    pub fn sampler_slot_assigned(&self, deck_id: usize, slot: usize) -> bool {
        let Some(dsp_engine) = self.dsp_engine.as_ref() else {
            return false;
        };
        let Ok(dsp) = dsp_engine.lock() else {
            return false;
        };
        dsp.sampler(deck_id).is_some_and(|s| s.slot_assigned(slot))
    }

    /// Recompute auto-gain for a loaded sampler slot after normalizer settings change.
    pub fn set_sampler_slot_auto_gain(
        &mut self,
        deck_id: usize,
        slot: usize,
        auto_gain_db: f32,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .set_slot_auto_gain_db(slot, auto_gain_db)
    }

    /// Record which library sampler bank is loaded on this deck's pads.
    pub fn set_deck_sampler_bank(&mut self, deck_id: usize, bank_id: Option<String>) -> Result<()> {
        let control = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        control.active_sampler_bank_id = bank_id;
        Ok(())
    }
}

fn sampler_slot_label(source: &AudioSource) -> String {
    if let Some(title) = source.metadata().title.as_ref().filter(|t| !t.is_empty()) {
        return title.clone();
    }
    source
        .file()
        .and_then(|f| {
            f.path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| source.id().as_str().to_string())
}
