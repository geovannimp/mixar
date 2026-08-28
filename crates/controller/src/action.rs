//! Action string → [`RoutedAction`].

use engine_api::{CmdBody, JogMode, Kind, Origin, PadMode};
use library_api::{EvtBody as LibraryEvtBody, Kind as LibraryKind, Origin as LibraryOrigin};

use crate::action_id::{bind_origin, parse_action_id, BoundOrigin};

/// Library hot-cue slots are 0..=15; MIDI `pad n` and LED cache match that.
pub const HOT_CUE_SLOT_COUNT: usize = 16;

/// Local mirrors for LED + soft-takeover (not MIDI pad routing).
/// Absolute values are wire `0..1` when present.
#[derive(Clone, Debug)]
pub struct ControlSnapshot {
    pub playing: [bool; 4],
    pub volume: [f32; 4],
    pub filter: [f32; 4],
    pub gain_trim: [f32; 4],
    pub speed: [f32; 4],
    /// Pitch fraction half-span per deck (`0.06` = ±6%).
    pub tempo_range: [f32; 4],
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
    pub hot_cues: [[Option<i32>; HOT_CUE_SLOT_COUNT]; 4],
}

impl Default for ControlSnapshot {
    fn default() -> Self {
        Self {
            playing: [false; 4],
            volume: [1.0; 4],
            filter: [0.5; 4],
            gain_trim: [0.5; 4],
            speed: [0.5; 4],
            tempo_range: [DEFAULT_TEMPO_RANGE; 4],
            eq_low: [0.5; 4],
            eq_mid: [0.5; 4],
            eq_high: [0.5; 4],
            headphone_cue: [false; 4],
            quantize: [false; 4],
            pad_mode: [PadMode::HotCue; 4],
            crossfader: 0.5,
            cue_mix: 0.5,
            master_cue: false,
            hot_cues: [[None; HOT_CUE_SLOT_COUNT]; 4],
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
                    "filter" | "filter_db" => self.filter[i] = value,
                    "gain" | "gain_db" | "gain_trim" => self.gain_trim[i] = value,
                    "speed" | "tempo" => self.speed[i] = value,
                    "tempo_range" => self.tempo_range[i] = value,
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
}

fn deck_idx(d: u16) -> usize {
    (d as usize).min(3)
}

/// Matches `engine_core::config::DEFAULT_TEMPO_RANGE` / `DEFAULT_TEMPO_RANGE_STEPS`.
const DEFAULT_TEMPO_RANGE: f32 = 0.06;
const TEMPO_RANGE_STEPS: &[f32] = &[0.06, 0.10, 0.16, 0.25];

fn next_tempo_range(current: f32) -> f32 {
    const EPS: f32 = 1e-4;
    if let Some(i) = TEMPO_RANGE_STEPS
        .iter()
        .position(|s| (*s - current).abs() < EPS)
    {
        return TEMPO_RANGE_STEPS[(i + 1) % TEMPO_RANGE_STEPS.len()];
    }
    TEMPO_RANGE_STEPS[0]
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

/// Wire value after device decode: absolute `0..1` or relative tick delta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlValue {
    Absolute(f32),
    Relative(i32),
}

impl ControlValue {
    fn as_absolute(self) -> Option<f32> {
        match self {
            Self::Absolute(v) => Some(v),
            Self::Relative(_) => None,
        }
    }

    fn as_relative(self) -> Option<i32> {
        match self {
            Self::Relative(d) => Some(d),
            Self::Absolute(_) => None,
        }
    }
}

/// Resolve qualified action to a routable cmd/evt.
pub fn resolve_action(
    action: &str,
    section: &str,
    value: ControlValue,
    active: bool,
    soft_takeover: bool,
    snap: &ControlSnapshot,
) -> Option<RoutedAction> {
    let (template, leaf, args) = parse_action_id(action).ok()?;
    let bound = bind_origin(template, section).ok()?;

    if let BoundOrigin::LibraryNavigation = bound {
        match leaf {
            "navigate_next" => {
                if !active {
                    return None;
                }
                return Some(RoutedAction::LibraryEvt {
                    origin: LibraryOrigin::LibraryNavigation,
                    kind: LibraryKind::Navigate,
                    body: LibraryEvtBody::Navigate { delta: 1 },
                });
            }
            "navigate_prev" => {
                if !active {
                    return None;
                }
                return Some(RoutedAction::LibraryEvt {
                    origin: LibraryOrigin::LibraryNavigation,
                    kind: LibraryKind::Navigate,
                    body: LibraryEvtBody::Navigate { delta: -1 },
                });
            }
            // Relative select-knob: delta already decoded from device `relative` mode.
            "navigate" => {
                let delta = value.as_relative()?;
                if delta == 0 {
                    return None;
                }
                return Some(RoutedAction::LibraryEvt {
                    origin: LibraryOrigin::LibraryNavigation,
                    kind: LibraryKind::Navigate,
                    body: LibraryEvtBody::Navigate { delta },
                });
            }
            "load_to_deck" => {
                if !active {
                    return None;
                }
                let deck_1based = args.require_int("deck").ok()?;
                if deck_1based < 1 {
                    return None;
                }
                let deck = (deck_1based - 1) as u16;
                return Some(RoutedAction::LibraryEvt {
                    origin: LibraryOrigin::LibraryNavigation,
                    kind: LibraryKind::Load,
                    body: LibraryEvtBody::Load { deck },
                });
            }
            _ => return None,
        }
    }

    let BoundOrigin::Engine(origin) = bound else {
        return None;
    };

    // Relative ticks are only for jog_turn (navigate handled above). Never coerce to 0.0.
    if matches!(value, ControlValue::Relative(_)) && leaf != "jog_turn" {
        return None;
    }
    let norm = value.as_absolute().unwrap_or(0.0);

    // Buttons: only fire on press (active edge handled by caller).
    match leaf {
        "toggle_play" => active.then_some(engine_cmd(origin, Kind::TogglePlay, CmdBody::Empty)),
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
            active.then_some(engine_cmd(origin, Kind::ToggleQuantize, CmdBody::Empty))
        }
        "set_volume" => Some(engine_cmd(
            origin,
            Kind::SetVolume,
            CmdBody::SetVolume {
                volume: norm.clamp(0.0, 1.0),
                soft_takeover,
            },
        )),
        "set_filter" => Some(engine_cmd(
            origin,
            Kind::SetFilter,
            CmdBody::SetFilter {
                filter: norm.clamp(0.0, 1.0),
                soft_takeover,
            },
        )),
        "set_gain" => Some(engine_cmd(
            origin,
            Kind::SetGainTrim,
            CmdBody::SetGainTrim {
                gain_trim: norm.clamp(0.0, 1.0),
                soft_takeover,
            },
        )),
        "set_speed" => Some(engine_cmd(
            origin,
            Kind::SetSpeed,
            CmdBody::SetSpeed {
                speed: norm.clamp(0.0, 1.0),
                soft_takeover,
            },
        )),
        "cycle_tempo_range" => {
            if !active {
                return None;
            }
            let Origin::Deck(d) = origin else {
                return None;
            };
            let i = deck_idx(d);
            let next = next_tempo_range(snap.tempo_range[i]);
            Some(engine_cmd(
                origin,
                Kind::SetTempoRange,
                CmdBody::SetTempoRange { tempo_range: next },
            ))
        }
        "set_eq_low" | "set_eq_mid" | "set_eq_high" => {
            let band = match leaf {
                "set_eq_low" => engine_api::EqBand::Low,
                "set_eq_mid" => engine_api::EqBand::Mid,
                "set_eq_high" => engine_api::EqBand::High,
                _ => return None,
            };
            Some(engine_cmd(
                origin,
                Kind::SetEqBand,
                CmdBody::SetEqBand {
                    band,
                    gain: norm.clamp(0.0, 1.0),
                    soft_takeover,
                },
            ))
        }
        "set_crossfader" => Some(engine_cmd(
            Origin::Mixer,
            Kind::SetCrossfader,
            CmdBody::SetCrossfader {
                position: norm.clamp(0.0, 1.0),
                soft_takeover,
            },
        )),
        "set_cue_mix" => Some(engine_cmd(
            Origin::Mixer,
            Kind::SetCueMix,
            CmdBody::SetCueMix {
                mix: norm.clamp(0.0, 1.0),
                soft_takeover,
            },
        )),
        "set_master_cue" => active.then_some(engine_cmd(
            Origin::Mixer,
            Kind::ToggleMasterCue,
            CmdBody::Empty,
        )),
        "set_headphone_cue" => {
            active.then_some(engine_cmd(origin, Kind::ToggleHeadphoneCue, CmdBody::Empty))
        }
        "jog_touch" => Some(engine_cmd(
            origin,
            Kind::JogTouch,
            CmdBody::JogTouch { touching: active },
        )),
        "jog_turn" => {
            let delta = value.as_relative()?;
            if delta == 0 {
                return None;
            }
            Some(engine_cmd(
                origin,
                Kind::JogTurn,
                CmdBody::JogTurn { delta },
            ))
        }
        "trigger_hot_cue" => {
            if !active {
                return None;
            }
            let slot = args.require_int("slot").ok()?;
            if slot < 1 {
                return None;
            }
            let slot_u = slot as u8;
            let Origin::Deck(d) = origin else {
                return None;
            };
            let idx = (slot_u - 1) as usize;
            let cues = &snap.hot_cues[deck_idx(d)];
            if idx >= cues.len() {
                return Some(engine_cmd(
                    origin,
                    Kind::SaveHotCue,
                    CmdBody::SaveHotCue { slot: slot_u - 1 },
                ));
            }
            match cues[idx] {
                Some(pos) => Some(engine_cmd(
                    origin,
                    Kind::TriggerHotCue,
                    CmdBody::TriggerHotCue { position_ms: pos },
                )),
                None => Some(engine_cmd(
                    origin,
                    Kind::SaveHotCue,
                    CmdBody::SaveHotCue { slot: slot_u - 1 },
                )),
            }
        }
        "delete_hot_cue" => {
            if !active {
                return None;
            }
            let slot = args.require_int("slot").ok()?;
            if slot < 1 {
                return None;
            }
            Some(engine_cmd(
                origin,
                Kind::DeleteHotCue,
                CmdBody::DeleteHotCue {
                    slot: (slot as u8) - 1,
                },
            ))
        }
        "loop_in" => active.then_some(engine_cmd(origin, Kind::LoopIn, CmdBody::Empty)),
        "loop_out" => active.then_some(engine_cmd(origin, Kind::LoopOut, CmdBody::Empty)),
        "exit_loop" => active.then_some(engine_cmd(origin, Kind::ExitLoop, CmdBody::Empty)),
        "auto_loop" => {
            if !active {
                return None;
            }
            let beats = args.require_f32("beats").ok()?;
            if !beats.is_finite() || beats <= 0.0 {
                return None;
            }
            Some(engine_cmd(
                origin,
                Kind::SetAutoLoop,
                CmdBody::SetAutoLoop { beats },
            ))
        }
        "beat_jump" => {
            if !active {
                return None;
            }
            let beats = args.require_f32("beats").ok()?;
            if !beats.is_finite() || beats == 0.0 {
                return None;
            }
            Some(engine_cmd(
                origin,
                Kind::BeatJump,
                CmdBody::BeatJump { beats },
            ))
        }
        "pad_mode" => {
            if !active {
                return None;
            }
            let mode = args.require_ident("mode").ok()?;
            let mode = match mode {
                "hot_cue" => PadMode::HotCue,
                "loop_roll" => PadMode::LoopRoll,
                "beat_jump" => PadMode::BeatJump,
                "sampler" => PadMode::Sampler,
                _ => return None,
            };
            Some(engine_cmd(
                origin,
                Kind::SetPadMode,
                CmdBody::SetPadMode { mode },
            ))
        }
        "pad" => {
            let slot = pad_n_slot(&args)?;
            if active {
                Some(engine_cmd(
                    origin,
                    Kind::PadPress,
                    CmdBody::PadPress { slot, shift: false },
                ))
            } else {
                Some(engine_cmd(
                    origin,
                    Kind::PadRelease,
                    CmdBody::PadRelease { slot },
                ))
            }
        }
        "hot_cue_pad" => resolve_pad_slot(origin, pad_n_slot(&args)?, active, PadMode::HotCue),
        "loop_roll_pad" => resolve_pad_slot(origin, pad_n_slot(&args)?, active, PadMode::LoopRoll),
        "beat_jump_pad" => resolve_pad_slot(origin, pad_n_slot(&args)?, active, PadMode::BeatJump),
        "sampler_pad" => resolve_pad_slot(origin, pad_n_slot(&args)?, active, PadMode::Sampler),
        "trigger_sampler" => {
            let slot = args.require_int("slot").ok()?;
            if slot < 1 {
                return None;
            }
            let slot = u8::try_from(slot - 1).ok()?;
            if active {
                Some(engine_cmd(
                    origin,
                    Kind::SamplerPadPress,
                    CmdBody::SamplerPadPress { slot, shift: false },
                ))
            } else {
                Some(engine_cmd(
                    origin,
                    Kind::SamplerPadRelease,
                    CmdBody::SamplerPadRelease { slot },
                ))
            }
        }
        _ => None,
    }
}

fn pad_n_slot(args: &crate::action_id::ActionArgs) -> Option<u8> {
    let n = args.require_int("n").ok()?;
    u8::try_from(n.checked_sub(1)?).ok()
}

/// Named pad leaves publish the mode-specific press/release pair (Pioneer note banks).
/// Generic `pad` publishes PadPress / PadRelease instead.
fn resolve_pad_slot(origin: Origin, slot: u8, active: bool, mode: PadMode) -> Option<RoutedAction> {
    match mode {
        PadMode::HotCue => {
            if active {
                Some(engine_cmd(
                    origin,
                    Kind::HotCuePadPress,
                    CmdBody::HotCuePadPress { slot, shift: false },
                ))
            } else {
                Some(engine_cmd(
                    origin,
                    Kind::HotCuePadRelease,
                    CmdBody::HotCuePadRelease { slot },
                ))
            }
        }
        PadMode::LoopRoll => {
            if active {
                Some(engine_cmd(
                    origin,
                    Kind::LoopRollPadPress,
                    CmdBody::LoopRollPadPress { slot },
                ))
            } else {
                Some(engine_cmd(
                    origin,
                    Kind::LoopRollPadRelease,
                    CmdBody::LoopRollPadRelease { slot },
                ))
            }
        }
        PadMode::BeatJump => {
            if active {
                Some(engine_cmd(
                    origin,
                    Kind::BeatJumpPadPress,
                    CmdBody::BeatJumpPadPress { slot },
                ))
            } else {
                Some(engine_cmd(
                    origin,
                    Kind::BeatJumpPadRelease,
                    CmdBody::BeatJumpPadRelease { slot },
                ))
            }
        }
        PadMode::Sampler => {
            if active {
                Some(engine_cmd(
                    origin,
                    Kind::SamplerPadPress,
                    CmdBody::SamplerPadPress { slot, shift: false },
                ))
            } else {
                Some(engine_cmd(
                    origin,
                    Kind::SamplerPadRelease,
                    CmdBody::SamplerPadRelease { slot },
                ))
            }
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
    fn eq_knob_max_maps_to_full_norm() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action(
            "Deck(_)::set_eq_low",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body:
                    CmdBody::SetEqBand {
                        band: engine_api::EqBand::Low,
                        gain,
                        soft_takeover: false,
                    },
                ..
            } => assert!((gain - 1.0).abs() < 1e-5, "gain={gain}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn eq_knob_center_maps_to_half_norm() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action(
            "Deck(_)::set_eq_mid",
            "deck_1",
            ControlValue::Absolute(0.5),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body:
                    CmdBody::SetEqBand {
                        band: engine_api::EqBand::Mid,
                        gain,
                        soft_takeover: false,
                    },
                ..
            } => assert!((gain - 0.5).abs() < 1e-5, "gain={gain}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn filter_knob_passes_norm() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action(
            "Deck(_)::set_filter",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body:
                    CmdBody::SetFilter {
                        filter,
                        soft_takeover: false,
                    },
                ..
            } => assert!((filter - 1.0).abs() < 1e-5, "filter={filter}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn gain_knob_passes_norm() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action(
            "Deck(_)::set_gain",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body:
                    CmdBody::SetGainTrim {
                        gain_trim,
                        soft_takeover: false,
                    },
                ..
            } => assert!((gain_trim - 1.0).abs() < 1e-5, "gain_trim={gain_trim}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn generic_pad_publishes_pad_press_release() {
        let mut snap = ControlSnapshot::default();
        let press = resolve_action(
            "Deck(_)::pad(n:1)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match press {
            RoutedAction::EngineCmd {
                body: CmdBody::PadPress { slot, shift },
                ..
            } => {
                assert_eq!(slot, 0);
                assert!(!shift);
            }
            other => panic!("expected PadPress, got {other:?}"),
        }
        let release = resolve_action(
            "Deck(_)::pad(n:1)",
            "deck_1",
            ControlValue::Absolute(0.0),
            false,
            false,
            &snap,
        )
        .unwrap();
        assert!(matches!(
            release,
            RoutedAction::EngineCmd {
                body: CmdBody::PadRelease { slot: 0 },
                ..
            }
        ));

        snap.pad_mode[0] = PadMode::BeatJump;
        snap.hot_cues[0][0] = Some(12_500);
        let still_generic = resolve_action(
            "Deck(_)::pad(n:1)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        assert!(
            matches!(
                still_generic,
                RoutedAction::EngineCmd {
                    body: CmdBody::PadPress {
                        slot: 0,
                        shift: false
                    },
                    ..
                }
            ),
            "generic pad must not read snapshot pad_mode or cues, got {still_generic:?}"
        );
    }

    #[test]
    fn named_pad_leaves_publish_mode_pair() {
        let snap = ControlSnapshot::default();
        let press = resolve_action(
            "Deck(_)::hot_cue_pad(n:1)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match press {
            RoutedAction::EngineCmd {
                body: CmdBody::HotCuePadPress { slot, shift: false },
                ..
            } => assert_eq!(slot, 0),
            other => panic!("expected HotCuePadPress, got {other:?}"),
        }
        let release = resolve_action(
            "Deck(_)::hot_cue_pad(n:1)",
            "deck_1",
            ControlValue::Absolute(0.0),
            false,
            false,
            &snap,
        )
        .unwrap();
        assert!(matches!(
            release,
            RoutedAction::EngineCmd {
                body: CmdBody::HotCuePadRelease { slot: 0 },
                ..
            }
        ));

        let routed = resolve_action(
            "Deck(_)::beat_jump_pad(n:1)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::BeatJumpPadPress { slot },
                ..
            } => assert_eq!(slot, 0),
            other => panic!("expected BeatJumpPadPress, got {other:?}"),
        }

        let begin = resolve_action(
            "Deck(_)::loop_roll_pad(n:3)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match begin {
            RoutedAction::EngineCmd {
                body: CmdBody::LoopRollPadPress { slot },
                ..
            } => assert_eq!(slot, 2),
            other => panic!("expected LoopRollPadPress slot 2, got {other:?}"),
        }
        let end = resolve_action(
            "Deck(_)::loop_roll_pad(n:3)",
            "deck_1",
            ControlValue::Absolute(0.0),
            false,
            false,
            &snap,
        )
        .unwrap();
        assert!(matches!(
            end,
            RoutedAction::EngineCmd {
                body: CmdBody::LoopRollPadRelease { slot: 2 },
                ..
            }
        ));

        let routed = resolve_action(
            "Deck(_)::sampler_pad(n:2)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SamplerPadPress { slot, shift: false },
                ..
            } => assert_eq!(slot, 1),
            other => panic!("expected SamplerPadPress, got {other:?}"),
        }
        let end = resolve_action(
            "Deck(_)::sampler_pad(n:2)",
            "deck_1",
            ControlValue::Absolute(0.0),
            false,
            false,
            &snap,
        )
        .unwrap();
        assert!(matches!(
            end,
            RoutedAction::EngineCmd {
                body: CmdBody::SamplerPadRelease { slot: 1 },
                ..
            }
        ));
    }

    #[test]
    fn midi_pad_n_is_not_capped_at_eight() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action(
            "Deck(_)::pad(n:9)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::PadPress { slot, shift: false },
                ..
            } => assert_eq!(slot, 8),
            other => panic!("expected PadPress slot 8, got {other:?}"),
        }
        assert!(resolve_action(
            "Deck(_)::pad(n:0)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap
        )
        .is_none());
        let named = resolve_action(
            "Deck(_)::hot_cue_pad(n:9)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match named {
            RoutedAction::EngineCmd {
                body: CmdBody::HotCuePadPress { slot, shift: false },
                ..
            } => assert_eq!(slot, 8),
            other => panic!("expected HotCuePadPress slot 8, got {other:?}"),
        }
    }

    #[test]
    fn trigger_sampler_leaf_publishes_pad_press_release() {
        let snap = ControlSnapshot::default();
        let press = resolve_action(
            "Deck(_)::trigger_sampler(slot:2)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match press {
            RoutedAction::EngineCmd {
                body: CmdBody::SamplerPadPress { slot, shift: false },
                ..
            } => assert_eq!(slot, 1),
            other => panic!("expected SamplerPadPress, got {other:?}"),
        }
        let release = resolve_action(
            "Deck(_)::trigger_sampler(slot:2)",
            "deck_1",
            ControlValue::Absolute(0.0),
            false,
            false,
            &snap,
        )
        .unwrap();
        assert!(matches!(
            release,
            RoutedAction::EngineCmd {
                body: CmdBody::SamplerPadRelease { slot: 1 },
                ..
            }
        ));
    }

    #[test]
    fn pad_mode_button_sets_mode() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action(
            "Deck(_)::pad_mode(mode:loop_roll)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SetPadMode { mode },
                ..
            } => assert_eq!(mode, PadMode::LoopRoll),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn auto_loop_accepts_decimal_beats() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action(
            "Deck(_)::auto_loop(beats:0.25)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SetAutoLoop { beats },
                ..
            } => assert!((beats - 0.25).abs() < 1e-6),
            other => panic!("expected SetAutoLoop 0.25, got {other:?}"),
        }
    }

    #[test]
    fn relative_browse_navigate_uses_decoded_delta() {
        let snap = ControlSnapshot::default();
        let next = resolve_action(
            "LibraryNavigation::navigate",
            "master",
            ControlValue::Relative(1),
            true,
            false,
            &snap,
        )
        .unwrap();
        match next {
            RoutedAction::LibraryEvt {
                kind: LibraryKind::Navigate,
                body: LibraryEvtBody::Navigate { delta },
                ..
            } => assert_eq!(delta, 1),
            other => panic!("expected Navigate +1, got {other:?}"),
        }
        let prev = resolve_action(
            "LibraryNavigation::navigate",
            "master",
            ControlValue::Relative(-1),
            true,
            false,
            &snap,
        )
        .unwrap();
        match prev {
            RoutedAction::LibraryEvt {
                kind: LibraryKind::Navigate,
                body: LibraryEvtBody::Navigate { delta },
                ..
            } => assert_eq!(delta, -1),
            other => panic!("expected Navigate -1, got {other:?}"),
        }
        assert!(
            resolve_action(
                "LibraryNavigation::navigate",
                "master",
                ControlValue::Absolute(1.0 / 127.0),
                true,
                false,
                &snap,
            )
            .is_none(),
            "absolute wire value must not decode inside navigate"
        );
    }

    #[test]
    fn jog_turn_requires_relative_delta() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action(
            "Deck(_)::jog_turn",
            "deck_1",
            ControlValue::Relative(3),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::JogTurn { delta },
                ..
            } => assert_eq!(delta, 3),
            other => panic!("expected JogTurn, got {other:?}"),
        }
        assert!(resolve_action(
            "Deck(_)::jog_turn",
            "deck_1",
            ControlValue::Absolute(0.6),
            true,
            false,
            &snap,
        )
        .is_none());
    }

    #[test]
    fn relative_value_rejected_for_absolute_action() {
        let snap = ControlSnapshot::default();
        assert!(
            resolve_action(
                "Deck(_)::set_volume",
                "deck_1",
                ControlValue::Relative(3),
                true,
                true,
                &snap,
            )
            .is_none(),
            "relative ticks must not become SetVolume(0.0)"
        );
        assert!(resolve_action(
            "Deck(_)::set_filter",
            "deck_1",
            ControlValue::Relative(-1),
            true,
            true,
            &snap,
        )
        .is_none());
    }

    #[test]
    fn cycle_tempo_range_advances_steps() {
        let mut snap = ControlSnapshot::default();
        let routed = resolve_action(
            "Deck(_)::cycle_tempo_range",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SetTempoRange { tempo_range },
                ..
            } => assert!((tempo_range - 0.10).abs() < 1e-5),
            other => panic!("unexpected {other:?}"),
        }
        snap.tempo_range[0] = 0.25;
        let routed = resolve_action(
            "Deck(_)::cycle_tempo_range",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::EngineCmd {
                body: CmdBody::SetTempoRange { tempo_range },
                ..
            } => assert!((tempo_range - 0.06).abs() < 1e-5),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn load_to_deck_publishes_focused_load_evt() {
        let snap = ControlSnapshot::default();
        let routed = resolve_action(
            "LibraryNavigation::load_to_deck(deck:2)",
            "master",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match routed {
            RoutedAction::LibraryEvt {
                kind: LibraryKind::Load,
                body: LibraryEvtBody::Load { deck },
                ..
            } => assert_eq!(deck, 1),
            other => panic!("expected Load deck 1, got {other:?}"),
        }
        assert!(resolve_action(
            "LibraryNavigation::load_to_deck(deck:1)",
            "master",
            ControlValue::Absolute(1.0),
            false,
            false,
            &snap
        )
        .is_none());
    }
}
