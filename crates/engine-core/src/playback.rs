use crate::sync::{require_positive_bpm, snap_ms};
use anyhow::Result;
use audio_core::{ms_to_secs, secs_to_ms};
use engine_api::LoopRegion;

use crate::engine::Engine;
use crate::status::map_jog_mode;

impl Engine {
    /// Play a deck
    pub fn play(&mut self, deck_id: usize) -> Result<()> {
        tracing::info!("Playing deck {}", deck_id);

        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.play()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Pause a deck
    pub fn pause(&mut self, deck_id: usize) -> Result<()> {
        tracing::info!("Pausing deck {}", deck_id);

        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.pause()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Enable/disable slip mode for a deck.
    pub fn set_deck_slip(&mut self, deck_id: usize, enabled: bool) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        deck.set_slip_enabled(enabled)
    }

    pub fn deck_slip_enabled(&self, deck_id: usize) -> Option<bool> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        Some(deck.slip_enabled())
    }

    pub fn deck_slip_shadow_position_ms(&self, deck_id: usize) -> Option<i32> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        deck.shadow_position_ms()
    }

    pub fn set_deck_jog_touch(&mut self, deck_id: usize, touching: bool) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        deck.set_jog_touch(touching);
        Ok(())
    }

    pub fn deck_jog_turn(&mut self, deck_id: usize, delta: i32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        deck.jog_turn(delta);
        Ok(())
    }

    pub fn set_deck_jog_mode(
        &mut self,
        deck_id: usize,
        top: engine_api::JogMode,
        outer: engine_api::JogMode,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        deck.set_jog_mode(map_jog_mode(top), map_jog_mode(outer));
        Ok(())
    }

    /// Seek a deck to a position in milliseconds.
    pub fn seek_deck(&mut self, deck_id: usize, position_ms: i32) -> Result<()> {
        self.seek_deck_inner(deck_id, position_ms, true)
    }

    /// Seek audible playhead only (slip shadow keeps advancing when enabled).
    pub fn seek_deck_audible(&mut self, deck_id: usize, position_ms: i32) -> Result<()> {
        self.seek_deck_inner(deck_id, position_ms, false)
    }

    fn seek_deck_inner(
        &mut self,
        deck_id: usize,
        position_ms: i32,
        move_shadow: bool,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            if move_shadow {
                deck.seek_ms(position_ms)?;
            } else {
                deck.seek_ms_audible_only(position_ms)?;
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set the temporary cue point in milliseconds.
    pub fn set_deck_cue_point(&mut self, deck_id: usize, position_ms: i32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_cue_point_ms(position_ms)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Begin cue-hold audition.
    pub fn begin_deck_cue_hold(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.begin_cue_hold()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// End cue-hold audition.
    pub fn end_deck_cue_hold(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.end_cue_hold()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Activate a loop region on a deck.
    pub fn set_deck_loop_region(&mut self, deck_id: usize, in_ms: i32, out_ms: i32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_loop_region_ms(in_ms, out_ms)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Clear the active loop on a deck.
    pub fn clear_deck_loop(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.clear_loop();
        } else {
            return Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id));
        }
        drop(dsp);
        if let Some(control) = self.deck_control.get_mut(deck_id) {
            control.pending_loop_in_ms = None;
        }
        Ok(())
    }

    /// Trigger hot cue: snap position, seek, play.
    pub fn trigger_deck_hot_cue(&mut self, deck_id: usize, position_ms: i32) -> Result<()> {
        if !self.deck_has_audio_loaded(deck_id).unwrap_or(false) {
            return Err(anyhow::anyhow!("Load a track before triggering a hot cue."));
        }
        let (bpm, first_beat_ms, quantize) = self.deck_quantize_params(deck_id)?;
        let target = snap_ms(position_ms, bpm, first_beat_ms, quantize);
        self.seek_deck_audible(deck_id, target)?;
        self.play(deck_id)
    }

    /// Recall a saved loop region: activate, seek to in, play.
    pub fn recall_deck_saved_loop(
        &mut self,
        deck_id: usize,
        in_ms: i32,
        out_ms: i32,
    ) -> Result<()> {
        if !self.deck_has_audio_loaded(deck_id).unwrap_or(false) {
            return Err(anyhow::anyhow!(
                "Load a track before recalling a saved loop."
            ));
        }
        if out_ms <= in_ms {
            return Err(anyhow::anyhow!(
                "Loop out must be after loop in: in={in_ms} out={out_ms}"
            ));
        }
        // Seek first: the loop activation anchors the slip cursor at the in point.
        self.seek_deck(deck_id, in_ms)?;
        self.set_deck_loop_region(deck_id, in_ms, out_ms)?;
        // Drop the pending loop-in only once the activation succeeded, so a
        // failed recall leaves it intact for a retry.
        if let Some(control) = self.deck_control.get_mut(deck_id) {
            control.pending_loop_in_ms = None;
        }
        self.play(deck_id)
    }

    /// Begin a temporary loop roll; stashes the prior active loop for restore.
    pub fn begin_deck_loop_roll(&mut self, deck_id: usize, beats: f32) -> Result<()> {
        if !beats.is_finite() || beats <= 0.0 {
            return Err(anyhow::anyhow!(
                "Loop roll requires a positive finite beat length."
            ));
        }
        let (bpm, first_beat_ms, quantize) = self.deck_quantize_params(deck_id)?;
        let bpm = bpm.ok_or_else(|| anyhow::anyhow!("Track BPM is required for loop roll."))?;
        let (position_ms, duration_ms) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let restore = self.active_loop_region(deck_id);
        let control = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        control.loop_roll_restore = restore;
        control.pending_loop_in_ms = None;

        let beat_len = 60.0 / bpm;
        let in_ms = snap_ms(position_ms, Some(bpm), first_beat_ms, quantize);
        let in_secs = ms_to_secs(in_ms);
        let duration = ms_to_secs(duration_ms);
        let out_ms = secs_to_ms((in_secs + beat_len * f64::from(beats)).min(duration));
        self.set_deck_loop_region(deck_id, in_ms, out_ms)
    }

    /// End loop roll; restore stashed loop or clear.
    pub fn end_deck_loop_roll(&mut self, deck_id: usize) -> Result<()> {
        let restore = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?
            .loop_roll_restore
            .take();
        if let Some(region) = restore.filter(|region| region.active) {
            self.set_deck_loop_region(deck_id, region.in_ms, region.out_ms)
        } else {
            self.clear_deck_loop(deck_id)
        }
    }

    fn active_loop_region(&self, deck_id: usize) -> Option<LoopRegion> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        deck.loop_region_ms().map(|(in_ms, out_ms)| LoopRegion {
            in_ms,
            out_ms,
            active: true,
        })
    }

    /// Set cue point to the snapped playhead.
    pub fn set_deck_cue_point_at_playhead(&mut self, deck_id: usize) -> Result<()> {
        if !self.deck_has_audio_loaded(deck_id).unwrap_or(false) {
            return Err(anyhow::anyhow!("Load a track before setting cue."));
        }
        let (bpm, first_beat_ms, quantize) = self.deck_quantize_params(deck_id)?;
        let (position_ms, _) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let target = snap_ms(position_ms, bpm, first_beat_ms, quantize);
        self.set_deck_cue_point(deck_id, target)
    }

    /// Auto-loop `beats` from the playhead, snapped to the nearest beat.
    pub fn set_deck_auto_loop(&mut self, deck_id: usize, beats: f32) -> Result<()> {
        if !beats.is_finite() || beats <= 0.0 {
            return Err(anyhow::anyhow!(
                "Loop length must be a positive finite beat count."
            ));
        }
        let (bpm, first_beat_ms, _) = self.deck_quantize_params(deck_id)?;
        let bpm = require_positive_bpm(bpm, "auto loop")?;
        let (position_ms, duration_ms) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let beat_len = 60.0 / bpm;
        let in_ms = snap_ms(position_ms, Some(bpm), first_beat_ms, true);
        let in_secs = ms_to_secs(in_ms);
        let duration = ms_to_secs(duration_ms);
        let out_ms = secs_to_ms((in_secs + beat_len * f64::from(beats)).min(duration));
        if let Some(control) = self.deck_control.get_mut(deck_id) {
            control.pending_loop_in_ms = None;
        }
        self.set_deck_loop_region(deck_id, in_ms, out_ms)
    }

    /// Resolve a caller-supplied loop boundary: clamp, then snap only when Quantize is on.
    fn resolve_loop_boundary_ms(&self, deck_id: usize, position_ms: i32, op: &str) -> Result<i32> {
        let (bpm, first_beat_ms, quantize) = self.deck_quantize_params(deck_id)?;
        let (_, duration_ms) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let clamp = |ms: i32| {
            if duration_ms > 0 {
                ms.clamp(0, duration_ms)
            } else {
                ms.max(0)
            }
        };
        let raw = clamp(position_ms);
        if quantize {
            let bpm = require_positive_bpm(bpm, op)?;
            Ok(clamp(snap_ms(raw, Some(bpm), first_beat_ms, true)))
        } else {
            Ok(raw)
        }
    }

    /// Set manual Loop In: pending when no active region; edit In when a region is active.
    pub fn set_deck_loop_in(&mut self, deck_id: usize, position_ms: i32) -> Result<()> {
        let in_ms = self.resolve_loop_boundary_ms(deck_id, position_ms, "loop in")?;
        if let Some((_, out_ms)) = self
            .deck_transport_state(deck_id)
            .and_then(|(_, loop_region)| loop_region)
        {
            if let Some(control) = self.deck_control.get_mut(deck_id) {
                control.pending_loop_in_ms = None;
            }
            return self.set_deck_loop_region(deck_id, in_ms, out_ms.max(in_ms + 10));
        }
        let control = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        control.pending_loop_in_ms = Some(in_ms);
        Ok(())
    }

    /// Set manual Loop Out: complete pending In, or edit Out of an active region.
    pub fn set_deck_loop_out(&mut self, deck_id: usize, position_ms: i32) -> Result<()> {
        let out_ms = self.resolve_loop_boundary_ms(deck_id, position_ms, "loop out")?;
        let pending_in = self
            .deck_control
            .get(deck_id)
            .and_then(|c| c.pending_loop_in_ms);
        if let Some(in_ms) = pending_in {
            if out_ms <= in_ms {
                return Err(anyhow::anyhow!("Loop out must be after loop in."));
            }
            if let Some(control) = self.deck_control.get_mut(deck_id) {
                control.pending_loop_in_ms = None;
            }
            return self.set_deck_loop_region(deck_id, in_ms, out_ms);
        }
        let in_ms = self
            .deck_transport_state(deck_id)
            .and_then(|(_, loop_region)| loop_region.map(|(inn, _)| inn));
        let Some(in_ms) = in_ms else {
            return Err(anyhow::anyhow!(
                "Set Loop In before Loop Out, or edit an active loop."
            ));
        };
        if out_ms <= in_ms {
            return Err(anyhow::anyhow!("Loop out must be after loop in."));
        }
        self.set_deck_loop_region(deck_id, in_ms, out_ms)
    }

    /// Jump playhead by `beats` (negative = backward), optionally snapped.
    pub fn beat_jump_deck(&mut self, deck_id: usize, beats: f32) -> Result<()> {
        if !beats.is_finite() || beats == 0.0 {
            return Err(anyhow::anyhow!(
                "Beat jump requires a non-zero finite beat count."
            ));
        }
        let (bpm, first_beat_ms, quantize) = self.deck_quantize_params(deck_id)?;
        let bpm = bpm.ok_or_else(|| anyhow::anyhow!("Track BPM is required for beat jump."))?;
        let (position_ms, _) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let beat_len = 60.0 / bpm;
        let raw = ms_to_secs(position_ms) + beat_len * f64::from(beats);
        let target = snap_ms(secs_to_ms(raw), Some(bpm), first_beat_ms, quantize);
        self.seek_deck_audible(deck_id, target)
    }
}
