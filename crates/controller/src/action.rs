//! Action string → [`RoutedAction`].

use engine_api::{CmdBody, JogMode, Kind, Origin, PadMode};
use library_api::{EvtBody as LibraryEvtBody, Kind as LibraryKind, Origin as LibraryOrigin};

use crate::action_id::{bind_origin, parse_action_id, BoundOrigin};

/// Engine control values mirrored for soft-takeover / action context.
#[derive(Clone, Debug)]
pub struct ControlSnapshot {
    pub playing: [bool; 4],
    pub volume: [f32; 4],
    pub filter_db: [f32; 4],
    pub gain_db: [f32; 4],
    pub speed: [f32; 4],
    pub eq_low: [f32; 4],
    pub eq_mid: [f32; 4],
    pub eq_high: [f32; 4],
    pub headphone_cue: [bool; 4],
    pub quantize: [bool; 4],
    pub pad_mode: [PadMode; 4],
    pub crossfader: f32,
    pub cue_mix: f32,
    pub master_cue: bool,
    /// Hot cue positions ms per deck/slot (None = empty).
    pub hot_cues: [[Option<i32>; 8]; 4],
}

impl Default for ControlSnapshot {
    fn default() -> Self {
        Self {
            playing: [false; 4],
            volume: [1.0; 4],
            filter_db: [0.0; 4],
            gain_db: [0.0; 4],
            speed: [1.0; 4],
            eq_low: [0.0; 4],
            eq_mid: [0.0; 4],
            eq_high: [0.0; 4],
            headphone_cue: [false; 4],
            quantize: [false; 4],
            pad_mode: [PadMode::HotCue; 4],
            crossfader: 0.5,
            cue_mix: 0.5,
            master_cue: false,
            hot_cues: [[None; 8]; 4],
        }
    }
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
                    "speed" | "tempo" => self.speed[i] = value,
                    "eq_low" => self.eq_low[i] = value,
                    "eq_mid" => self.eq_mid[i] = value,
                    "eq_high" => self.eq_high[i] = value,
                    "playing" => self.playing[i] = value > 0.5,
                    "headphone_cue" => self.headphone_cue[i] = value > 0.5,
                    "quantize" => self.quantize[i] = value > 0.5,
                    "pad_mode" => {
                        self.pad_mode[i] = match value as u8 {
                            1 => PadMode::LoopRoll,
                            2 => PadMode::BeatJump,
                            3 => PadMode::Sampler,
                            _ => PadMode::HotCue,
                        };
                    }
                    _ => {}
                }
            }
            Origin::Engine => {}
        }
    }

    pub fn get_norm_for_action(&self, origin: Origin, leaf: &str) -> Option<f32> {
        match origin {
            Origin::Mixer => match leaf {
                "set_crossfader" => Some(self.crossfader),
                "set_cue_mix" => Some(self.cue_mix),
                _ => None,
            },
            Origin::Deck(d) => {
                let i = deck_idx(d);
                match leaf {
                    "set_volume" => Some(self.volume[i]),
                    "set_filter" => Some(filter_db_to_norm(self.filter_db[i])),
                    "set_gain" => Some(gain_db_to_norm(self.gain_db[i])),
                    "set_speed" => Some(speed_to_norm(self.speed[i])),
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

/// Match `engine-dsp` / GUI ±24 dB strip range.
const STRIP_DB_MIN: f32 = -24.0;
const STRIP_DB_MAX: f32 = 24.0;

fn db_to_norm(db: f32) -> f32 {
    ((db - STRIP_DB_MIN) / (STRIP_DB_MAX - STRIP_DB_MIN)).clamp(0.0, 1.0)
}

fn norm_to_db(n: f32) -> f32 {
    // Match GUI EQ_STEP_DB (0.1) so MIDI center detents land on 0.
    let db = n.clamp(0.0, 1.0) * (STRIP_DB_MAX - STRIP_DB_MIN) + STRIP_DB_MIN;
    (db * 10.0).round() / 10.0
}

fn filter_db_to_norm(db: f32) -> f32 {
    db_to_norm(db)
}

fn norm_to_filter_db(n: f32) -> f32 {
    norm_to_db(n)
}

fn gain_db_to_norm(db: f32) -> f32 {
    db_to_norm(db)
}

fn norm_to_gain_db(n: f32) -> f32 {
    norm_to_db(n)
}

fn eq_to_norm(db: f32) -> f32 {
    db_to_norm(db)
}

fn norm_to_eq(n: f32) -> f32 {
    norm_to_db(n)
}

/// Pioneer tempo fader: top = slow. Map engine speed (~0.84..1.16) ↔ MIDI 0..1 inverted.
fn speed_to_norm(speed: f32) -> f32 {
    let s = speed.clamp(0.84, 1.16);
    (1.0 - (s - 0.84) / 0.32).clamp(0.0, 1.0)
}

fn norm_to_speed(n: f32) -> f32 {
    // inverted: MIDI 0 (top) → 1.16, MIDI 1 (bottom) → 0.84
    1.16 - n.clamp(0.0, 1.0) * 0.32
}

/// Resolved mapping publish target.
#[derive(Clone, Debug, PartialEq)]
pub enum RoutedAction {
    EngineCmd {
        origin: Origin,
        kind: Kind,
        body: CmdBody,
    },
    LibraryEvt {
        origin: LibraryOrigin,
        kind: LibraryKind,
        body: LibraryEvtBody,
    },
}

fn engine_cmd(origin: Origin, kind: Kind, body: CmdBody) -> RoutedAction {
    RoutedAction::EngineCmd { origin, kind, body }
}

/// Resolve qualified action to a routable cmd/evt. `norm` is MIDI 0..1; `active` for buttons.
pub fn resolve_action(
    action: &str,
    section: &str,
    norm: f32,
    active: bool,
    snap: &ControlSnapshot,
) -> Option<RoutedAction> {
    let (template, leaf) = parse_action_id(action).ok()?;
    let bound = bind_origin(template, section).ok()?;

    if let BoundOrigin::LibraryNavigation = bound {
        if !active {
            return None;
        }
        let kind = match leaf {
            "navigate_next" => LibraryKind::NavigateNext,
            "navigate_prev" => LibraryKind::NavigatePrev,
            _ => return None,
        };
        return Some(RoutedAction::LibraryEvt {
            origin: LibraryOrigin::LibraryNavigation,
            kind,
            body: LibraryEvtBody::Empty,
        });
    }

    let BoundOrigin::Engine(origin) = bound else {
        return None;
    };

    // Buttons: only fire on press (active edge handled by caller).
    match leaf {
        "toggle_play" => {
            if !active {
                return None;
            }
            let playing = match origin {
                Origin::Deck(d) => snap.playing[deck_idx(d)],
                _ => false,
            };
            if playing {
                Some(engine_cmd(origin, Kind::Pause, CmdBody::Empty))
            } else {
                Some(engine_cmd(origin, Kind::Play, CmdBody::Empty))
            }
        }
        "play" => active.then_some(engine_cmd(origin, Kind::Play, CmdBody::Empty)),
        "pause" => active.then_some(engine_cmd(origin, Kind::Pause, CmdBody::Empty)),
        "cue" => active.then_some(engine_cmd(origin, Kind::SetCuePoint, CmdBody::Empty)),
        "cue_default" => {
            if active {
                Some(engine_cmd(origin, Kind::BeginCueHold, CmdBody::Empty))
            } else {
                Some(engine_cmd(origin, Kind::EndCueHold, CmdBody::Empty))
            }
        }
        "begin_cue_hold" => {
            active.then_some(engine_cmd(origin, Kind::BeginCueHold, CmdBody::Empty))
        }
        "end_cue_hold" => (!active).then_some(engine_cmd(origin, Kind::EndCueHold, CmdBody::Empty)),
        "toggle_sync" => active.then_some(engine_cmd(
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
            Some(engine_cmd(
                origin,
                Kind::SetQuantize,
                CmdBody::SetQuantize { enabled },
            ))
        }
        "set_volume" => Some(engine_cmd(
            origin,
            Kind::SetVolume,
            CmdBody::SetVolume {
                volume: norm.clamp(0.0, 1.0),
            },
        )),
        "set_filter" => Some(engine_cmd(
            origin,
            Kind::SetFilter,
            CmdBody::SetFilter {
                filter_db: norm_to_filter_db(norm),
            },
        )),
        "set_gain" => Some(engine_cmd(
            origin,
            Kind::SetGainTrim,
            CmdBody::SetGainTrim {
                gain_db: norm_to_gain_db(norm),
            },
        )),
        "set_speed" => Some(engine_cmd(
            origin,
            Kind::SetSpeed,
            CmdBody::SetSpeed {
                speed: norm_to_speed(norm),
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
            match leaf {
                "set_eq_low" => low = v,
                "set_eq_mid" => mid = v,
                "set_eq_high" => high = v,
                _ => {}
            }
            Some(engine_cmd(
                origin,
                Kind::SetEq,
                CmdBody::SetEq { low, mid, high },
            ))
        }
        "set_crossfader" => Some(engine_cmd(
            Origin::Mixer,
            Kind::SetCrossfader,
            CmdBody::SetCrossfader {
                position: norm.clamp(0.0, 1.0),
            },
        )),
        "set_cue_mix" => Some(engine_cmd(
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
            Some(engine_cmd(
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
            Some(engine_cmd(
                origin,
                Kind::SetHeadphoneCue,
                CmdBody::SetHeadphoneCue { enabled },
            ))
        }
        "jog_touch" => Some(engine_cmd(
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
            Some(engine_cmd(
                origin,
                Kind::JogTurn,
                CmdBody::JogTurn { delta },
            ))
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
            let idx = (slot - 1) as usize;
            match snap.hot_cues[deck_idx(d)][idx] {
                Some(pos) => Some(engine_cmd(
                    origin,
                    Kind::TriggerHotCue,
                    CmdBody::TriggerHotCue { position_ms: pos },
                )),
                None => Some(engine_cmd(
                    origin,
                    Kind::SaveHotCue,
                    CmdBody::SaveHotCue { slot: slot - 1 },
                )),
            }
        }
        a if a.starts_with("delete_hot_cue_") => {
            if !active {
                return None;
            }
            let slot: u8 = a.strip_prefix("delete_hot_cue_")?.parse().ok()?;
            if !(1..=8).contains(&slot) {
                return None;
            }
            Some(engine_cmd(
                origin,
                Kind::DeleteHotCue,
                CmdBody::DeleteHotCue { slot: slot - 1 },
            ))
        }
        "loop_in" => active.then_some(engine_cmd(origin, Kind::LoopIn, CmdBody::Empty)),
        "loop_out" => active.then_some(engine_cmd(origin, Kind::LoopOut, CmdBody::Empty)),
        "exit_loop" => active.then_some(engine_cmd(origin, Kind::ExitLoop, CmdBody::Empty)),
        a if a.starts_with("auto_loop_") => {
            if !active {
                return None;
            }
            let beats: u32 = a.strip_prefix("auto_loop_")?.parse().ok()?;
            Some(engine_cmd(
                origin,
                Kind::SetAutoLoop,
                CmdBody::SetAutoLoop { beats },
            ))
        }
        a if a.starts_with("beat_jump_fwd_") => {
            if !active {
                return None;
            }
            let beats: i32 = a.strip_prefix("beat_jump_fwd_")?.parse().ok()?;
            Some(engine_cmd(
                origin,
                Kind::BeatJump,
                CmdBody::BeatJump { beats },
            ))
        }
        a if a.starts_with("beat_jump_back_") => {
            if !active {
                return None;
            }
            let beats: i32 = a.strip_prefix("beat_jump_back_")?.parse().ok()?;
            Some(engine_cmd(
                origin,
                Kind::BeatJump,
                CmdBody::BeatJump { beats: -beats },
            ))
        }
        "pad_mode_hot_cue" => active.then_some(engine_cmd(
            origin,
            Kind::SetPadMode,
            CmdBody::SetPadMode {
                mode: PadMode::HotCue,
            },
        )),
        "pad_mode_loop_roll" => active.then_some(engine_cmd(
            origin,
            Kind::SetPadMode,
            CmdBody::SetPadMode {
                mode: PadMode::LoopRoll,
            },
        )),
        "pad_mode_beat_jump" => active.then_some(engine_cmd(
            origin,
            Kind::SetPadMode,
            CmdBody::SetPadMode {
                mode: PadMode::BeatJump,
            },
        )),
        "pad_mode_sampler" => active.then_some(engine_cmd(
            origin,
            Kind::SetPadMode,
            CmdBody::SetPadMode {
                mode: PadMode::Sampler,
            },
        )),
        a if a.starts_with("pad_") => {
            let slot: u8 = a.strip_prefix("pad_")?.parse().ok()?;
            if !(1..=8).contains(&slot) {
                return None;
            }
            let Origin::Deck(d) = origin else {
                return None;
            };
            let mode = snap.pad_mode[deck_idx(d)];
            resolve_pad_slot(origin, slot, active, mode, snap)
        }
        a if a.starts_with("trigger_sampler_") => {
            if !active {
                return None;
            }
            let slot: u8 = a.strip_prefix("trigger_sampler_")?.parse().ok()?;
            Some(engine_cmd(
                origin,
                Kind::TriggerSampler,
                CmdBody::TriggerSampler { slot: slot - 1 },
            ))
        }
        _ => None,
    }
}

/// Match GUI pad grid (`LOOP_ROLL_BEATS` / beat-jump layout).
const LOOP_ROLL_BEATS: [u32; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
const BEAT_JUMP_BEATS: [i32; 8] = [1, 2, 4, 8, -1, -2, -4, -8];

fn resolve_pad_slot(
    origin: Origin,
    slot: u8,
    active: bool,
    mode: PadMode,
    snap: &ControlSnapshot,
) -> Option<RoutedAction> {
    let idx = (slot - 1) as usize;
    match mode {
        PadMode::HotCue => {
            if !active {
                return None;
            }
            let Origin::Deck(d) = origin else {
                return None;
            };
            match snap.hot_cues[deck_idx(d)][idx] {
                Some(pos) => Some(engine_cmd(
                    origin,
                    Kind::TriggerHotCue,
                    CmdBody::TriggerHotCue { position_ms: pos },
                )),
                None => Some(engine_cmd(
                    origin,
                    Kind::SaveHotCue,
                    CmdBody::SaveHotCue { slot: slot - 1 },
                )),
            }
        }
        PadMode::LoopRoll => {
            let beats = LOOP_ROLL_BEATS[idx];
            if active {
                Some(engine_cmd(
                    origin,
                    Kind::BeginLoopRoll,
                    CmdBody::BeginLoopRoll { beats },
                ))
            } else {
                Some(engine_cmd(origin, Kind::EndLoopRoll, CmdBody::Empty))
            }
        }
        PadMode::BeatJump => {
            if !active {
                return None;
            }
            let beats = BEAT_JUMP_BEATS[idx];
            Some(engine_cmd(
                origin,
                Kind::BeatJump,
                CmdBody::BeatJump { beats },
            ))
        }
        PadMode::Sampler => {
            if !active {
                return None;
            }
            Some(engine_cmd(
                origin,
                Kind::TriggerSampler,
                CmdBody::TriggerSampler { slot: slot - 1 },
            ))
        }
    }
}

#[allow(dead_code)]
pub fn _jog_mode_placeholder() -> JogMode {
    JogMode::Vinyl
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_knob_max_maps_to_plus_24_db() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action("Deck(_)::set_eq_low", "deck_1", 1.0, true, &snap).unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SetEq { low, mid, high },
                ..
            } => {
                assert!((low - 24.0).abs() < 1e-5, "low={low}");
                assert_eq!(mid, 0.0);
                assert_eq!(high, 0.0);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn eq_knob_center_maps_to_0_db() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action("Deck(_)::set_eq_mid", "deck_1", 0.5, true, &snap).unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SetEq { mid, .. },
                ..
            } => assert!((mid - 0.0).abs() < 1e-5, "mid={mid}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn filter_knob_max_maps_to_plus_24_db() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action("Deck(_)::set_filter", "deck_1", 1.0, true, &snap).unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SetFilter { filter_db },
                ..
            } => assert!((filter_db - 24.0).abs() < 1e-5, "filter_db={filter_db}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn gain_knob_max_maps_to_plus_24_db() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action("Deck(_)::set_gain", "deck_1", 1.0, true, &snap).unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SetGainTrim { gain_db },
                ..
            } => assert!((gain_db - 24.0).abs() < 1e-5, "gain_db={gain_db}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn pad_routes_by_software_pad_mode() {
        let mut snap = ControlSnapshot::default();
        let routed = resolve_action("Deck(_)::pad_1", "deck_1", 1.0, true, &snap).unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SaveHotCue { slot },
                ..
            } => assert_eq!(slot, 0),
            other => panic!("expected SaveHotCue, got {other:?}"),
        }

        snap.hot_cues[0][0] = Some(12_500);
        let routed = resolve_action("Deck(_)::pad_1", "deck_1", 1.0, true, &snap).unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::TriggerHotCue { position_ms },
                ..
            } => assert_eq!(position_ms, 12_500),
            other => panic!("expected TriggerHotCue, got {other:?}"),
        }

        snap.pad_mode[0] = PadMode::BeatJump;
        let routed = resolve_action("Deck(_)::pad_1", "deck_1", 1.0, true, &snap).unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::BeatJump { beats },
                ..
            } => assert_eq!(beats, 1),
            other => panic!("expected BeatJump +1, got {other:?}"),
        }

        snap.pad_mode[0] = PadMode::LoopRoll;
        let begin = resolve_action("Deck(_)::pad_3", "deck_1", 1.0, true, &snap).unwrap();
        match begin {
            RoutedAction::EngineCmd {
                body: CmdBody::BeginLoopRoll { beats },
                ..
            } => assert_eq!(beats, 4),
            other => panic!("expected BeginLoopRoll 4, got {other:?}"),
        }
        let end = resolve_action("Deck(_)::pad_3", "deck_1", 0.0, false, &snap).unwrap();
        assert!(matches!(
            end,
            RoutedAction::EngineCmd {
                kind: Kind::EndLoopRoll,
                ..
            }
        ));

        snap.pad_mode[0] = PadMode::Sampler;
        let routed = resolve_action("Deck(_)::pad_2", "deck_1", 1.0, true, &snap).unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::TriggerSampler { slot },
                ..
            } => assert_eq!(slot, 1),
            other => panic!("expected TriggerSampler, got {other:?}"),
        }
    }

    #[test]
    fn pad_mode_button_sets_mode() {
        let snap = ControlSnapshot::default();
        let routed =
            resolve_action("Deck(_)::pad_mode_loop_roll", "deck_1", 1.0, true, &snap).unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SetPadMode { mode },
                ..
            } => assert_eq!(mode, PadMode::LoopRoll),
            other => panic!("unexpected {other:?}"),
        }
    }
}
