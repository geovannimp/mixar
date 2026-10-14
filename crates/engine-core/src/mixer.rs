use anyhow::Result;
use engine_dsp::DeckEqGains;

use crate::engine::DeckStripNorms;
use crate::engine::Engine;

impl Engine {
    /// Snapshot pre-fader stereo peaks for all decks (read from their mixer channels).
    pub fn deck_level_snapshot(&self) -> Vec<(usize, f32, f32)> {
        let Some(dsp_engine) = self.dsp_engine.as_ref() else {
            return Vec::new();
        };
        let dsp = match dsp_engine.lock() {
            Ok(dsp) => dsp,
            Err(_) => return Vec::new(),
        };

        let mut snapshot = Vec::with_capacity(dsp.num_decks());
        for deck_id in 0..dsp.num_decks() {
            let Some(channel) = dsp.mixer().channel(deck_id) else {
                continue;
            };
            let peaks = channel.level_peaks();
            snapshot.push((deck_id, peaks.peak_l, peaks.peak_r));
        }
        snapshot
    }

    /// Post-fader master bus peaks from the last DSP process cycle.
    pub fn master_level_snapshot(&self) -> Option<(f32, f32)> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let peaks = dsp.mixer().master_level_peaks();
        Some((peaks.peak_l, peaks.peak_r))
    }

    /// Set a deck's volume (0.0..=1.0), routed through its mixer channel.
    pub fn set_deck_volume(&mut self, deck_id: usize, volume: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(channel) = dsp.mixer_mut().channel_mut(deck_id) {
            channel.set_volume(volume)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set whether a deck is routed to the headphone cue bus, via its mixer channel.
    pub fn set_deck_headphone_cue(&mut self, deck_id: usize, enabled: bool) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        let channel = dsp
            .mixer_mut()
            .channel_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        channel.set_headphone_cue(enabled);
        Ok(())
    }

    /// Set a deck's three-band EQ gains in decibels, via its mixer channel.
    pub fn set_deck_eq(&mut self, deck_id: usize, gains: DeckEqGains) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(channel) = dsp.mixer_mut().channel_mut(deck_id) {
            channel.set_eq_gains(gains)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set a deck's three-band EQ gains in decibels (clamped to ±24 dB).
    pub fn set_deck_eq_bands(
        &mut self,
        deck_id: usize,
        low_db: f32,
        mid_db: f32,
        high_db: f32,
    ) -> Result<()> {
        self.set_deck_eq(deck_id, DeckEqGains::clamped(low_db, mid_db, high_db))
    }

    /// Set DJ filter position for a deck (negative = LP, positive = HP), via its mixer channel.
    pub fn set_deck_filter_db(&mut self, deck_id: usize, filter_db: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(channel) = dsp.mixer_mut().channel_mut(deck_id) {
            channel.set_filter_db(filter_db)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set pre-fader gain trim for a deck in decibels, via its mixer channel.
    pub fn set_deck_gain_trim_db(&mut self, deck_id: usize, gain_db: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(channel) = dsp.mixer_mut().channel_mut(deck_id) {
            channel.set_gain_trim_db(gain_db)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Channel strip + tempo readbacks as wire `0..1` for soft-takeover compares.
    pub fn deck_strip_norms(&self, deck_id: usize) -> Option<DeckStripNorms> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        let channel = dsp.mixer().channel(deck_id)?;
        let eq = channel.eq_gains();
        Some(DeckStripNorms {
            volume: channel.volume(),
            filter: crate::control_norm::strip_db_to_norm(channel.filter_db()),
            gain_trim: crate::control_norm::strip_db_to_norm(channel.gain_trim_db()),
            eq_low: crate::control_norm::strip_db_to_norm(eq.low_db),
            eq_mid: crate::control_norm::strip_db_to_norm(eq.mid_db),
            eq_high: crate::control_norm::strip_db_to_norm(eq.high_db),
            speed: deck.speed(),
            headphone_cue: channel.headphone_cue(),
        })
    }

    /// Set crossfader position (0.0 = deck A, 1.0 = deck B).
    pub fn set_crossfader(&mut self, position: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.mixer_mut().set_crossfader(position)
    }

    pub fn crossfader(&self) -> Option<f32> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.mixer().crossfader())
    }

    /// Set master bus volume (0.0 … 1.0).
    pub fn set_master_volume(&mut self, volume: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.mixer_mut().set_master_volume(volume)
    }

    /// Current master bus volume when the engine is running.
    pub fn master_volume(&self) -> Option<f32> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.mixer().master_volume())
    }

    /// Set cue/headphone bus volume (0.0 … 1.0).
    pub fn set_cue_volume(&mut self, volume: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.mixer_mut().set_cue_volume(volume)
    }

    /// Current cue/headphone bus volume when the engine is running.
    pub fn cue_volume(&self) -> Option<f32> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.mixer().cue_volume())
    }

    /// Set cue blend (0.0 = PFL only, 1.0 = master tap only when `master_cue`).
    pub fn set_cue_mix(&mut self, mix: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.mixer_mut().set_cue_mix(mix)
    }

    /// Current cue blend when the engine is running.
    pub fn cue_mix(&self) -> Option<f32> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.mixer().cue_mix())
    }

    /// Enable or disable master tap on the cue bus.
    pub fn set_master_cue(&mut self, enabled: bool) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.mixer_mut().set_master_cue(enabled);
        Ok(())
    }

    /// Whether master tap is enabled on the cue bus when the engine is running.
    pub fn master_cue(&self) -> Option<bool> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.mixer().master_cue())
    }
}
