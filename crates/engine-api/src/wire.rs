//! MessagePack encode/decode for wire messages and nested cmd/evt bodies.

use crate::{CmdBody, EvtBody, Kind, Origin};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Host-facing bus frame: origin, kind, revision, and nested body bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireMessage {
    pub origin: Origin,
    pub kind: Kind,
    pub revision: u64,
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
}

pub fn encode_wire(msg: &WireMessage) -> Result<Vec<u8>, EncodeError> {
    Ok(rmp_serde::to_vec_named(msg)?)
}

pub fn decode_wire(bytes: &[u8]) -> Result<WireMessage, DecodeError> {
    Ok(rmp_serde::from_slice(bytes)?)
}

pub fn encode_cmd_body(body: &CmdBody) -> Result<Vec<u8>, EncodeError> {
    Ok(rmp_serde::to_vec_named(body)?)
}

pub fn decode_cmd_body(bytes: &[u8]) -> Result<CmdBody, DecodeError> {
    Ok(rmp_serde::from_slice(bytes)?)
}

pub fn encode_evt_body(body: &EvtBody) -> Result<Vec<u8>, EncodeError> {
    Ok(rmp_serde::to_vec_named(body)?)
}

pub fn decode_evt_body(bytes: &[u8]) -> Result<EvtBody, DecodeError> {
    Ok(rmp_serde::from_slice(bytes)?)
}
