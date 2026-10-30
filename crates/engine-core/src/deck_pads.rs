use anyhow::Result;
use bus_api::PadMode;

use crate::engine::Engine;

impl Engine {
    /// Generic pad press: dispatch using this deck's engine `pad_mode`.
    pub fn pad_press(&mut self, deck_id: usize, slot: u8, shift: bool) -> Result<()> {
        match self.deck_pad_mode(deck_id)? {
            PadMode::HotCue => self.hot_cue_pad_press(deck_id, slot, shift),
            PadMode::LoopRoll => self.loop_roll_pad_press(deck_id, slot),
            PadMode::BeatJump => self.beat_jump_pad_press(deck_id, slot),
            PadMode::Sampler => self.sampler_pad_press(deck_id, slot, shift),
            PadMode::Stems => self.stems_pad_press(deck_id, slot),
            PadMode::Keyboard => self.keyboard_pad_press(deck_id, slot, shift),
            PadMode::KeyShift => self.key_shift_pad_press(deck_id, slot, shift),
        }
    }

    pub fn pad_release(&mut self, deck_id: usize, slot: u8) -> Result<()> {
        match self.deck_pad_mode(deck_id)? {
            PadMode::HotCue => self.hot_cue_pad_release(deck_id, slot),
            PadMode::LoopRoll => self.loop_roll_pad_release(deck_id, slot),
            PadMode::BeatJump => self.beat_jump_pad_release(deck_id, slot),
            PadMode::Sampler => self.sampler_pad_release(deck_id, slot),
            PadMode::Stems => Ok(()),
            PadMode::Keyboard => self.keyboard_pad_release(deck_id, slot),
            PadMode::KeyShift => self.key_shift_pad_release(deck_id, slot),
        }
    }

    fn deck_pad_mode(&self, deck_id: usize) -> Result<PadMode> {
        self.deck_control
            .get(deck_id)
            .map(|c| c.pad_mode)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))
    }

    /// Hot-cue pad press: save, trigger, or delete from engine-owned slot state.
    pub fn hot_cue_pad_press(&mut self, deck_id: usize, slot: u8, shift: bool) -> Result<()> {
        if usize::from(slot) >= crate::pads::HOT_CUE_SLOT_COUNT {
            return Err(anyhow::anyhow!(
                "Hot cue slot must be 0..={}.",
                crate::pads::HOT_CUE_SLOT_COUNT - 1
            ));
        }
        let filled = self
            .deck_control
            .get(deck_id)
            .and_then(|c| c.hot_cues.get(usize::from(slot)).copied())
            .flatten();
        if shift {
            if filled.is_some() {
                self.delete_deck_hot_cue(deck_id, slot)?;
            }
            return Ok(());
        }
        if let Some(position_ms) = filled {
            return self.trigger_deck_hot_cue(deck_id, position_ms);
        }
        self.save_deck_hot_cue(deck_id, slot)
    }

    pub fn hot_cue_pad_release(&mut self, _deck_id: usize, _slot: u8) -> Result<()> {
        Ok(())
    }

    pub fn loop_roll_pad_press(&mut self, deck_id: usize, slot: u8) -> Result<()> {
        let beats = crate::pads::loop_roll_beats(slot)
            .ok_or_else(|| anyhow::anyhow!("Loop roll slot must be 0..=7."))?;
        self.begin_deck_loop_roll(deck_id, beats)
    }

    pub fn loop_roll_pad_release(&mut self, deck_id: usize, _slot: u8) -> Result<()> {
        self.end_deck_loop_roll(deck_id)
    }

    pub fn beat_jump_pad_press(&mut self, deck_id: usize, slot: u8) -> Result<()> {
        let beats = crate::pads::beat_jump_beats(slot)
            .ok_or_else(|| anyhow::anyhow!("Beat jump slot must be 0..=7."))?;
        self.beat_jump_deck(deck_id, beats)
    }

    pub fn beat_jump_pad_release(&mut self, _deck_id: usize, _slot: u8) -> Result<()> {
        Ok(())
    }

    pub fn sampler_pad_press(&mut self, deck_id: usize, slot: u8, shift: bool) -> Result<()> {
        let slot_i = slot as usize;
        if shift {
            if self.sampler_slot_assigned(deck_id, slot_i) {
                self.clear_sampler_slot(deck_id, slot_i)?;
            }
            return Ok(());
        }
        self.trigger_sampler(deck_id, slot_i)
    }

    pub fn sampler_pad_release(&mut self, deck_id: usize, slot: u8) -> Result<()> {
        self.end_sampler(deck_id, slot as usize)
    }

    /// Key Shift pad: press applies the page action and latches it. `shift` uses
    /// the shift bank: slot 6 → next page, slot 7 → previous page, else no-op.
    pub fn key_shift_pad_press(&mut self, deck_id: usize, slot: u8, shift: bool) -> Result<()> {
        if usize::from(slot) >= 8 {
            return Err(anyhow::anyhow!("Key shift pad slot must be 0..=7."));
        }
        let (page, current) = {
            let control = self
                .deck_control
                .get(deck_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
            (
                control.pitch_pad.key_shift_page,
                control.pitch_pad.key_shift_semitones,
            )
        };
        if shift {
            return match slot {
                6 => self.set_deck_key_shift_page(deck_id, crate::pads::key_shift_page_next(page)),
                7 => self.set_deck_key_shift_page(deck_id, crate::pads::key_shift_page_prev(page)),
                _ => Ok(()),
            };
        }
        match crate::pads::key_shift_page_action(page, slot) {
            crate::pads::PitchPadAction::Semitone(s) => {
                self.set_deck_key_shift(deck_id, f32::from(s))
            }
            crate::pads::PitchPadAction::KeyReset => self.set_deck_key_shift(deck_id, 0.0),
            crate::pads::PitchPadAction::SemitoneUp => {
                self.set_deck_key_shift(deck_id, current + 1.0)
            }
            crate::pads::PitchPadAction::SemitoneDown => {
                self.set_deck_key_shift(deck_id, current - 1.0)
            }
            crate::pads::PitchPadAction::KeySync | crate::pads::PitchPadAction::None => Ok(()),
        }
    }

    pub fn key_shift_pad_release(&mut self, _deck_id: usize, _slot: u8) -> Result<()> {
        Ok(())
    }

    /// Keyboard pad: the selected hot cue is the root. Press seeks to the root and
    /// applies the page's semitone offset (momentary/gate); `shift` uses the shift
    /// bank: slots 0–5 delete that hot cue, slot 6 → next page, slot 7 → previous.
    pub fn keyboard_pad_press(&mut self, deck_id: usize, slot: u8, shift: bool) -> Result<()> {
        if usize::from(slot) >= 8 {
            return Err(anyhow::anyhow!("Keyboard pad slot must be 0..=7."));
        }
        if shift {
            return match slot {
                0..=5 => self.delete_deck_hot_cue(deck_id, slot),
                6 => {
                    let page = self.keyboard_page(deck_id)?;
                    self.set_deck_keyboard_page(deck_id, crate::pads::keyboard_page_next(page))
                }
                7 => {
                    let page = self.keyboard_page(deck_id)?;
                    self.set_deck_keyboard_page(deck_id, crate::pads::keyboard_page_prev(page))
                }
                _ => Ok(()),
            };
        }
        let slot_i = usize::from(slot);
        let (page, first_press, current_shift, root_position) = {
            let control = self
                .deck_control
                .get(deck_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
            (
                control.pitch_pad.keyboard_page,
                !control.pitch_pad.keyboard_held.iter().any(|held| *held),
                control.pitch_pad.key_shift_semitones,
                control
                    .hot_cues
                    .get(usize::from(control.pitch_pad.keyboard_root_hot_cue))
                    .copied()
                    .flatten(),
            )
        };
        match crate::pads::keyboard_page_action(page, slot) {
            crate::pads::PitchPadAction::Semitone(s) => {
                {
                    let control = self
                        .deck_control
                        .get_mut(deck_id)
                        .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
                    if first_press {
                        // Snapshot whatever was latched (e.g. a Key Shift pad) so
                        // the momentary Keyboard bank can put it back on release.
                        control.pitch_pad.keyboard_restore_semitones = Some(current_shift);
                    }
                    control.pitch_pad.keyboard_held[slot_i] = true;
                    control.pitch_pad.keyboard_held_semitones[slot_i] = Some(f32::from(s));
                }
                if let Some(position_ms) = root_position {
                    self.trigger_deck_hot_cue(deck_id, position_ms)?;
                }
                self.set_deck_key_shift(deck_id, f32::from(s))
            }
            crate::pads::PitchPadAction::KeyReset => self.set_deck_key_shift(deck_id, 0.0),
            crate::pads::PitchPadAction::SemitoneUp => {
                self.set_deck_key_shift(deck_id, current_shift + 1.0)
            }
            crate::pads::PitchPadAction::SemitoneDown => {
                self.set_deck_key_shift(deck_id, current_shift - 1.0)
            }
            crate::pads::PitchPadAction::KeySync | crate::pads::PitchPadAction::None => Ok(()),
        }
    }

    /// Keyboard pad release: momentary gate. Fall back to another held pad's page
    /// semitone, else restore the pre-press key shift and seek back to the root hot
    /// cue. A release for a pad that was never held is a no-op.
    pub fn keyboard_pad_release(&mut self, deck_id: usize, slot: u8) -> Result<()> {
        let slot_i = usize::from(slot);
        let mut seek_back: Option<i32> = None;
        let target = {
            let control = self
                .deck_control
                .get_mut(deck_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
            let was_held = control
                .pitch_pad
                .keyboard_held
                .get(slot_i)
                .copied()
                .unwrap_or(false);
            if was_held {
                control.pitch_pad.keyboard_held[slot_i] = false;
                control.pitch_pad.keyboard_held_semitones[slot_i] = None;
            }
            let fallback = control
                .pitch_pad
                .keyboard_held
                .iter()
                .enumerate()
                .filter(|(_, held)| **held)
                .find_map(|(index, _)| control.pitch_pad.keyboard_held_semitones[index]);
            match (was_held, fallback) {
                (true, Some(semitones)) => Some(semitones),
                (true, None) => {
                    seek_back = control
                        .hot_cues
                        .get(usize::from(control.pitch_pad.keyboard_root_hot_cue))
                        .copied()
                        .flatten();
                    Some(
                        control
                            .pitch_pad
                            .keyboard_restore_semitones
                            .take()
                            .unwrap_or(0.0),
                    )
                }
                (false, _) => None,
            }
        };
        if let Some(semitones) = target {
            self.set_deck_key_shift(deck_id, semitones)?;
        }
        if let Some(position_ms) = seek_back {
            self.trigger_deck_hot_cue(deck_id, position_ms)?;
        }
        Ok(())
    }

    fn keyboard_page(&self, deck_id: usize) -> Result<u8> {
        self.deck_control
            .get(deck_id)
            .map(|control| control.pitch_pad.keyboard_page)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))
    }

    /// Set controller pad mode for a deck (UI mode; no audio side effects).
    pub fn set_deck_pad_mode(&mut self, deck_id: usize, mode: PadMode) -> Result<()> {
        let restore = {
            let control = self
                .deck_control
                .get_mut(deck_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
            // A mid-hold mode switch must not leave stale momentary Keyboard
            // state: the held pad's release is routed to the new mode's release
            // handler, so restore the pre-press shift now (and update the DSP deck
            // below) before clearing the held bookkeeping.
            let held = control.pitch_pad.keyboard_held.iter().any(|held| *held);
            let restore = held.then(|| {
                control
                    .pitch_pad
                    .keyboard_restore_semitones
                    .take()
                    .unwrap_or(0.0)
            });
            control.pad_mode = mode;
            control.pitch_pad.keyboard_held = [false; 8];
            control.pitch_pad.keyboard_held_semitones = [None; 8];
            control.pitch_pad.keyboard_restore_semitones = None;
            restore
        };
        if let Some(semitones) = restore {
            self.set_deck_key_shift(deck_id, semitones)?;
        }
        Ok(())
    }
}
