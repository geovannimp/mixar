use crate::sync::snap_ms;
use anyhow::Result;
use std::sync::Arc;

use crate::engine::Engine;

impl Engine {
    /// Snap playhead and persist a hot cue via the library cmd bus.
    pub fn save_deck_hot_cue(&mut self, deck_id: usize, slot: u8) -> Result<()> {
        if usize::from(slot) >= crate::pads::HOT_CUE_SLOT_COUNT {
            return Err(anyhow::anyhow!(
                "Hot cue slot must be 0..={}.",
                crate::pads::HOT_CUE_SLOT_COUNT - 1
            ));
        }
        let track_id = self
            .deck_control
            .get(deck_id)
            .and_then(|c| c.track_id.clone())
            .ok_or_else(|| anyhow::anyhow!("Only library tracks can persist hot cues."))?;
        let (bpm, first_beat_ms, quantize) = self.deck_quantize_params(deck_id)?;
        let (position_ms, _) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let position_ms = snap_ms(position_ms, bpm, first_beat_ms, quantize);
        // Memory/UI commit on library `HotCuesChanged` (control thread), not here.
        self.publish_library_cmd(
            library_api::Kind::SaveHotCue,
            library_api::CmdBody::SaveHotCue {
                track_id: track_id.as_str().to_string(),
                slot,
                position_ms,
                loop_length_beats: None,
                color: None,
                label: None,
            },
        )?;
        Ok(())
    }

    /// Persist a delete for `slot` via the library cmd bus.
    pub fn delete_deck_hot_cue(&mut self, deck_id: usize, slot: u8) -> Result<()> {
        if usize::from(slot) >= crate::pads::HOT_CUE_SLOT_COUNT {
            return Err(anyhow::anyhow!(
                "Hot cue slot must be 0..={}.",
                crate::pads::HOT_CUE_SLOT_COUNT - 1
            ));
        }
        let track_id = self
            .deck_control
            .get(deck_id)
            .and_then(|c| c.track_id.clone())
            .ok_or_else(|| anyhow::anyhow!("Only library tracks can persist hot cues."))?;
        // Memory/UI commit on library `HotCuesChanged` (control thread), not here.
        self.publish_library_cmd(
            library_api::Kind::DeleteHotCue,
            library_api::CmdBody::DeleteHotCue {
                track_id: track_id.as_str().to_string(),
                slot,
            },
        )?;
        Ok(())
    }

    /// Apply library-acked hot cues to every deck currently loaded with `track_id`.
    ///
    /// Returns deck ids whose slot map changed (caller publishes `DeckUpdated`).
    pub fn apply_track_hot_cues(
        &mut self,
        track_id: &str,
        hot_cues: &[library_api::HotCue],
    ) -> Vec<usize> {
        let mut slots = [None; crate::pads::HOT_CUE_SLOT_COUNT];
        for cue in hot_cues {
            let idx = usize::from(cue.slot);
            if idx < slots.len() {
                slots[idx] = Some(cue.position_ms);
            }
        }
        let mut changed = Vec::new();
        for (deck_id, control) in self.deck_control.iter_mut().enumerate() {
            let Some(loaded) = control.track_id.as_ref() else {
                continue;
            };
            if loaded.as_str() != track_id {
                continue;
            }
            if control.hot_cues != slots {
                control.hot_cues = slots;
                changed.push(deck_id);
            }
        }
        changed
    }

    /// Apply a live beat-grid edit to every deck currently loaded with `track_id`.
    ///
    /// Returns deck ids whose BPM or first-beat phase changed (caller publishes `DeckUpdated`).
    pub fn apply_track_beat_grid(
        &mut self,
        track_id: &str,
        beat_grid: &library_api::BeatGrid,
    ) -> Vec<usize> {
        let first_beat_ms = beat_grid
            .beats
            .first()
            .copied()
            .map(|s| f64::from(s) * 1000.0)
            .unwrap_or(0.0);
        let new_bpm = beat_grid
            .bpm
            .is_finite()
            .then_some(beat_grid.bpm)
            .filter(|b| *b > 0.0);

        let mut changed = Vec::new();
        // Collect BPM pushes and apply under one DSP lock after the loop so
        // multi-deck same-track edits do not bounce the producer lock per deck.
        let mut bpm_updates: Vec<(usize, f64)> = Vec::new();
        for deck_id in 0..self.deck_control.len() {
            let Some(control) = self.deck_control.get_mut(deck_id) else {
                continue;
            };
            let Some(loaded) = control.track_id.as_ref() else {
                continue;
            };
            if loaded.as_str() != track_id {
                continue;
            }

            let mut did_change = false;
            let mut bpm_changed = false;
            if let Some(b) = new_bpm {
                if control.bpm != Some(b) {
                    control.bpm = Some(b);
                    did_change = true;
                    bpm_changed = true;
                }
            }
            // Phase comes from f32 secs × 1000; use a ms-scale tolerance so
            // semantically identical grids do not thrash DeckUpdated.
            const FIRST_BEAT_MS_EPS: f64 = 1e-3;
            if (control.first_beat_ms - first_beat_ms).abs() > FIRST_BEAT_MS_EPS {
                control.first_beat_ms = first_beat_ms;
                did_change = true;
            }
            if !did_change {
                continue;
            }
            changed.push(deck_id);

            // Only push BPM to DSP when it actually changed. `set_track_bpm`
            // clears ratio_override and would break an active tempo/beat sync
            // on phase-only grid edits.
            if bpm_changed {
                if let Some(bpm) = control.bpm {
                    bpm_updates.push((deck_id, bpm));
                }
            }
        }
        if !bpm_updates.is_empty() {
            if let Some(dsp_engine) = self.dsp_engine.as_ref() {
                if let Ok(mut dsp) = dsp_engine.lock() {
                    for (deck_id, bpm) in bpm_updates {
                        if let Some(deck) = dsp.deck_mut(deck_id) {
                            deck.set_track_bpm(Some(bpm));
                        }
                    }
                }
            }
        }
        changed
    }

    fn publish_library_cmd(
        &self,
        kind: library_api::Kind,
        body: library_api::CmdBody,
    ) -> Result<()> {
        let bus = self.library_cmd.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Engine has no library cmd bus for deck performance persistence")
        })?;
        let bytes = library_api::encode_cmd_body(&body)
            .map_err(|e| anyhow::anyhow!("encode library cmd: {e}"))?;
        bus.publish(omnibus::Event::new(
            library_api::Origin::Library,
            kind,
            Arc::from(bytes),
        ))
        .map_err(|e| anyhow::anyhow!("library cmd publish: {e}"))?;
        Ok(())
    }
}
