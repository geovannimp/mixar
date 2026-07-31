//! Host-facing engine bus origin/kind/payload schema.
//!
//! Wire messages use MessagePack (`rmp_serde`). The `body` field holds a nested payload:
//! - cmd bus: [`CmdBody`]
//! - evt bus: [`EvtBody`]

mod kind;
mod origin;
mod payload;
mod wire;

pub use kind::Kind;
pub use origin::Origin;
pub use payload::{
    CmdBody, DeckEq, DeckHotCue, DeckSavedLoop, DeckSnapshot, EngineStatus, EvtBody, JogMode,
    LoopRegion, PadMode, SamplerBankInfo, SamplerPlayMode, SamplerSlotInfo, SamplerStatus,
    SyncMode,
};
pub use wire::{
    decode_cmd_body, decode_evt_body, decode_wire, encode_cmd_body, encode_evt_body, encode_wire,
    DecodeError, EncodeError, WireMessage,
};
