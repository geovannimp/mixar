//! Host-facing engine bus origin/kind/payload schema.
//!
//! Wire messages use postcard. The `body` field holds a nested postcard payload:
//! - cmd bus: [`CmdBody`]
//! - evt bus: [`EvtBody`]

mod kind;
mod origin;
mod payload;
mod wire;

pub use kind::Kind;
pub use origin::Origin;
pub use payload::{CmdBody, DeckEq, DeckSnapshot, EngineStatus, EvtBody, SyncMode};
pub use wire::{
    decode_cmd_body, decode_evt_body, decode_wire, encode_cmd_body, encode_evt_body, encode_wire,
    DecodeError, EncodeError, WireMessage,
};
