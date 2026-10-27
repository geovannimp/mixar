use anyhow::Result;
use std::sync::Arc;

use crate::engine::Engine;

impl Engine {
    /// Attach four stem buffers to a deck (drums, bass, other, vocals — NI index order).
    pub fn attach_deck_stems(
        &mut self,
        deck_id: usize,
        stems: [Arc<audio_core::LoadedAudio>; 4],
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        if !deck.has_audio_loaded() {
            return Err(anyhow::anyhow!("Load a track before attaching stems."));
        }
        deck.attach_stems(stems);
        Ok(())
    }

    /// Attach a live stem ring (producer-fed); replaces any attached stems.
    pub fn attach_live_deck_stems(
        &mut self,
        deck_id: usize,
        ring: std::sync::Arc<audio_core::LiveStemRing>,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        if !deck.has_audio_loaded() {
            return Err(anyhow::anyhow!("Load a track before attaching stems."));
        }
        deck.attach_live_stems(ring);
        Ok(())
    }

    /// Stems pad: slots 0–3 toggle mute; 4–7 toggle isolate for that stem.
    pub fn stems_pad_press(&mut self, deck_id: usize, slot: u8) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        if !deck.stems_ready() {
            return Ok(());
        }
        match slot {
            0..=3 => deck.toggle_stem_mute(usize::from(slot)),
            4..=7 => deck.toggle_stem_isolate(slot - 4),
            _ => {
                return Err(anyhow::anyhow!("Stem pad slot must be 0..=7."));
            }
        }
        Ok(())
    }
}
