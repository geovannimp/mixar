//! Host-facing engine bus origin/kind/payload schema.
//!
//! Wire messages use MessagePack (`rmp_serde`). The `body` field holds a nested payload:
//! - cmd bus: [`CmdBody`]
//! - evt bus: [`EvtBody`]

mod kind;
mod origin;
pub mod pads;
mod payload;
mod wire;

/// Product display name shared by desktop hosts (Tauri / Flutter).
pub const APP_DISPLAY_NAME: &str = "Mixar";

pub use kind::Kind;
pub use origin::Origin;
pub use pads::{
    key_shift_page_action, key_shift_page_next, key_shift_page_prev, keyboard_page_action,
    keyboard_page_next, keyboard_page_prev, PitchPadAction, DEFAULT_PITCH_PAGE,
    KEYBOARD_PAGE_COUNT, KEY_SHIFT_PAGE_COUNT,
};
pub use payload::{
    default_pitch_page, CmdBody, DeckEq, DeckHotCue, DeckSavedLoop, DeckSnapshot, EngineStatus,
    EqBand, EvtBody, JogMode, LoopRegion, PadMode, SamplerBankInfo, SamplerPlayMode,
    SamplerSlotInfo, SamplerStatus, SyncMode,
};
pub use wire::{
    decode_cmd_body, decode_evt_body, decode_wire, encode_cmd_body, encode_evt_body, encode_wire,
    DecodeError, EncodeError, WireMessage, MAX_WIRE_PAYLOAD_BYTES,
};
