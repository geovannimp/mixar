//! Action string → `(Origin, Kind, CmdBody)`.

use engine_api::{CmdBody, JogMode, Kind, Origin, PadMode};

use crate::device::{origin_deck_id, SECTION_MASTER, SECTION_SAMPLER};
use crate::error::LoadError;

/// Engine control values mirrored for soft-takeover / action context.
#[derive(Clone, Debug, Default)]
pub struct ControlSnapshot {
    pub playing: [bool; 4],
    pub volume: [f32; 4],
    pub filter_db: [f32; 4],
    pub gain_db: [f32; 4],
    pub eq_low: [f32; 4],
    pub eq_mid: [f32; 4],
    pub eq_high: [f32; 4],
    pub headphone_cue: [bool; 4],
    pub quantize: [bool; 4],
    pub crossfader: f32,
    pub cue_mix: f32,
    pub master_cue: bool,
    /// Hot cue positions ms per deck/slot (None = empty).
    pub hot_cues: [[Option<i32>; 8]; 4],
}

impl ControlSnapshot {
    pub fn set_value(&mut self, origin: Origin, key: &str, value: f32) {
        match origin {
            Origin::Mixer => match key {
                "crossfader" => self.crossfader = value,
                "cue_mix" => self.cue_mix = value,
                "master_cue" => self.master_cue = value > 0.5,
                _ => {}
            },
            Origin::Deck(d) => {
                let i = deck_idx(d);
                match key {
                    "volume" => self.volume[i] = value,
                    "filter" | "filter_db" => self.filter_db[i] = value,
                    "gain" | "gain_db" => self.gain_db[i] = value,
                    "eq_low" => self.eq_low[i] = value,
                    "eq_mid" => self.eq_mid[i] = value,
                    "eq_high" => self.eq_high[i] = value,
                    "playing" => self.playing[i] = value > 0.5,
                    "headphone_cue" => self.headphone_cue[i] = value > 0.5,
                    "quantize" => self.quantize[i] = value > 0.5,
                    _ => {}
                }
            }
            Origin::Engine => {}
        }
    }

    pub fn get_norm_for_action(&self, origin: Origin, action: &str) -> Option<f32> {
        match origin {
            Origin::Mixer => match action {
                "set_crossfader" => Some(self.crossfader),
                "set_cue_mix" => Some(self.cue_mix),
                _ => None,
            },
            Origin::Deck(d) => {
                let i = deck_idx(d);
                match action {
                    "set_volume" => Some(self.volume[i]),
                    // filter_db mirrored as normalized -1..1 mapped externally; store as 0..1-ish
                    "set_filter" => Some(filter_db_to_norm(self.filter_db[i])),
                    "set_gain" => Some(gain_db_to_norm(self.gain_db[i])),
                    "set_eq_low" => Some(eq_to_norm(self.eq_low[i])),
                    "set_eq_mid" => Some(eq_to_norm(self.eq_mid[i])),
                    "set_eq_high" => Some(eq_to_norm(self.eq_high[i])),
                    _ => None,
                }
            }
            Origin::Engine => None,
        }
    }
}

fn deck_idx(d: u16) -> usize {
    (d as usize).min(3)
}

/// Map filter_db (-1..+1 typical) to 0..1 for soft-takeover compare.
fn filter_db_to_norm(db: f32) -> f32 {
    ((db + 1.0) * 0.5).clamp(0.0, 1.0)
}

fn norm_to_filter_db(n: f32) -> f32 {
    n.clamp(0.0, 1.0) * 2.0 - 1.0
}

fn gain_db_to_norm(db: f32) -> f32 {
    // ponytail: ±12 dB span → 0..1; widen if engine range differs
    ((db + 12.0) / 24.0).clamp(0.0, 1.0)
}

fn norm_to_gain_db(n: f32) -> f32 {
    n.clamp(0.0, 1.0) * 24.0 - 12.0
}

fn eq_to_norm(g: f32) -> f32 {
    ((g + 1.0) * 0.5).clamp(0.0, 1.0)
}

fn norm_to_eq(n: f32) -> f32 {
    n.clamp(0.0, 1.0) * 2.0 - 1.0
}

pub fn origin_for_section(section: &str) -> Result<Origin, LoadError> {
    if section == SECTION_MASTER {
        return Ok(Origin::Mixer);
    }
    if section == SECTION_SAMPLER {
        // Sampler cmds still use deck origin in engine today for per-deck slots;
        // v1 uses Deck(0) as default bank deck when section is sampler-global.
        return Ok(Origin::Deck(0));
    }
    if let Some(id) = origin_deck_id(section) {
        return Ok(Origin::Deck(id));
    }
    Err(LoadError::Validation(format!(
        "cannot derive Origin for section `{section}`"
    )))
}

/// Resolve action to cmd. `norm` is MIDI 0..1; `active` for buttons.
pub fn resolve_action(
    action: &str,
    origin: Origin,
    norm: f32,
    active: bool,
    snap: &ControlSnapshot,
) -> Option<(Origin, Kind, CmdBody)> {
    // Buttons: only fire on press (active edge handled by caller).
    match action {
        "toggle_play" => {
            if !active {
                return None;
            }
            let playing = match origin {
                Origin::Deck(d) => snap.playing[deck_idx(d)],
                _ => false,
            };
            if playing {
                Some((origin, Kind::Pause, CmdBody::Empty))
            } else {
                Some((origin, Kind::Play, CmdBody::Empty))
            }
        }
        "play" => active.then_some((origin, Kind::Play, CmdBody::Empty)),
        "pause" => active.then_some((origin, Kind::Pause, CmdBody::Empty)),
        "cue" => active.then_some((origin, Kind::SetCuePoint, CmdBody::Empty)),
        "begin_cue_hold" => active.then_some((origin, Kind::BeginCueHold, CmdBody::Empty)),
        "end_cue_hold" => (!active).then_some((origin, Kind::EndCueHold, CmdBody::Empty)),
        "toggle_sync" => active.then_some((
            origin,
            Kind::ToggleSync,
            CmdBody::ToggleSync { beat_sync: false },
        )),
        "set_quantize" => {
            if !active {
                return None;
            }
            let enabled = match origin {
                Origin::Deck(d) => !snap.quantize[deck_idx(d)],
                _ => true,
            };
            Some((origin, Kind::SetQuantize, CmdBody::SetQuantize { enabled }))
        }
        "set_volume" => Some((
            origin,
            Kind::SetVolume,
            CmdBody::SetVolume {
                volume: norm.clamp(0.0, 1.0),
            },
        )),
        "set_filter" => Some((
            origin,
            Kind::SetFilter,
            CmdBody::SetFilter {
                filter_db: norm_to_filter_db(norm),
            },
        )),
        "set_gain" => Some((
            origin,
            Kind::SetGainTrim,
            CmdBody::SetGainTrim {
                gain_db: norm_to_gain_db(norm),
            },
        )),
        "set_eq_low" | "set_eq_mid" | "set_eq_high" => {
            let Origin::Deck(d) = origin else {
                return None;
            };
            let i = deck_idx(d);
            let mut low = snap.eq_low[i];
            let mut mid = snap.eq_mid[i];
            let mut high = snap.eq_high[i];
            let v = norm_to_eq(norm);
            match action {
                "set_eq_low" => low = v,
                "set_eq_mid" => mid = v,
                "set_eq_high" => high = v,
                _ => {}
            }
            Some((origin, Kind::SetEq, CmdBody::SetEq { low, mid, high }))
        }
        "set_crossfader" => Some((
            Origin::Mixer,
            Kind::SetCrossfader,
            CmdBody::SetCrossfader {
                position: norm.clamp(0.0, 1.0),
            },
        )),
        "set_cue_mix" => Some((
            Origin::Mixer,
            Kind::SetCueMix,
            CmdBody::SetCueMix {
                mix: norm.clamp(0.0, 1.0),
            },
        )),
        "set_master_cue" => {
            if !active {
                return None;
            }
            Some((
                Origin::Mixer,
                Kind::SetMasterCue,
                CmdBody::SetMasterCue {
                    enabled: !snap.master_cue,
                },
            ))
        }
        "set_headphone_cue" => {
            if !active {
                return None;
            }
            let Origin::Deck(d) = origin else {
                return None;
            };
            let enabled = !snap.headphone_cue[deck_idx(d)];
            Some((
                origin,
                Kind::SetHeadphoneCue,
                CmdBody::SetHeadphoneCue { enabled },
            ))
        }
        "jog_touch" => Some((
            origin,
            Kind::JogTouch,
            CmdBody::JogTouch { touching: active },
        )),
        "jog_turn" => {
            // CC relative-ish: map 0..1 around center to delta ticks
            let delta = ((norm - 0.5) * 128.0).round() as i32;
            if delta == 0 {
                return None;
            }
            Some((origin, Kind::JogTurn, CmdBody::JogTurn { delta }))
        }
        a if a.starts_with("trigger_hot_cue_") => {
            if !active {
                return None;
            }
            let slot: u8 = a.strip_prefix("trigger_hot_cue_")?.parse().ok()?;
            if !(1..=8).contains(&slot) {
                return None;
            }
            let Origin::Deck(d) = origin else {
                return None;
            };
            let pos = snap.hot_cues[deck_idx(d)][(slot - 1) as usize].unwrap_or(0);
            Some((
                origin,
                Kind::TriggerHotCue,
                CmdBody::TriggerHotCue { position_ms: pos },
            ))
        }
        "loop_in" => active.then_some((origin, Kind::LoopIn, CmdBody::Empty)),
        "loop_out" => active.then_some((origin, Kind::LoopOut, CmdBody::Empty)),
        "exit_loop" => active.then_some((origin, Kind::ExitLoop, CmdBody::Empty)),
        "auto_loop_4" => {
            active.then_some((origin, Kind::SetAutoLoop, CmdBody::SetAutoLoop { beats: 4 }))
        }
        "beat_jump_fwd_4" => {
            active.then_some((origin, Kind::BeatJump, CmdBody::BeatJump { beats: 4 }))
        }
        "beat_jump_back_4" => {
            active.then_some((origin, Kind::BeatJump, CmdBody::BeatJump { beats: -4 }))
        }
        "pad_mode_hot_cue" => active.then_some((
            origin,
            Kind::SetPadMode,
            CmdBody::SetPadMode {
                mode: PadMode::HotCue,
            },
        )),
        "pad_mode_loop_roll" => active.then_some((
            origin,
            Kind::SetPadMode,
            CmdBody::SetPadMode {
                mode: PadMode::LoopRoll,
            },
        )),
        "pad_mode_beat_jump" => active.then_some((
            origin,
            Kind::SetPadMode,
            CmdBody::SetPadMode {
                mode: PadMode::BeatJump,
            },
        )),
        "pad_mode_sampler" => active.then_some((
            origin,
            Kind::SetPadMode,
            CmdBody::SetPadMode {
                mode: PadMode::Sampler,
            },
        )),
        a if a.starts_with("trigger_sampler_") => {
            if !active {
                return None;
            }
            let slot: u8 = a.strip_prefix("trigger_sampler_")?.parse().ok()?;
            Some((
                origin,
                Kind::TriggerSampler,
                CmdBody::TriggerSampler { slot: slot - 1 },
            ))
        }
        _ => None,
    }
}

#[allow(dead_code)]
pub fn _jog_mode_placeholder() -> JogMode {
    JogMode::Vinyl
}
