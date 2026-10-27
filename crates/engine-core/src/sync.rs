//! Tempo/beat sync follow helpers for the engine control path.

use crate::engine::Engine;
use crate::pads::{DEFAULT_PITCH_PAGE, HOT_CUE_SLOT_COUNT};
use anyhow::{Context, Result};
use engine_api::{LoopRegion, PadMode, SyncMode};
use engine_dsp::KEY_SHIFT_SEMITONE_LIMIT;
use library_core::TrackId;

/// Session Keyboard / Key Shift pad state (pages, root, momentary holds).
#[derive(Clone, Debug)]
pub(crate) struct PitchPadState {
    pub key_shift_semitones: f32,
    pub keyboard_page: u8,
    pub key_shift_page: u8,
    pub keyboard_root_hot_cue: u8,
    pub keyboard_restore_semitones: Option<f32>,
    pub keyboard_held: [bool; 8],
    /// Semitone applied when each pad was pressed (stable across mid-hold page changes).
    pub keyboard_held_semitones: [Option<f32>; 8],
}

impl Default for PitchPadState {
    fn default() -> Self {
        Self {
            key_shift_semitones: 0.0,
            keyboard_page: DEFAULT_PITCH_PAGE,
            key_shift_page: DEFAULT_PITCH_PAGE,
            keyboard_root_hot_cue: 0,
            keyboard_restore_semitones: None,
            keyboard_held: [false; 8],
            keyboard_held_semitones: [None; 8],
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DeckControlState {
    pub sync_mode: SyncMode,
    pub bpm: Option<f64>,
    /// First beat of the constant-tempo grid in media milliseconds (phase for [`snap_ms`]).
    pub first_beat_ms: f64,
    pub quantize: bool,
    pub pad_mode: PadMode,
    pub loop_roll_restore: Option<LoopRegion>,
    /// Pending manual Loop In (ms) before Loop Out completes the region.
    pub pending_loop_in_ms: Option<i32>,
    /// Library track id when the deck holds a library-backed (or id'd) load.
    pub track_id: Option<TrackId>,
    /// Filesystem path or stream URI for the loaded source.
    pub track_path: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub key: Option<String>,
    /// International Standard Recording Code when known at load time.
    pub isrc: Option<String>,
    /// Runtime hot-cue positions (library hydrate + in-session save/delete).
    pub hot_cues: [Option<i32>; HOT_CUE_SLOT_COUNT],
    /// Library sampler bank currently loaded onto this deck's pads.
    pub active_sampler_bank_id: Option<String>,
    pub pitch_pad: PitchPadState,
}

impl Default for DeckControlState {
    fn default() -> Self {
        Self {
            sync_mode: SyncMode::Off,
            bpm: None,
            first_beat_ms: 0.0,
            quantize: false,
            pad_mode: PadMode::HotCue,
            loop_roll_restore: None,
            pending_loop_in_ms: None,
            track_id: None,
            track_path: None,
            title: None,
            artist: None,
            album: None,
            key: None,
            isrc: None,
            hot_cues: [None; HOT_CUE_SLOT_COUNT],
            active_sampler_bank_id: None,
            pitch_pad: PitchPadState::default(),
        }
    }
}

impl DeckControlState {
    /// Reset the Keyboard / Key Shift session state shared by `Default`,
    /// [`Self::clear_loaded_track`], and [`Self::apply_loaded_metadata`].
    ///
    /// Keeping the pitch-pad fields in one place avoids drift when a new field is
    /// added to only one of the three initialisation sites.
    fn reset_pitch_pad_state(&mut self) {
        self.pitch_pad = PitchPadState::default();
    }

    pub fn clear_loaded_track(&mut self) {
        self.bpm = None;
        self.first_beat_ms = 0.0;
        self.sync_mode = SyncMode::Off;
        self.loop_roll_restore = None;
        self.pending_loop_in_ms = None;
        self.track_id = None;
        self.track_path = None;
        self.title = None;
        self.artist = None;
        self.album = None;
        self.key = None;
        self.isrc = None;
        self.hot_cues = [None; HOT_CUE_SLOT_COUNT];
        self.reset_pitch_pad_state();
    }

    pub fn apply_source_load(&mut self, source: &library_core::AudioSource, track_id: TrackId) {
        self.apply_loaded_metadata(track_id, source.source_ref(), source.metadata());
    }

    pub fn apply_loaded_metadata(
        &mut self,
        track_id: TrackId,
        track_path: String,
        metadata: &library_core::TrackMetadata,
    ) {
        self.bpm = metadata.bpm.filter(|b| b.is_finite() && *b > 0.0);
        self.first_beat_ms = 0.0;
        self.sync_mode = SyncMode::Off;
        self.loop_roll_restore = None;
        self.pending_loop_in_ms = None;
        self.track_id = Some(track_id);
        self.track_path = Some(track_path);
        self.title = non_empty_opt(metadata.title.clone());
        self.artist = non_empty_opt(metadata.artist.clone());
        self.album = non_empty_opt(metadata.album.clone());
        self.key = non_empty_opt(metadata.key.clone());
        self.isrc = non_empty_opt(metadata.isrc.clone());
        self.hot_cues = [None; HOT_CUE_SLOT_COUNT];
        // A newly loaded track must not inherit a stale session shift/page.
        self.reset_pitch_pad_state();
    }
}

fn non_empty_opt(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.is_empty())
}

/// Beat length in milliseconds for a given BPM.
fn beat_ms(bpm: f64) -> f64 {
    60_000.0 / bpm
}

/// Require a finite BPM > 0 for loop / beat-timed ops.
pub(crate) fn require_positive_bpm(bpm: Option<f64>, op: &str) -> anyhow::Result<f64> {
    match bpm {
        Some(b) if b.is_finite() && b > 0.0 => Ok(b),
        _ => Err(anyhow::anyhow!("Track BPM is required for {op}.")),
    }
}

/// Snap media time to the nearest beat when quantize is on. Stays in ms end-to-end.
///
/// `first_beat_ms` is the grid phase (constant-tempo first beat), matching the
/// waveform beat lines painted by the UI.
pub(crate) fn snap_ms(ms: i32, bpm: Option<f64>, first_beat_ms: f64, quantize: bool) -> i32 {
    if !quantize {
        return ms;
    }
    let Some(bpm) = bpm else {
        return ms;
    };
    if bpm <= 0.0 {
        return ms;
    }
    let phase = first_beat_ms;
    // Bus/grid payloads can carry NaN/negative phase; refuse to snap rather than
    // emit NaN or negative beat times.
    if !phase.is_finite() || phase < 0.0 {
        return ms;
    }
    let beat = beat_ms(bpm);
    (((f64::from(ms) - phase) / beat).round() * beat + phase).round() as i32
}

pub(crate) fn target_sync_speed(master_bpm: f64, master_speed: f32, slave_bpm: f64) -> f32 {
    let master_effective = master_bpm * f64::from(master_speed);
    ((master_effective / slave_bpm) as f32).clamp(0.5, 2.0)
}

#[allow(clippy::too_many_arguments)] // master/slave pos+bpm+phase are parallel inputs
pub(crate) fn beat_align_target(
    master_pos_ms: i32,
    slave_pos_ms: i32,
    duration_ms: i32,
    master_bpm: f64,
    slave_bpm: f64,
    master_first_beat_ms: f64,
    slave_first_beat_ms: f64,
    quantize: bool,
) -> i32 {
    let master_pos = f64::from(master_pos_ms);
    let slave_pos = f64::from(slave_pos_ms);
    let master_beat = beat_ms(master_bpm);
    let slave_beat = beat_ms(slave_bpm);
    // Phase relative to each deck's painted grid (not media t=0).
    let master_phase = (master_pos - master_first_beat_ms).rem_euclid(master_beat);

    let slave_beat_index = ((slave_pos - slave_first_beat_ms) / slave_beat).floor();
    let mut target = slave_beat_index * slave_beat + slave_first_beat_ms + master_phase;
    if target + slave_beat * 0.5 < slave_pos {
        target += slave_beat;
    }

    let snapped = snap_ms(
        target.round() as i32,
        Some(slave_bpm),
        slave_first_beat_ms,
        quantize,
    );
    // Clamp to valid media time: negative playheads must not seek before 0.
    if duration_ms > 0 {
        snapped.clamp(0, duration_ms)
    } else {
        snapped.max(0)
    }
}

impl Engine {
    /// Set tempo fader position for a deck (`0..1`, center = track BPM).
    ///
    /// When the deck is master, synced slaves follow tempo (not beat phase).
    /// Returns every deck whose speed changed (master first).
    pub fn set_deck_speed(&mut self, deck_id: usize, speed: f32) -> Result<Vec<usize>> {
        if !(0.0..=1.0).contains(&speed) {
            return Err(anyhow::anyhow!("Speed must be between 0 and 1."));
        }
        self.set_deck_speed_raw(deck_id, speed)?;
        let mut updated = vec![deck_id];
        if deck_id == self.master_deck {
            for slave_id in 0..self.deck_control.len() {
                if slave_id == deck_id {
                    continue;
                }
                if self.deck_control[slave_id].sync_mode == SyncMode::Off {
                    continue;
                }
                self.apply_tempo_sync(slave_id, deck_id)?;
                updated.push(slave_id);
            }
        }
        Ok(updated)
    }

    /// Set tempo fader half-span (pitch fraction, e.g. `0.06` = ±6%).
    ///
    /// Keeps fader position; remaps displayed speed when synced. If this deck is
    /// master, re-applies tempo sync so slaves follow the new master ratio.
    pub fn set_deck_tempo_range(&mut self, deck_id: usize, tempo_range: f32) -> Result<Vec<usize>> {
        if !tempo_range.is_finite() || tempo_range <= 0.0 {
            return Err(anyhow::anyhow!("tempo_range must be finite and > 0"));
        }
        {
            let dsp_engine = self
                .dsp_engine
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
            let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
            let deck = dsp
                .deck_mut(deck_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
            deck.set_tempo_range(tempo_range);
        }
        let mut updated = vec![deck_id];
        if deck_id == self.master_deck {
            for slave_id in 0..self.deck_control.len() {
                if slave_id == deck_id {
                    continue;
                }
                if self.deck_control[slave_id].sync_mode == SyncMode::Off {
                    continue;
                }
                self.apply_tempo_sync(slave_id, deck_id)?;
                updated.push(slave_id);
            }
        }
        Ok(updated)
    }

    /// Enable/disable key lock (time-stretch) for a deck.
    pub fn set_deck_key_lock(&mut self, deck_id: usize, enabled: bool) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        deck.set_key_lock(enabled)
    }

    /// Set the session key-shift offset in semitones
    /// (`±KEY_SHIFT_SEMITONE_LIMIT`).
    ///
    /// Out-of-range values are clamped; non-finite values become `0`.
    pub fn set_deck_key_shift(&mut self, deck_id: usize, semitones: f32) -> Result<()> {
        let s = if semitones.is_finite() {
            semitones.clamp(-KEY_SHIFT_SEMITONE_LIMIT, KEY_SHIFT_SEMITONE_LIMIT)
        } else {
            0.0
        };
        // Build the stretcher before mutating any state and before taking the
        // DSP lock. Construction is heavy: doing it under the lock would stall
        // the producer thread that shares the mutex, and Deck's ensure path
        // would repeat that stall on failure.
        let prepared = if s == 0.0 {
            None
        } else if let Some(dsp_engine) = self.dsp_engine.as_ref() {
            let spec = {
                let dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
                dsp.deck(deck_id).and_then(|deck| deck.stretcher_spec())
            };
            match spec.map(|spec| spec.build()) {
                Some(Ok(prepared)) => Some(prepared),
                Some(Err(err)) => {
                    return Err(err).with_context(|| {
                        format!("Deck {deck_id}: could not build the key-shift stretcher")
                    });
                }
                None => None,
            }
        } else {
            None
        };
        {
            let control = self
                .deck_control
                .get_mut(deck_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
            control.pitch_pad.key_shift_semitones = s;
        }
        if let Some(dsp_engine) = self.dsp_engine.as_ref() {
            let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
            if let Some(deck) = dsp.deck_mut(deck_id) {
                if let Some(prepared) = prepared {
                    deck.install_stretcher(prepared);
                }
                deck.set_key_shift_semitones(s)?;
            }
        }
        Ok(())
    }

    /// Set the Keyboard pad semitone page for a deck (`1..=4`).
    ///
    /// Out-of-range values are clamped.
    pub fn set_deck_keyboard_page(&mut self, deck_id: usize, page: u8) -> Result<()> {
        let control = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        control.pitch_pad.keyboard_page = page.clamp(1, crate::pads::KEYBOARD_PAGE_COUNT);
        Ok(())
    }

    /// Set the Key Shift pad semitone page for a deck (`1..=5`).
    ///
    /// Out-of-range values are clamped.
    pub fn set_deck_key_shift_page(&mut self, deck_id: usize, page: u8) -> Result<()> {
        let control = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        control.pitch_pad.key_shift_page = page.clamp(1, crate::pads::KEY_SHIFT_PAGE_COUNT);
        Ok(())
    }

    /// Set the hot-cue slot used as the Keyboard pad root for a deck (`0..=15`).
    ///
    /// The root may be any hot cue in the 16-slot domain; out-of-range values
    /// are clamped.
    pub fn set_deck_keyboard_root(&mut self, deck_id: usize, slot: u8) -> Result<()> {
        let control = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        control.pitch_pad.keyboard_root_hot_cue =
            slot.min((crate::pads::HOT_CUE_SLOT_COUNT - 1) as u8);
        Ok(())
    }

    fn set_deck_speed_raw(&mut self, deck_id: usize, speed: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_speed(speed)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    fn set_deck_playback_ratio_raw(&mut self, deck_id: usize, ratio: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_playback_ratio(ratio)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    pub(crate) fn apply_tempo_sync(&mut self, slave_id: usize, master_id: usize) -> Result<()> {
        if slave_id == master_id {
            return Err(anyhow::anyhow!("Cannot sync a deck to itself."));
        }
        let slave_bpm = self
            .deck_control
            .get(slave_id)
            .and_then(|d| d.bpm)
            .ok_or_else(|| anyhow::anyhow!("Slave deck BPM is required for sync."))?;
        let master_bpm = self
            .deck_control
            .get(master_id)
            .and_then(|d| d.bpm)
            .ok_or_else(|| anyhow::anyhow!("Master deck BPM is required for sync."))?;
        let master_ratio = {
            let dsp = self
                .dsp_engine
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?
                .lock()
                .expect("DSP engine mutex poisoned");
            dsp.deck(master_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid master deck ID: {master_id}"))?
                .playback_ratio()
        };
        let target = target_sync_speed(master_bpm, master_ratio, slave_bpm);
        self.set_deck_playback_ratio_raw(slave_id, target)
    }

    fn align_beat_phase(&mut self, slave_id: usize, master_id: usize) -> Result<()> {
        let (master_bpm, master_first_beat_ms) = {
            let control = self
                .deck_control
                .get(master_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid master deck ID: {master_id}"))?;
            let bpm = control
                .bpm
                .ok_or_else(|| anyhow::anyhow!("Master deck BPM is required for beat sync."))?;
            (bpm, control.first_beat_ms)
        };
        let (slave_bpm, slave_first_beat_ms, quantize) = {
            let control = self
                .deck_control
                .get(slave_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid slave deck ID: {slave_id}"))?;
            let bpm = control
                .bpm
                .ok_or_else(|| anyhow::anyhow!("Slave deck BPM is required for beat sync."))?;
            (bpm, control.first_beat_ms, control.quantize)
        };
        let (master_pos, _) = self.deck_playback_ms(master_id).unwrap_or((0, 0));
        let (slave_pos, duration) = self.deck_playback_ms(slave_id).unwrap_or((0, 0));
        let target = beat_align_target(
            master_pos,
            slave_pos,
            duration,
            master_bpm,
            slave_bpm,
            master_first_beat_ms,
            slave_first_beat_ms,
            quantize,
        );
        self.seek_deck(slave_id, target)
    }

    /// Toggle sync for a slave deck (`beat_sync` chooses beat vs tempo when enabling).
    pub fn toggle_deck_sync(&mut self, deck_id: usize, beat_sync: bool) -> Result<Vec<usize>> {
        if deck_id >= self.deck_control.len() {
            return Err(anyhow::anyhow!("Invalid deck ID: {deck_id}"));
        }
        if !self.deck_has_audio_loaded(deck_id).unwrap_or(false) {
            return Err(anyhow::anyhow!("Load a track before enabling sync."));
        }
        let master_id = self.master_deck;
        if deck_id == master_id {
            return Err(anyhow::anyhow!(
                "Master deck cannot sync to itself. Choose the other deck."
            ));
        }

        let next_mode = if self.deck_control[deck_id].sync_mode == SyncMode::Off {
            if beat_sync {
                SyncMode::Beat
            } else {
                SyncMode::Tempo
            }
        } else {
            SyncMode::Off
        };
        self.deck_control[deck_id].sync_mode = next_mode;

        let updated = vec![deck_id];
        if next_mode != SyncMode::Off {
            self.apply_tempo_sync(deck_id, master_id)?;
            if next_mode == SyncMode::Beat {
                self.align_beat_phase(deck_id, master_id)?;
            }
        }
        Ok(updated)
    }

    /// Designate the master deck and re-apply sync to all active slaves.
    pub fn set_master_deck(&mut self, deck_id: usize) -> Result<Vec<usize>> {
        if deck_id >= self.deck_control.len() {
            return Err(anyhow::anyhow!("Invalid deck ID: {deck_id}"));
        }
        self.master_deck = deck_id;
        if let Some(dsp_engine) = self.dsp_engine.as_ref() {
            if let Ok(mut dsp) = dsp_engine.lock() {
                dsp.mixer_mut().set_master_deck(deck_id);
            }
        }
        let mut updated = Vec::with_capacity(self.deck_control.len());
        for id in 0..self.deck_control.len() {
            updated.push(id);
        }
        for slave_id in 0..self.deck_control.len() {
            if slave_id == deck_id {
                continue;
            }
            let mode = self.deck_control[slave_id].sync_mode;
            if mode == SyncMode::Off {
                continue;
            }
            self.apply_tempo_sync(slave_id, deck_id)?;
            if mode == SyncMode::Beat {
                self.align_beat_phase(slave_id, deck_id)?;
            }
        }
        Ok(updated)
    }

    pub fn master_deck(&self) -> usize {
        self.master_deck
    }

    pub fn deck_sync_mode(&self, deck_id: usize) -> Option<SyncMode> {
        self.deck_control.get(deck_id).map(|d| d.sync_mode)
    }

    /// Mirror UI quantize into engine control state (beat-sync snap).
    pub fn set_deck_quantize(&mut self, deck_id: usize, enabled: bool) -> Result<()> {
        let control = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?;
        control.quantize = enabled;
        Ok(())
    }

    pub fn deck_quantize(&self, deck_id: usize) -> Option<bool> {
        self.deck_control.get(deck_id).map(|c| c.quantize)
    }

    pub(crate) fn deck_quantize_params(&self, deck_id: usize) -> Result<(Option<f64>, f64, bool)> {
        let control = self
            .deck_control
            .get(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?;
        Ok((control.bpm, control.first_beat_ms, control.quantize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_sync_speed_matches_master_effective_tempo() {
        // Master 120 BPM at 1.0 → slave 100 BPM needs 1.2×
        assert!((target_sync_speed(120.0, 1.0, 100.0) - 1.2).abs() < f32::EPSILON);
    }

    #[test]
    fn snap_ms_without_quantize_passes_through_including_negatives() {
        assert_eq!(snap_ms(500, Some(120.0), 0.0, false), 500);
        assert_eq!(snap_ms(-100, None, 0.0, false), -100);
    }

    #[test]
    fn snap_ms_quantizes_to_nearest_beat_in_ms() {
        // 120 BPM → 500 ms per beat; 620 → 500, 760 → 1000
        assert_eq!(snap_ms(620, Some(120.0), 0.0, true), 500);
        assert_eq!(snap_ms(760, Some(120.0), 0.0, true), 1000);
    }

    #[test]
    fn snap_ms_respects_first_beat_phase() {
        // 120 BPM, phase 100 ms → beats at 100, 600, 1100…
        assert_eq!(snap_ms(200, Some(120.0), 100.0, true), 100);
        assert_eq!(snap_ms(400, Some(120.0), 100.0, true), 600);
    }

    #[test]
    fn snap_ms_passes_through_when_phase_is_invalid() {
        assert_eq!(snap_ms(200, Some(120.0), f64::NAN, true), 200);
        assert_eq!(snap_ms(200, Some(120.0), -50.0, true), 200);
    }

    #[test]
    fn beat_align_target_clamps_negative_slave_position_to_zero() {
        // Negative slave playhead must not yield a negative seek target.
        let target = beat_align_target(
            250,  // master mid-beat at 120 BPM
            -100, // slave before 0
            10_000, 120.0, 120.0, 0.0, 0.0, false,
        );
        assert!(target >= 0, "expected non-negative target, got {target}");
    }

    #[test]
    fn require_positive_bpm_rejects_missing_zero_and_non_finite() {
        assert!(require_positive_bpm(Some(128.0), "auto loop").is_ok());
        assert!(require_positive_bpm(None, "auto loop").is_err());
        assert!(require_positive_bpm(Some(0.0), "auto loop").is_err());
        assert!(require_positive_bpm(Some(-1.0), "loop in").is_err());
        assert!(require_positive_bpm(Some(f64::NAN), "loop out").is_err());
        assert!(require_positive_bpm(Some(f64::INFINITY), "auto loop").is_err());
        let err = require_positive_bpm(None, "loop in")
            .unwrap_err()
            .to_string();
        assert!(err.contains("loop in"));
    }
}
