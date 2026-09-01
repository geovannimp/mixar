//! MessagePack encode/decode for wire messages and nested cmd/evt bodies.

use crate::{CmdBody, EvtBody, Kind, Origin};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum allowed MessagePack payload size before decode.
pub const MAX_WIRE_PAYLOAD_BYTES: usize = 256 * 1024;

/// Host-facing bus frame: origin, kind, revision, optional client timestamp, nested body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireMessage {
    pub origin: Origin,
    pub kind: Kind,
    pub revision: u64,
    /// Wall-clock ms when the client action was taken (`Date.now()`); 0 for engine-originated evt.
    #[serde(default)]
    pub action_timestamp_ms: u64,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum EncodeError {
    #[error("messagepack encode failed: {0}")]
    Msgpack(#[from] rmp_serde::encode::Error),
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("messagepack decode failed: {0}")]
    Msgpack(#[from] rmp_serde::decode::Error),
    #[error("payload too large: {len} bytes (max {max})")]
    PayloadTooLarge { len: usize, max: usize },
}

fn ensure_payload_size(len: usize) -> Result<(), DecodeError> {
    if len > MAX_WIRE_PAYLOAD_BYTES {
        return Err(DecodeError::PayloadTooLarge {
            len,
            max: MAX_WIRE_PAYLOAD_BYTES,
        });
    }
    Ok(())
}

pub fn encode_wire(msg: &WireMessage) -> Result<Vec<u8>, EncodeError> {
    Ok(rmp_serde::to_vec_named(msg)?)
}

pub fn decode_wire(bytes: &[u8]) -> Result<WireMessage, DecodeError> {
    ensure_payload_size(bytes.len())?;
    let msg: WireMessage = rmp_serde::from_slice(bytes)?;
    ensure_payload_size(msg.body.len())?;
    Ok(msg)
}

pub fn encode_cmd_body(body: &CmdBody) -> Result<Vec<u8>, EncodeError> {
    Ok(rmp_serde::to_vec_named(body)?)
}

pub fn decode_cmd_body(bytes: &[u8]) -> Result<CmdBody, DecodeError> {
    ensure_payload_size(bytes.len())?;
    Ok(rmp_serde::from_slice(bytes)?)
}

pub fn encode_evt_body(body: &EvtBody) -> Result<Vec<u8>, EncodeError> {
    Ok(rmp_serde::to_vec_named(body)?)
}

pub fn decode_evt_body(bytes: &[u8]) -> Result<EvtBody, DecodeError> {
    ensure_payload_size(bytes.len())?;
    Ok(rmp_serde::from_slice(bytes)?)
}
