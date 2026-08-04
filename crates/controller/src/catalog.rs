//! Closed alias + action vocabularies (single source of truth).

use crate::action_id::{parse_action_id, OriginTemplate};

/// Action name as written in `map.toml` (`OriginTemplate::leaf`).
pub type ActionName = str;

const DECK_ALIASES: &[&str] = &[
    "play_pause",
    "cue",
    "cue_hold",
    "sync",
    "quantize",
    "volume",
    "gain",
    "tempo",
    "eq_high",
    "eq_mid",
    "eq_low",
    "filter",
    "jog_touch",
    "jog_turn",
    "jog_side",
    "jog_touch_shift",
    "jog_search",
    "headphone_cue",
    "hot_cue_1",
    "hot_cue_2",
    "hot_cue_3",
    "hot_cue_4",
    "hot_cue_5",
    "hot_cue_6",
    "hot_cue_7",
    "hot_cue_8",
    "delete_hot_cue_1",
    "delete_hot_cue_2",
    "delete_hot_cue_3",
    "delete_hot_cue_4",
    "delete_hot_cue_5",
    "delete_hot_cue_6",
    "delete_hot_cue_7",
    "delete_hot_cue_8",
    "pad_1",
    "pad_2",
    "pad_3",
    "pad_4",
    "pad_5",
    "pad_6",
    "pad_7",
    "pad_8",
    "loop_in",
    "loop_out",
    "exit_loop",
    "auto_loop",
    "auto_loop_1",
    "auto_loop_2",
    "auto_loop_4",
    "auto_loop_8",
    "auto_loop_16",
    "auto_loop_32",
    "beat_jump_fwd",
    "beat_jump_back",
    "beat_jump_fwd_1",
    "beat_jump_back_1",
    "beat_jump_fwd_2",
    "beat_jump_back_2",
    "beat_jump_fwd_4",
    "beat_jump_back_4",
    "beat_jump_fwd_8",
    "beat_jump_back_8",
    "pad_mode_hot_cue",
    "pad_mode_loop_roll",
    "pad_mode_beat_jump",
    "pad_mode_sampler",
];

const MASTER_ALIASES: &[&str] = &[
    "crossfader",
    "cue_mix",
    "master_cue",
    "headphone_cue_1",
    "headphone_cue_2",
    "headphone_cue_3",
    "headphone_cue_4",
];

const SAMPLER_ALIASES: &[&str] = &[
    "trigger_1",
    "trigger_2",
    "trigger_3",
    "trigger_4",
    "trigger_5",
    "trigger_6",
    "trigger_7",
    "trigger_8",
    "end_1",
    "end_2",
    "end_3",
    "end_4",
    "end_5",
    "end_6",
    "end_7",
    "end_8",
];

/// Leaves that map 1:1 to absolute CC / faders (default soft-takeover on).
pub const ABSOLUTE_LEAVES: &[&str] = &[
    "set_volume",
    "set_filter",
    "set_gain",
    "set_eq_high",
    "set_eq_mid",
    "set_eq_low",
    "set_crossfader",
    "set_cue_mix",
    "set_speed",
];

const DECK_LEAVES: &[&str] = &[
    "toggle_play",
    "play",
    "pause",
    "cue",
    "cue_default",
    "begin_cue_hold",
    "end_cue_hold",
    "toggle_sync",
    "set_quantize",
    "set_volume",
    "set_filter",
    "set_gain",
    "set_eq_high",
    "set_eq_mid",
    "set_eq_low",
    "set_speed",
    "set_headphone_cue",
    "jog_touch",
    "jog_turn",
    "trigger_hot_cue_1",
    "trigger_hot_cue_2",
    "trigger_hot_cue_3",
    "trigger_hot_cue_4",
    "trigger_hot_cue_5",
    "trigger_hot_cue_6",
    "trigger_hot_cue_7",
    "trigger_hot_cue_8",
    "delete_hot_cue_1",
    "delete_hot_cue_2",
    "delete_hot_cue_3",
    "delete_hot_cue_4",
    "delete_hot_cue_5",
    "delete_hot_cue_6",
    "delete_hot_cue_7",
    "delete_hot_cue_8",
    "loop_in",
    "loop_out",
    "exit_loop",
    "auto_loop_1",
    "auto_loop_2",
    "auto_loop_4",
    "auto_loop_8",
    "auto_loop_16",
    "auto_loop_32",
    "beat_jump_fwd_1",
    "beat_jump_back_1",
    "beat_jump_fwd_2",
    "beat_jump_back_2",
    "beat_jump_fwd_4",
    "beat_jump_back_4",
    "beat_jump_fwd_8",
    "beat_jump_back_8",
    "pad_mode_hot_cue",
    "pad_mode_loop_roll",
    "pad_mode_beat_jump",
    "pad_mode_sampler",
    "trigger_sampler_1",
    "trigger_sampler_2",
    "trigger_sampler_3",
    "trigger_sampler_4",
    "trigger_sampler_5",
    "trigger_sampler_6",
    "trigger_sampler_7",
    "trigger_sampler_8",
];

const MIXER_LEAVES: &[&str] = &["set_crossfader", "set_cue_mix", "set_master_cue"];

const ENGINE_LEAVES: &[&str] = &["start_engine"];

const LIBRARY_NAV_LEAVES: &[&str] = &["navigate_next", "navigate_prev"];

/// Backward-compatible name used by map soft-takeover defaults.
pub const ABSOLUTE_ACTIONS: &[&str] = ABSOLUTE_LEAVES;

pub fn is_known_action(name: &str) -> bool {
    let Ok((template, leaf)) = parse_action_id(name) else {
        return false;
    };
    match template {
        OriginTemplate::Deck(_) => DECK_LEAVES.contains(&leaf),
        OriginTemplate::Mixer => MIXER_LEAVES.contains(&leaf),
        OriginTemplate::Engine => ENGINE_LEAVES.contains(&leaf),
        OriginTemplate::LibraryNavigation => LIBRARY_NAV_LEAVES.contains(&leaf),
    }
}

pub fn is_absolute_action(name: &str) -> bool {
    let Ok((_, leaf)) = parse_action_id(name) else {
        // Allow leaf-only checks for internal soft-takeover defaults.
        return ABSOLUTE_LEAVES.contains(&name);
    };
    ABSOLUTE_LEAVES.contains(&leaf)
}

pub fn is_closed_input_alias(section: &str, alias: &str) -> bool {
    if section == "custom" {
        return false;
    }
    if section == "master" {
        return MASTER_ALIASES.contains(&alias);
    }
    if section == "sampler" {
        return SAMPLER_ALIASES.contains(&alias);
    }
    if section.starts_with("deck_") {
        return DECK_ALIASES.contains(&alias);
    }
    false
}

pub fn is_snake_case(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
}
