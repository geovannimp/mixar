//! Action string → [`RoutedAction`].

use bus_api::BeatJump;
use bus_api::BeatJumpPadPress;
use bus_api::BeatJumpPadRelease;
use bus_api::BeginCueHold;
use bus_api::CrossfaderAssign;
use bus_api::DeleteDeckHotCue;
use bus_api::EndCueHold;
use bus_api::EqBand;
use bus_api::Event;
use bus_api::ExitLoop;
use bus_api::Focus;
use bus_api::FxKind;
use bus_api::FxSlotState;
use bus_api::HotCuePadPress;
use bus_api::HotCuePadRelease;
use bus_api::JogMode;
use bus_api::JogTouch;
use bus_api::JogTurn;
use bus_api::KeyShiftPadPress;
use bus_api::KeyShiftPadRelease;
use bus_api::KeyboardPadPress;
use bus_api::KeyboardPadRelease;
use bus_api::Load;
use bus_api::LoopIn;
use bus_api::LoopOut;
use bus_api::LoopRollPadPress;
use bus_api::LoopRollPadRelease;
use bus_api::Navigate;
use bus_api::PadMode;
use bus_api::PadPress;
use bus_api::PadRelease;
use bus_api::Pause;
use bus_api::Play;
use bus_api::SamplerPadPress;
use bus_api::SamplerPadRelease;
use bus_api::SaveDeckHotCue;
use bus_api::SetAutoLoop;
use bus_api::SetCrossfader;
use bus_api::SetCrossfaderAssign;
use bus_api::SetCueMix;
use bus_api::SetCuePoint;
use bus_api::SetCueVolume;
use bus_api::SetDeckFxRouted;
use bus_api::SetEqBand;
use bus_api::SetFilter;
use bus_api::SetFxUnitParam;
use bus_api::SetFxUnitSlot;
use bus_api::SetFxUnitSlotEnabled;
use bus_api::SetFxUnitWet;
use bus_api::SetGainTrim;
use bus_api::SetMasterVolume;
use bus_api::SetPadMode;
use bus_api::SetSpeed;
use bus_api::SetTempoRange;
use bus_api::SetVolume;
use bus_api::ToggleHeadphoneCue;
use bus_api::ToggleMasterCue;
use bus_api::TogglePlay;
use bus_api::ToggleQuantize;
use bus_api::ToggleSync;
use bus_api::TriggerHotCue;
use bus_api::DEFAULT_PITCH_PAGE;
use bus_api::FX_DIVISION_COUNT;

use crate::action_id::{bind_origin, parse_action_id, BoundOrigin};

/// Library hot-cue slots are 0..=15; MIDI `pad n` and LED cache match that.
pub const HOT_CUE_SLOT_COUNT: usize = 16;

/// Beat FX unit slot (slot 0 is the Beat FX unit on every shipped bundle).
const FX_SLOT: u8 = 0;

/// FX SELECT cycle: the unit always holds an effect, so `Off` is not a stop.
const FX_SELECT_CYCLE: [FxKind; 2] = [FxKind::Echo, FxKind::Reverb];

/// Engine side a bound action addresses: one deck, the mixer strip, or the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineScope {
    Deck(u16),
    Mixer,
    Engine,
}

impl EngineScope {
    /// Deck id, defaulting to deck 0 for scopes that address no deck.
    pub(crate) fn deck_or_default(self) -> u16 {
        match self {
            Self::Deck(deck) => deck,
            Self::Mixer | Self::Engine => 0,
        }
    }
}

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
    /// Session key-shift offset in semitones (Keyboard / Key Shift pad modes).
    pub key_shift: [f32; 4],
    /// Keyboard pad semitone page (`1..=4`).
    pub keyboard_page: [u8; 4],
    /// Key Shift pad semitone page (`1..=5`).
    pub key_shift_page: [u8; 4],
    /// Hot-cue slot used as the Keyboard pad root.
    pub keyboard_root_hot_cue: [u8; 4],
    pub crossfader: f32,
    pub master_volume: f32,
    pub cue_volume: f32,
    pub cue_mix: f32,
    pub master_cue: bool,
    /// Shared Beat FX unit slot 0 (engine boot state; see [`default_fx_slot`]).
    pub fx: FxSlotState,
    pub cue_hold: [bool; 4],
    pub sync: [bool; 4],
    pub loop_active: [bool; 4],
    /// Loop In marker set (pending entrance and/or active loop).
    pub loop_in_set: [bool; 4],
    pub track_loaded: [bool; 4],
    /// Saved-loop slot presence per deck (loop-roll pad LEDs).
    pub loop_slots: [[bool; crate::catalog::LOOP_SLOT_COUNT]; 4],
    /// Last known playhead ms per deck (from Position / DeckUpdated).
    pub position_ms: [i32; 4],
    /// Hot cue positions ms per deck/slot (None = empty).
    pub hot_cues: [[Option<i32>; HOT_CUE_SLOT_COUNT]; 4],
}

/// Engine deck state mirrored into a [`MappingSession`](crate::session::MappingSession)
/// for LED feedback. Hosts build this from `DeckUpdated` / `EngineStatus`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DeckFeedback {
    pub playing: bool,
    pub sync: bool,
    pub quantize: bool,
    pub headphone_cue: bool,
    pub loop_active: bool,
    /// True when a Loop In marker exists (pending) or a loop is active.
    pub loop_in_set: bool,
    pub track_loaded: bool,
    pub loop_slots: [bool; crate::catalog::LOOP_SLOT_COUNT],
    pub pad_mode: PadMode,
    pub hot_cues: [Option<i32>; HOT_CUE_SLOT_COUNT],
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
            cue_hold: [false; 4],
            sync: [false; 4],
            loop_active: [false; 4],
            loop_in_set: [false; 4],
            track_loaded: [false; 4],
            loop_slots: [[false; crate::catalog::LOOP_SLOT_COUNT]; 4],
            pad_mode: [PadMode::HotCue; 4],
            key_shift: [0.0; 4],
            keyboard_page: [DEFAULT_PITCH_PAGE; 4],
            key_shift_page: [DEFAULT_PITCH_PAGE; 4],
            keyboard_root_hot_cue: [0; 4],
            crossfader: 0.5,
            master_volume: 1.0,
            cue_volume: 1.0,
            cue_mix: 0.5,
            master_cue: false,
            fx: FxSlotState::boot(),
            position_ms: [0; 4],
            hot_cues: [[None; HOT_CUE_SLOT_COUNT]; 4],
        }
    }
}

impl ControlSnapshot {
    pub fn set_value(&mut self, scope: EngineScope, key: &str, value: f32) {
        match scope {
            EngineScope::Mixer => match key {
                "crossfader" => self.crossfader = value,
                "master_volume" => self.master_volume = value,
                "cue_volume" => self.cue_volume = value,
                "cue_mix" => self.cue_mix = value,
                "master_cue" => self.master_cue = value > 0.5,
                _ => {}
            },
            EngineScope::Deck(d) => {
                let Some(i) = deck_idx(d) else {
                    return;
                };
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
                    "sync" => self.sync[i] = value > 0.5,
                    "cue_hold" => self.cue_hold[i] = value > 0.5,
                    "loop_active" => self.loop_active[i] = value > 0.5,
                    "loop_in_set" => self.loop_in_set[i] = value > 0.5,
                    "track_loaded" => self.track_loaded[i] = value > 0.5,
                    "pad_mode" => {
                        self.pad_mode[i] = match value as u8 {
                            1 => PadMode::LoopRoll,
                            2 => PadMode::BeatJump,
                            3 => PadMode::Sampler,
                            4 => PadMode::Stems,
                            _ => PadMode::HotCue,
                        };
                    }
                    _ => {}
                }
            }
            EngineScope::Engine => {}
        }
    }

    /// Resolve a `map.toml` output `signal` name against this snapshot.
    ///
    /// `pad_<mode>_<n>` is mode-gated so only the deck's active pad bank lights up;
    /// `shift_held` is resolved by the session (it lives in the modifier set).
    pub fn signal(&self, section: &str, name: &str) -> Option<bool> {
        if section == crate::device::SECTION_MASTER {
            return match name {
                "master_cue" => Some(self.master_cue),
                "fx_armed" => Some(self.fx.enabled),
                _ => None,
            };
        }
        let i = deck_idx(crate::device::origin_deck_id(section)?)?;
        Some(match name {
            "playing" => self.playing[i],
            "cue_hold" => self.cue_hold[i],
            "sync" => self.sync[i],
            "quantize" => self.quantize[i],
            "headphone_cue" => self.headphone_cue[i],
            "track_loaded" => self.track_loaded[i],
            "loop_active" => self.loop_active[i],
            "loop_in_set" => self.loop_in_set[i],
            "pad_mode_hot_cue" => self.pad_mode[i] == PadMode::HotCue,
            "pad_mode_loop_roll" => self.pad_mode[i] == PadMode::LoopRoll,
            "pad_mode_beat_jump" => self.pad_mode[i] == PadMode::BeatJump,
            "pad_mode_sampler" => self.pad_mode[i] == PadMode::Sampler,
            "pad_mode_stems" => self.pad_mode[i] == PadMode::Stems,
            "pad_mode_keyboard" => self.pad_mode[i] == PadMode::Keyboard,
            "pad_mode_key_shift" => self.pad_mode[i] == PadMode::KeyShift,
            _ => {
                return self.slot_signal(i, name);
            }
        })
    }

    /// `hot_cue_<n>` (cue saved), `loop_slot_<n>` (loop saved) and the mode-gated
    /// `pad_<mode>_<n>` pad-bank signals.
    fn slot_signal(&self, i: usize, name: &str) -> Option<bool> {
        if let Some(n) = numbered(name, "hot_cue_") {
            return Some(self.hot_cues[i].get(n - 1).copied().flatten().is_some());
        }
        if let Some(n) = numbered(name, "loop_slot_") {
            return Some(self.loop_slots[i].get(n - 1).copied().unwrap_or(false));
        }
        let mode = match name.split_once('_') {
            Some(("pad", rest)) => pad_mode_from_signal(rest)?,
            _ => return None,
        };
        let n = name.rsplit('_').next()?.parse::<usize>().ok()?;
        if self.pad_mode[i] != mode || n == 0 || n > crate::catalog::PAD_SLOT_COUNT {
            return Some(false);
        }
        Some(match mode {
            PadMode::HotCue => self.hot_cues[i].get(n - 1).copied().flatten().is_some(),
            PadMode::LoopRoll => self.loop_slots[i].get(n - 1).copied().unwrap_or(false),
            // Keyboard / Key Shift pad banks are host-driven (signal-less outputs);
            // declarative `pad_keyboard_*` / `pad_key_shift_*` are not advertised.
            _ => false,
        })
    }
}

fn numbered(name: &str, prefix: &str) -> Option<usize> {
    let n = name.strip_prefix(prefix)?.parse::<usize>().ok()?;
    (n > 0).then_some(n)
}

/// `hot_cue` | `loop` | `beat_jump` | `sampler` | `stems` → pad mode.
///
/// Keyboard / Key Shift pad-bank LEDs stay host-driven via signal-less outputs;
/// do not parse them here (that would advertise inert `pad_keyboard_*` signals).
fn pad_mode_from_signal(rest: &str) -> Option<PadMode> {
    let (mode, n) = rest.rsplit_once('_')?;
    if n.parse::<usize>().is_err() {
        return None;
    }
    match mode {
        "hot_cue" => Some(PadMode::HotCue),
        "loop" => Some(PadMode::LoopRoll),
        "beat_jump" => Some(PadMode::BeatJump),
        "sampler" => Some(PadMode::Sampler),
        "stems" => Some(PadMode::Stems),
        _ => None,
    }
}

fn deck_idx(d: u16) -> Option<usize> {
    (d < 4).then_some(d as usize)
}

/// Normalized selector bands for a mapped three-state crossfader assignment.
const CROSSFADER_ASSIGN_A_BAND: f32 = 1.0 / 3.0;
const CROSSFADER_ASSIGN_THRU_BAND: f32 = 2.0 / 3.0;

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
///
/// `Event` is a fat enum (`DeckUpdated` carries a whole deck); boxing it would
/// allocate on every MIDI edge, so the enum stays inline and is built once per
/// control press.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum RoutedAction {
    /// One command for the engine or library worker.
    Cmd(Event),
    /// One physical control publishing several cmds (exclusive routing groups).
    Cmds(Vec<Event>),
    /// Egress event the host re-broadcasts (library navigate/focus/load).
    Evt(Event),
}

/// Deck-scoped command: `None` when the bound section addresses no deck.
fn deck_cmd(scope: EngineScope, build: impl FnOnce(u16) -> Event) -> Option<RoutedAction> {
    match scope {
        EngineScope::Deck(deck) => Some(RoutedAction::Cmd(build(deck))),
        EngineScope::Mixer | EngineScope::Engine => None,
    }
}

/// Deck-scoped command gated on a press edge.
fn deck_press(
    active: bool,
    scope: EngineScope,
    build: impl FnOnce(u16) -> Event,
) -> Option<RoutedAction> {
    if active {
        deck_cmd(scope, build)
    } else {
        None
    }
}

/// Mixer-scoped command (no deck to fill in).
fn mixer_cmd(event: impl FnOnce() -> Event) -> Option<RoutedAction> {
    Some(RoutedAction::Cmd(event()))
}

/// Mixer-scoped command gated on a press edge.
fn mixer_press(active: bool, event: impl FnOnce() -> Event) -> Option<RoutedAction> {
    if active {
        mixer_cmd(event)
    } else {
        None
    }
}

/// Arm or disarm the shared Beat FX unit ([`FX_SLOT`]).
fn fx_unit_enabled(enabled: bool) -> Option<RoutedAction> {
    mixer_cmd(|| {
        Event::SetFxUnitSlotEnabled(SetFxUnitSlotEnabled {
            slot: FX_SLOT,
            enabled,
        })
    })
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
                return Some(RoutedAction::Evt(Event::Navigate(Navigate { delta: 1 })));
            }
            "navigate_prev" => {
                if !active {
                    return None;
                }
                return Some(RoutedAction::Evt(Event::Navigate(Navigate { delta: -1 })));
            }
            // Relative select-knob: delta already decoded from device `relative` mode.
            "navigate" => {
                let delta = value.as_relative()?;
                if delta == 0 {
                    return None;
                }
                return Some(RoutedAction::Evt(Event::Navigate(Navigate { delta })));
            }
            // Rotary-selector press: step browser focus to the next pane.
            "focus_next" => {
                if !active {
                    return None;
                }
                return Some(RoutedAction::Evt(Event::Focus(Focus { delta: 1 })));
            }
            // SHIFT + rotary-selector press: step browser focus back.
            "focus_prev" => {
                if !active {
                    return None;
                }
                return Some(RoutedAction::Evt(Event::Focus(Focus { delta: -1 })));
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
                return Some(RoutedAction::Evt(Event::Load(Load { deck })));
            }
            _ => return None,
        }
    }

    let BoundOrigin::Engine(scope) = bound else {
        return None;
    };

    // Relative ticks are only for jog_turn (navigate handled above). Never coerce to 0.0.
    if matches!(value, ControlValue::Relative(_)) && leaf != "jog_turn" {
        return None;
    }
    let norm = value.as_absolute().unwrap_or(0.0);

    // Buttons: only fire on press (active edge handled by caller).
    match leaf {
        "toggle_play" => deck_press(active, scope, |deck| Event::TogglePlay(TogglePlay { deck })),
        "play" => deck_press(active, scope, |deck| Event::Play(Play { deck })),
        "pause" => deck_press(active, scope, |deck| Event::Pause(Pause { deck })),
        "cue" => deck_press(active, scope, |deck| {
            Event::SetCuePoint(SetCuePoint { deck })
        }),
        "cue_default" => {
            if active {
                deck_cmd(scope, |deck| Event::BeginCueHold(BeginCueHold { deck }))
            } else {
                deck_cmd(scope, |deck| Event::EndCueHold(EndCueHold { deck }))
            }
        }
        "begin_cue_hold" => deck_press(active, scope, |deck| {
            Event::BeginCueHold(BeginCueHold { deck })
        }),
        "end_cue_hold" => {
            if active {
                None
            } else {
                deck_cmd(scope, |deck| Event::EndCueHold(EndCueHold { deck }))
            }
        }
        "toggle_sync" => deck_press(active, scope, |deck| {
            Event::ToggleSync(ToggleSync {
                deck,
                beat_sync: false,
            })
        }),
        "set_quantize" => deck_press(active, scope, |deck| {
            Event::ToggleQuantize(ToggleQuantize { deck })
        }),
        "set_volume" => deck_cmd(scope, |deck| {
            Event::SetVolume(SetVolume {
                deck,
                volume: norm.clamp(0.0, 1.0),
                soft_takeover,
            })
        }),
        "set_filter" => deck_cmd(scope, |deck| {
            Event::SetFilter(SetFilter {
                deck,
                filter: norm.clamp(0.0, 1.0),
                soft_takeover,
            })
        }),
        "set_gain" => deck_cmd(scope, |deck| {
            Event::SetGainTrim(SetGainTrim {
                deck,
                gain_trim: norm.clamp(0.0, 1.0),
                soft_takeover,
            })
        }),
        "set_speed" => deck_cmd(scope, |deck| {
            Event::SetSpeed(SetSpeed {
                deck,
                speed: norm.clamp(0.0, 1.0),
                soft_takeover,
            })
        }),
        "cycle_tempo_range" => {
            if !active {
                return None;
            }
            let EngineScope::Deck(d) = scope else {
                return None;
            };
            let i = deck_idx(d)?;
            let next = next_tempo_range(snap.tempo_range[i]);
            deck_cmd(scope, |deck| {
                Event::SetTempoRange(SetTempoRange {
                    deck,
                    tempo_range: next,
                })
            })
        }
        "set_eq_low" | "set_eq_mid" | "set_eq_high" => {
            let band = match leaf {
                "set_eq_low" => EqBand::Low,
                "set_eq_mid" => EqBand::Mid,
                "set_eq_high" => EqBand::High,
                _ => return None,
            };
            deck_cmd(scope, |deck| {
                Event::SetEqBand(SetEqBand {
                    deck,
                    band,
                    gain: norm.clamp(0.0, 1.0),
                    soft_takeover,
                })
            })
        }
        "set_crossfader_assign" => {
            let assignment = if norm < CROSSFADER_ASSIGN_A_BAND {
                CrossfaderAssign::A
            } else if norm < CROSSFADER_ASSIGN_THRU_BAND {
                CrossfaderAssign::Thru
            } else {
                CrossfaderAssign::B
            };
            deck_cmd(scope, |deck| {
                Event::SetCrossfaderAssign(SetCrossfaderAssign { deck, assignment })
            })
        }
        "set_crossfader" => mixer_cmd(|| {
            Event::SetCrossfader(SetCrossfader {
                position: norm.clamp(0.0, 1.0),
                soft_takeover,
            })
        }),
        "set_master_volume" => mixer_cmd(|| {
            Event::SetMasterVolume(SetMasterVolume {
                volume: norm.clamp(0.0, 1.0),
                soft_takeover,
            })
        }),
        "set_cue_volume" => mixer_cmd(|| {
            Event::SetCueVolume(SetCueVolume {
                volume: norm.clamp(0.0, 1.0),
                soft_takeover,
            })
        }),
        "set_cue_mix" => mixer_cmd(|| {
            Event::SetCueMix(SetCueMix {
                mix: norm.clamp(0.0, 1.0),
                soft_takeover,
            })
        }),
        "set_master_cue" => mixer_press(active, || Event::ToggleMasterCue(ToggleMasterCue {})),
        // Beat FX: ON/OFF arms the shared unit; SHIFT+ON/OFF disarms it.
        "toggle_fx" => active.then(|| fx_unit_enabled(!snap.fx.enabled)).flatten(),
        "disarm_fx" => active.then(|| fx_unit_enabled(false)).flatten(),
        "cycle_fx_next" | "cycle_fx_prev" => {
            if !active {
                return None;
            }
            let len = FX_SELECT_CYCLE.len();
            let i = FX_SELECT_CYCLE
                .iter()
                .position(|k| *k == snap.fx.kind)
                .unwrap_or(0);
            let next = if leaf == "cycle_fx_next" {
                (i + 1) % len
            } else {
                (i + len - 1) % len
            };
            mixer_cmd(|| {
                Event::SetFxUnitSlot(SetFxUnitSlot {
                    slot: FX_SLOT,
                    kind: FX_SELECT_CYCLE[next],
                })
            })
        }
        // BEAT ◀▶ steps Echo's division knob; other effects have no beat table.
        "step_fx_division_down" | "step_fx_division_up" => {
            if !active || snap.fx.kind != FxKind::Echo {
                return None;
            }
            let last = (FX_DIVISION_COUNT - 1) as f32;
            let idx = (snap.fx.params[0] * last).round().clamp(0.0, last) as i32;
            let dir: i32 = if leaf == "step_fx_division_up" { 1 } else { -1 };
            let next = (idx + dir).clamp(0, FX_DIVISION_COUNT as i32 - 1);
            if next == idx {
                return None;
            }
            mixer_cmd(|| {
                Event::SetFxUnitParam(SetFxUnitParam {
                    slot: FX_SLOT,
                    param: 0,
                    value: next as f32 / last,
                })
            })
        }
        "set_fx_wet" => mixer_cmd(|| {
            Event::SetFxUnitWet(SetFxUnitWet {
                slot: FX_SLOT,
                wet: norm.clamp(0.0, 1.0),
                soft_takeover,
            })
        }),
        // CH SELECT is exclusive: every press publishes the full routing group.
        "set_fx_route" => {
            if !active {
                return None;
            }
            let routed =
                |deck: u16, on: bool| Event::SetDeckFxRouted(SetDeckFxRouted { deck, routed: on });
            let cmds = match args.require_ident("target").ok()? {
                "ch1" => vec![routed(0, true), routed(1, false)],
                "ch2" => vec![routed(0, false), routed(1, true)],
                // No master-bus FX insert: both decks routed is the closest analog.
                "master" => vec![routed(0, true), routed(1, true)],
                _ => return None,
            };
            Some(RoutedAction::Cmds(cmds))
        }
        "set_headphone_cue" => deck_press(active, scope, |deck| {
            Event::ToggleHeadphoneCue(ToggleHeadphoneCue { deck })
        }),
        "jog_touch" => deck_cmd(scope, |deck| {
            Event::JogTouch(JogTouch {
                deck,
                touching: active,
            })
        }),
        "jog_turn" => {
            let delta = value.as_relative()?;
            if delta == 0 {
                return None;
            }
            deck_cmd(scope, |deck| Event::JogTurn(JogTurn { deck, delta }))
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
            let EngineScope::Deck(d) = scope else {
                return None;
            };
            let i = deck_idx(d)?;
            let cues = &snap.hot_cues[i];
            let idx = (slot_u - 1) as usize;
            if idx >= cues.len() {
                return deck_cmd(scope, |deck| {
                    Event::SaveDeckHotCue(SaveDeckHotCue {
                        deck,
                        slot: slot_u - 1,
                    })
                });
            }
            match cues[idx] {
                Some(pos) => deck_cmd(scope, |deck| {
                    Event::TriggerHotCue(TriggerHotCue {
                        deck,
                        position_ms: pos,
                    })
                }),
                None => deck_cmd(scope, |deck| {
                    Event::SaveDeckHotCue(SaveDeckHotCue {
                        deck,
                        slot: slot_u - 1,
                    })
                }),
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
            deck_cmd(scope, |deck| {
                Event::DeleteDeckHotCue(DeleteDeckHotCue {
                    deck,
                    slot: (slot as u8) - 1,
                })
            })
        }
        "loop_in" => {
            if !active {
                return None;
            }
            let EngineScope::Deck(d) = scope else {
                return None;
            };
            let i = deck_idx(d)?;
            deck_cmd(scope, |deck| {
                Event::LoopIn(LoopIn {
                    deck,
                    position_ms: snap.position_ms[i],
                })
            })
        }
        "loop_out" => {
            if !active {
                return None;
            }
            let EngineScope::Deck(d) = scope else {
                return None;
            };
            let i = deck_idx(d)?;
            deck_cmd(scope, |deck| {
                Event::LoopOut(LoopOut {
                    deck,
                    position_ms: snap.position_ms[i],
                })
            })
        }
        "exit_loop" => deck_press(active, scope, |deck| Event::ExitLoop(ExitLoop { deck })),
        "auto_loop" => {
            if !active {
                return None;
            }
            let beats = args.require_f32("beats").ok()?;
            if !beats.is_finite() || beats <= 0.0 {
                return None;
            }
            deck_cmd(scope, |deck| {
                Event::SetAutoLoop(SetAutoLoop { deck, beats })
            })
        }
        "beat_jump" => {
            if !active {
                return None;
            }
            let beats = args.require_f32("beats").ok()?;
            if !beats.is_finite() || beats == 0.0 {
                return None;
            }
            deck_cmd(scope, |deck| Event::BeatJump(BeatJump { deck, beats }))
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
                "stems" => PadMode::Stems,
                "keyboard" => PadMode::Keyboard,
                "key_shift" => PadMode::KeyShift,
                _ => return None,
            };
            deck_cmd(scope, |deck| Event::SetPadMode(SetPadMode { deck, mode }))
        }
        "pad" => {
            let slot = pad_n_slot(&args)?;
            if active {
                deck_cmd(scope, |deck| {
                    Event::PadPress(PadPress {
                        deck,
                        slot,
                        shift: false,
                    })
                })
            } else {
                deck_cmd(scope, |deck| Event::PadRelease(PadRelease { deck, slot }))
            }
        }
        "hot_cue_pad" => {
            resolve_pad_slot(scope, pad_n_slot(&args)?, active, false, PadMode::HotCue)
        }
        "loop_roll_pad" => {
            resolve_pad_slot(scope, pad_n_slot(&args)?, active, false, PadMode::LoopRoll)
        }
        "beat_jump_pad" => {
            resolve_pad_slot(scope, pad_n_slot(&args)?, active, false, PadMode::BeatJump)
        }
        "sampler_pad" => {
            resolve_pad_slot(scope, pad_n_slot(&args)?, active, false, PadMode::Sampler)
        }
        "keyboard_pad" => resolve_pad_slot(
            scope,
            pad_n_slot(&args)?,
            active,
            pad_shift(&args)?,
            PadMode::Keyboard,
        ),
        "key_shift_pad" => resolve_pad_slot(
            scope,
            pad_n_slot(&args)?,
            active,
            pad_shift(&args)?,
            PadMode::KeyShift,
        ),
        "trigger_sampler" => {
            let slot = args.require_int("slot").ok()?;
            if slot < 1 {
                return None;
            }
            let slot = u8::try_from(slot - 1).ok()?;
            if active {
                deck_cmd(scope, |deck| {
                    Event::SamplerPadPress(SamplerPadPress {
                        deck,
                        slot,
                        shift: false,
                    })
                })
            } else {
                deck_cmd(scope, |deck| {
                    Event::SamplerPadRelease(SamplerPadRelease { deck, slot })
                })
            }
        }
        _ => None,
    }
}

fn pad_n_slot(args: &crate::action_id::ActionArgs) -> Option<u8> {
    let n = args.require_int("n").ok()?;
    u8::try_from(n.checked_sub(1)?).ok()
}

/// Optional `shift` bank selector (defaults to `false` when omitted).
///
/// A malformed `shift` arg is treated as a mapping error (returns `None` upstream).
fn pad_shift(args: &crate::action_id::ActionArgs) -> Option<bool> {
    match args.optional_bool("shift").ok()? {
        Some(v) => Some(v),
        None => Some(false),
    }
}

/// Named pad leaves publish the mode-specific press/release pair (Pioneer note banks).
/// Generic `pad` publishes PadPress / PadRelease instead.
fn resolve_pad_slot(
    scope: EngineScope,
    slot: u8,
    active: bool,
    shift: bool,
    mode: PadMode,
) -> Option<RoutedAction> {
    deck_cmd(scope, |deck| match (mode, active) {
        (PadMode::HotCue, true) => Event::HotCuePadPress(HotCuePadPress {
            deck,
            slot,
            shift: false,
        }),
        (PadMode::HotCue, false) => Event::HotCuePadRelease(HotCuePadRelease { deck, slot }),
        (PadMode::LoopRoll, true) => Event::LoopRollPadPress(LoopRollPadPress { deck, slot }),
        (PadMode::LoopRoll, false) => Event::LoopRollPadRelease(LoopRollPadRelease { deck, slot }),
        (PadMode::BeatJump, true) => Event::BeatJumpPadPress(BeatJumpPadPress { deck, slot }),
        (PadMode::BeatJump, false) => Event::BeatJumpPadRelease(BeatJumpPadRelease { deck, slot }),
        (PadMode::Sampler, true) => Event::SamplerPadPress(SamplerPadPress {
            deck,
            slot,
            shift: false,
        }),
        (PadMode::Sampler, false) => Event::SamplerPadRelease(SamplerPadRelease { deck, slot }),
        (PadMode::Stems, true) => Event::PadPress(PadPress {
            deck,
            slot,
            shift: false,
        }),
        (PadMode::Stems, false) => Event::PadRelease(PadRelease { deck, slot }),
        (PadMode::Keyboard, true) => {
            Event::KeyboardPadPress(KeyboardPadPress { deck, slot, shift })
        }
        (PadMode::Keyboard, false) => Event::KeyboardPadRelease(KeyboardPadRelease { deck, slot }),
        (PadMode::KeyShift, true) => {
            Event::KeyShiftPadPress(KeyShiftPadPress { deck, slot, shift })
        }
        (PadMode::KeyShift, false) => Event::KeyShiftPadRelease(KeyShiftPadRelease { deck, slot }),
    })
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
            RoutedAction::Cmd(Event::SetEqBand(SetEqBand {
                deck: 0,
                band: EqBand::Low,
                gain,
                soft_takeover: false,
            })) => assert!((gain - 1.0).abs() < 1e-5, "gain={gain}"),
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
            RoutedAction::Cmd(Event::SetEqBand(SetEqBand {
                deck: 0,
                band: EqBand::Mid,
                gain,
                soft_takeover: false,
            })) => assert!((gain - 0.5).abs() < 1e-5, "gain={gain}"),
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
            RoutedAction::Cmd(Event::SetFilter(SetFilter {
                deck: 0,
                filter,
                soft_takeover: false,
            })) => assert!((filter - 1.0).abs() < 1e-5, "filter={filter}"),
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
            RoutedAction::Cmd(Event::SetGainTrim(SetGainTrim {
                deck: 0,
                gain_trim,
                soft_takeover: false,
            })) => assert!((gain_trim - 1.0).abs() < 1e-5, "gain_trim={gain_trim}"),
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
            RoutedAction::Cmd(Event::PadPress(PadPress {
                deck: 0,
                slot,
                shift,
            })) => {
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
            RoutedAction::Cmd(Event::PadRelease(PadRelease { deck: 0, slot: 0 }))
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
                RoutedAction::Cmd(Event::PadPress(PadPress {
                    deck: 0,
                    slot: 0,
                    shift: false
                }))
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
            RoutedAction::Cmd(Event::HotCuePadPress(HotCuePadPress {
                deck: 0,
                slot,
                shift: false,
            })) => assert_eq!(slot, 0),
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
            RoutedAction::Cmd(Event::HotCuePadRelease(HotCuePadRelease {
                deck: 0,
                slot: 0
            }))
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
            RoutedAction::Cmd(Event::BeatJumpPadPress(BeatJumpPadPress { deck: 0, slot })) => {
                assert_eq!(slot, 0)
            }
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
            RoutedAction::Cmd(Event::LoopRollPadPress(LoopRollPadPress { deck: 0, slot })) => {
                assert_eq!(slot, 2)
            }
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
            RoutedAction::Cmd(Event::LoopRollPadRelease(LoopRollPadRelease {
                deck: 0,
                slot: 2
            }))
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
            RoutedAction::Cmd(Event::SamplerPadPress(SamplerPadPress {
                deck: 0,
                slot,
                shift: false,
            })) => assert_eq!(slot, 1),
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
            RoutedAction::Cmd(Event::SamplerPadRelease(SamplerPadRelease {
                deck: 0,
                slot: 1
            }))
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
            RoutedAction::Cmd(Event::PadPress(PadPress {
                deck: 0,
                slot,
                shift: false,
            })) => assert_eq!(slot, 8),
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
            RoutedAction::Cmd(Event::HotCuePadPress(HotCuePadPress {
                deck: 0,
                slot,
                shift: false,
            })) => assert_eq!(slot, 8),
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
            RoutedAction::Cmd(Event::SamplerPadPress(SamplerPadPress {
                deck: 0,
                slot,
                shift: false,
            })) => assert_eq!(slot, 1),
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
            RoutedAction::Cmd(Event::SamplerPadRelease(SamplerPadRelease {
                deck: 0,
                slot: 1
            }))
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
            RoutedAction::Cmd(Event::SetPadMode(SetPadMode { deck: 0, mode })) => {
                assert_eq!(mode, PadMode::LoopRoll)
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn loop_in_out_stamp_snapshot_position() {
        let mut snap = ControlSnapshot::default();
        snap.position_ms[0] = 12_345;
        let inn = resolve_action(
            "Deck(_)::loop_in",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match inn {
            RoutedAction::Cmd(Event::LoopIn(LoopIn {
                deck: 0,
                position_ms,
            })) => assert_eq!(position_ms, 12_345),
            other => panic!("unexpected {other:?}"),
        }
        let out = resolve_action(
            "Deck(_)::loop_out",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match out {
            RoutedAction::Cmd(Event::LoopOut(LoopOut {
                deck: 0,
                position_ms,
            })) => assert_eq!(position_ms, 12_345),
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
            RoutedAction::Cmd(Event::SetAutoLoop(SetAutoLoop { deck: 0, beats })) => {
                assert!((beats - 0.25).abs() < 1e-6)
            }
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
            RoutedAction::Evt(Event::Navigate(Navigate { delta })) => assert_eq!(delta, 1),
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
            RoutedAction::Evt(Event::Navigate(Navigate { delta })) => assert_eq!(delta, -1),
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
            RoutedAction::Cmd(Event::JogTurn(JogTurn { deck: 0, delta })) => {
                assert_eq!(delta, 3)
            }
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
            RoutedAction::Cmd(Event::SetTempoRange(SetTempoRange {
                deck: 0,
                tempo_range,
            })) => assert!((tempo_range - 0.10).abs() < 1e-5),
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
            RoutedAction::Cmd(Event::SetTempoRange(SetTempoRange {
                deck: 0,
                tempo_range,
            })) => assert!((tempo_range - 0.06).abs() < 1e-5),
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
            RoutedAction::Evt(Event::Load(Load { deck })) => assert_eq!(deck, 1),
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

    #[test]
    fn key_shift_pad_resolves_to_named_kind() {
        let snap = ControlSnapshot::default();
        let a = resolve_action(
            "Deck(_)::key_shift_pad(n:2)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        assert!(matches!(
            a,
            RoutedAction::Cmd(Event::KeyShiftPadPress(KeyShiftPadPress { deck: 0, .. }))
        ));
    }

    #[test]
    fn keyboard_and_key_shift_shift_bank_resolve() {
        let snap = ControlSnapshot::default();
        let keyboard = resolve_action(
            "Deck(_)::keyboard_pad(n:1, shift:true)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        assert!(matches!(
            keyboard,
            RoutedAction::Cmd(Event::KeyboardPadPress(KeyboardPadPress {
                shift: true,
                ..
            }))
        ));
        let key_shift = resolve_action(
            "Deck(_)::key_shift_pad(n:6, shift:true)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        assert!(matches!(
            key_shift,
            RoutedAction::Cmd(Event::KeyShiftPadPress(KeyShiftPadPress {
                shift: true,
                ..
            }))
        ));
        // Without `shift`, the main bank publishes a non-shift press.
        let plain = resolve_action(
            "Deck(_)::keyboard_pad(n:1)",
            "deck_1",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        assert!(matches!(
            plain,
            RoutedAction::Cmd(Event::KeyboardPadPress(KeyboardPadPress {
                shift: false,
                ..
            }))
        ));
    }

    #[test]
    fn focus_step_publishes_pane_focus_evt_on_press_only() {
        let snap = ControlSnapshot::default();
        let next = resolve_action(
            "LibraryNavigation::focus_next",
            "master",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match next {
            RoutedAction::Evt(Event::Focus(Focus { delta })) => assert_eq!(delta, 1),
            other => panic!("expected Focus +1, got {other:?}"),
        }
        let prev = resolve_action(
            "LibraryNavigation::focus_prev",
            "master",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .unwrap();
        match prev {
            RoutedAction::Evt(Event::Focus(Focus { delta })) => assert_eq!(delta, -1),
            other => panic!("expected Focus -1, got {other:?}"),
        }
        // Release edge is not a focus step.
        assert!(resolve_action(
            "LibraryNavigation::focus_next",
            "master",
            ControlValue::Absolute(0.0),
            false,
            false,
            &snap,
        )
        .is_none());
    }

    #[test]
    fn fx_on_off_reads_the_snapshot_armed_flag() {
        let mut snap = ControlSnapshot::default();
        assert!(!snap.fx.enabled, "a fresh unit is loaded but not armed");

        assert_eq!(
            resolve_action(
                "Mixer::toggle_fx",
                "master",
                ControlValue::Absolute(1.0),
                true,
                false,
                &snap,
            ),
            fx_unit_enabled(true)
        );

        snap.fx.enabled = true;
        assert_eq!(
            resolve_action(
                "Mixer::toggle_fx",
                "master",
                ControlValue::Absolute(1.0),
                true,
                false,
                &snap,
            ),
            fx_unit_enabled(false)
        );

        // Release edges never publish.
        assert!(resolve_action(
            "Mixer::toggle_fx",
            "master",
            ControlValue::Absolute(0.0),
            false,
            false,
            &snap,
        )
        .is_none());
    }

    #[test]
    fn fx_select_cycles_both_ways_over_the_shipped_effects() {
        let mut snap = ControlSnapshot::default();
        let selected = |snap: &ControlSnapshot, leaf: &str| match resolve_action(
            &format!("Mixer::{leaf}"),
            "master",
            ControlValue::Absolute(1.0),
            true,
            false,
            snap,
        )
        .unwrap()
        {
            RoutedAction::Cmd(Event::SetFxUnitSlot(SetFxUnitSlot { slot: 0, kind })) => kind,
            other => panic!("unexpected {other:?}"),
        };

        assert_eq!(selected(&snap, "cycle_fx_next"), FxKind::Reverb);
        assert_eq!(selected(&snap, "cycle_fx_prev"), FxKind::Reverb);
        snap.fx.kind = FxKind::Reverb;
        assert_eq!(selected(&snap, "cycle_fx_next"), FxKind::Echo);
        assert_eq!(selected(&snap, "cycle_fx_prev"), FxKind::Echo);
        snap.fx.kind = FxKind::Off;
        // Off is not in the cycle: stepping falls back to its first entry.
        assert_eq!(selected(&snap, "cycle_fx_next"), FxKind::Reverb);
    }

    #[test]
    fn fx_division_steps_clamp_and_require_echo() {
        let mut snap = ControlSnapshot::default();
        let step = |snap: &ControlSnapshot, leaf: &str| {
            resolve_action(
                &format!("Mixer::{leaf}"),
                "master",
                ControlValue::Absolute(1.0),
                true,
                false,
                snap,
            )
        };

        // Boot division is index 3 of 7 → 4/6 after one up-step.
        match step(&snap, "step_fx_division_up").unwrap() {
            RoutedAction::Cmd(Event::SetFxUnitParam(SetFxUnitParam {
                slot: 0,
                param: 0,
                value,
            })) => assert!((value - 4.0 / 6.0).abs() < 1e-6, "value={value}"),
            other => panic!("unexpected {other:?}"),
        }

        snap.fx.params[0] = 1.0;
        assert!(step(&snap, "step_fx_division_up").is_none(), "clamped at 6");
        snap.fx.params[0] = 0.0;
        assert!(
            step(&snap, "step_fx_division_down").is_none(),
            "clamped at 0"
        );

        snap.fx.params[0] = 0.5;
        snap.fx.kind = FxKind::Reverb;
        assert!(
            step(&snap, "step_fx_division_up").is_none(),
            "the division table belongs to Echo"
        );
    }

    #[test]
    fn fx_wet_knob_publishes_the_gate_flag() {
        let snap = ControlSnapshot::default();
        match resolve_action(
            "Mixer::set_fx_wet",
            "master",
            ControlValue::Absolute(0.25),
            true,
            true,
            &snap,
        )
        .unwrap()
        {
            RoutedAction::Cmd(Event::SetFxUnitWet(SetFxUnitWet {
                slot: 0,
                wet,
                soft_takeover: true,
            })) => assert!((wet - 0.25).abs() < 1e-6, "wet={wet}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn fx_route_publishes_the_full_exclusive_group() {
        let snap = ControlSnapshot::default();
        let route = |leaf: &str| {
            resolve_action(
                &format!("Mixer::set_fx_route(target:{leaf})"),
                "master",
                ControlValue::Absolute(1.0),
                true,
                false,
                &snap,
            )
        };
        let routed =
            |deck: u16, on: bool| Event::SetDeckFxRouted(SetDeckFxRouted { deck, routed: on });

        assert_eq!(
            route("ch1").unwrap(),
            RoutedAction::Cmds(vec![routed(0, true), routed(1, false)])
        );
        assert_eq!(
            route("ch2").unwrap(),
            RoutedAction::Cmds(vec![routed(0, false), routed(1, true)])
        );
        assert_eq!(
            route("master").unwrap(),
            RoutedAction::Cmds(vec![routed(0, true), routed(1, true)])
        );

        // Unknown targets, releases and missing args resolve to nothing.
        assert!(route("bogus").is_none());
        assert!(resolve_action(
            "Mixer::set_fx_route(target:ch1)",
            "master",
            ControlValue::Absolute(0.0),
            false,
            false,
            &snap,
        )
        .is_none());
        assert!(resolve_action(
            "Mixer::set_fx_route",
            "master",
            ControlValue::Absolute(1.0),
            true,
            false,
            &snap,
        )
        .is_none());
    }
    #[test]
    fn crossfader_assign_midi_bands_include_deck_four_and_boundaries() {
        let snap = ControlSnapshot::default();
        let map = |value| {
            resolve_action(
                "Deck(_)::set_crossfader_assign",
                "deck_4",
                ControlValue::Absolute(value),
                true,
                false,
                &snap,
            )
        };
        for (value, expected) in [
            (0.0, CrossfaderAssign::A),
            (1.0 / 3.0 - 0.0001, CrossfaderAssign::A),
            (1.0 / 3.0, CrossfaderAssign::Thru),
            (2.0 / 3.0 - 0.0001, CrossfaderAssign::Thru),
            (2.0 / 3.0, CrossfaderAssign::B),
            (1.0, CrossfaderAssign::B),
        ] {
            assert_eq!(
                map(value),
                Some(RoutedAction::Cmd(Event::SetCrossfaderAssign(
                    SetCrossfaderAssign {
                        deck: 3,
                        assignment: expected
                    }
                )))
            );
        }
    }

    #[test]
    fn fx_armed_signal_tracks_the_shared_unit() {
        let mut snap = ControlSnapshot::default();
        assert_eq!(snap.signal("master", "fx_armed"), Some(false));
        snap.fx.enabled = true;
        assert_eq!(snap.signal("master", "fx_armed"), Some(true));
        assert_eq!(snap.signal("master", "nonsense"), None);
    }
}
