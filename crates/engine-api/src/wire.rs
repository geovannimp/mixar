//! Postcard encode/decode for wire messages and nested cmd/evt bodies.

use crate::{CmdBody, EvtBody, Kind, Origin};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Host-facing bus frame: origin, kind, revision, and nested body bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireMessage {
    pub origin: Origin,
    pub kind: Kind,
    pub revision: u64,
    pub body: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum EncodeError {
    #[error("postcard encode failed: {0}")]
    Postcard(#[from] postcard::Error),
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("postcard decode failed: {0}")]
    Postcard(#[from] postcard::Error),
}

pub fn encode_wire(msg: &WireMessage) -> Result<Vec<u8>, EncodeError> {
    Ok(postcard::to_allocvec(msg)?)
}

pub fn decode_wire(bytes: &[u8]) -> Result<WireMessage, DecodeError> {
    Ok(postcard::from_bytes(bytes)?)
}

pub fn encode_cmd_body(body: &CmdBody) -> Result<Vec<u8>, EncodeError> {
    Ok(postcard::to_allocvec(body)?)
}

pub fn decode_cmd_body(bytes: &[u8]) -> Result<CmdBody, DecodeError> {
    Ok(postcard::from_bytes(bytes)?)
}

pub fn encode_evt_body(body: &EvtBody) -> Result<Vec<u8>, EncodeError> {
    Ok(postcard::to_allocvec(body)?)
}

pub fn decode_evt_body(bytes: &[u8]) -> Result<EvtBody, DecodeError> {
    Ok(postcard::from_bytes(bytes)?)
}
