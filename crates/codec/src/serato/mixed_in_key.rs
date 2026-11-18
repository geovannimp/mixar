//! Parser for Mixed In Key's JSON GEOB frames (`CuePoints`, `Key`).
//!
//! Mixed In Key writes `application/json` GEOB objects whose data is either the
//! JSON text itself or that JSON base64-encoded. Only the fields this reader
//! consumes are deserialized — unknown fields are skipped without building a
//! JSON tree — and at most [`MAX_CUES`] cues are kept, so a hostile or corrupt
//! frame can never amplify its bounded tag bytes into an unbounded allocation.
//! Both members must be JSON *objects*: a positional array is not the documented
//! shape and is rejected instead of being mapped to fields by position.
//!
//! `source`, `algorithm` and anything else stay in the raw frame payload and are
//! never synthesized here.

use std::fmt;

use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::Engine as _;
use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;

use super::SeratoCue;

const SOURCE: &str = "mixedinkey";
/// Cues beyond this slot carry no deck slot and are ignored.
const MAX_CUES: usize = 8;

const BASE64: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// The fields this reader consumes. Every other member of the object is skipped
/// by serde without allocating a value for it.
#[derive(Default)]
struct MikObject {
    source: Option<String>,
    key: Option<String>,
    cues: Option<LimitedCues>,
}

struct MikCue {
    time: Option<f64>,
    name: Option<String>,
}

/// At most [`MAX_CUES`] entries; the rest are skipped without materializing.
struct LimitedCues(Vec<MikCue>);

/// Drive a JSON-object visitor: hand every member to `member`, skip the rest.
///
/// Shared by the object visitors so the "unknown members are skipped without
/// materializing a value" rule has one implementation.
fn visit_members<'de, A, T>(
    mut map: A,
    mut object: T,
    mut member: impl FnMut(&mut T, &str, &mut A) -> Result<bool, A::Error>,
) -> Result<T, A::Error>
where
    A: MapAccess<'de>,
{
    while let Some(key) = map.next_key::<String>()? {
        if !member(&mut object, &key, &mut map)? {
            map.next_value::<IgnoredAny>()?;
        }
    }
    Ok(object)
}

impl<'de> Deserialize<'de> for MikObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ObjectVisitor;

        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = MikObject;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                visit_members(map, MikObject::default(), |object, member, map| {
                    match member {
                        "source" => object.source = map.next_value()?,
                        "key" => object.key = map.next_value()?,
                        "cues" => object.cues = map.next_value()?,
                        _ => return Ok(false),
                    }
                    Ok(true)
                })
            }
        }

        // `deserialize_map` rejects sequences, so `["mixedinkey", …]` is invalid.
        deserializer.deserialize_map(ObjectVisitor)
    }
}

impl<'de> Deserialize<'de> for MikCue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct CueVisitor;

        impl<'de> Visitor<'de> for CueVisitor {
            type Value = MikCue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON cue object")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                visit_members(
                    map,
                    MikCue {
                        time: None,
                        name: None,
                    },
                    |cue, member, map| {
                        match member {
                            "time" => cue.time = map.next_value()?,
                            "name" => cue.name = map.next_value()?,
                            _ => return Ok(false),
                        }
                        Ok(true)
                    },
                )
            }
        }

        deserializer.deserialize_map(CueVisitor)
    }
}

impl<'de> Deserialize<'de> for LimitedCues {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct CuesVisitor;

        impl<'de> Visitor<'de> for CuesVisitor {
            type Value = LimitedCues;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an array of cues")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut cues = Vec::new();
                while cues.len() < MAX_CUES {
                    match seq.next_element::<MikCue>()? {
                        Some(cue) => cues.push(cue),
                        None => return Ok(LimitedCues(cues)),
                    }
                }
                // Beyond the deck's slots the entries are ignored, including
                // their shape: skip them without materializing a value.
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(LimitedCues(cues))
            }
        }

        deserializer.deserialize_seq(CuesVisitor)
    }
}

/// Parse a `CuePoints` object into cue rows in array order.
pub(crate) fn parse_cue_points(data: &[u8]) -> Result<Vec<SeratoCue>, ()> {
    let object = decode_object(data)?;
    let cues = object.cues.ok_or(())?.0;
    let mut out = Vec::with_capacity(cues.len());
    for (slot, cue) in cues.into_iter().enumerate() {
        let time = cue.time.ok_or(())?;
        if !time.is_finite() || time < 0.0 {
            return Err(());
        }
        let rounded = time.round();
        if rounded > f64::from(u32::MAX) {
            return Err(());
        }
        let label = cue.name.filter(|name| !name.is_empty());
        out.push(SeratoCue {
            slot: slot as u8,
            position_ms: rounded as u32,
            label,
            color_rgb: None,
        });
    }
    Ok(out)
}

/// Parse a `Key` object into its raw key string.
pub(crate) fn parse_key(data: &[u8]) -> Result<String, ()> {
    let object = decode_object(data)?;
    let key = object.key.ok_or(())?;
    if key.trim().is_empty() {
        return Err(());
    }
    Ok(key)
}

fn decode_object(data: &[u8]) -> Result<MikObject, ()> {
    let trimmed = trim_outer(data);
    if trimmed.is_empty() {
        return Err(());
    }
    let json = if trimmed[0] == b'{' {
        String::from_utf8(trimmed.to_vec()).map_err(|_| ())?
    } else {
        let decoded = BASE64.decode(trimmed).map_err(|_| ())?;
        String::from_utf8(decoded).map_err(|_| ())?
    };
    let object: MikObject = serde_json::from_str(&json).map_err(|_| ())?;
    if object.source.as_deref() != Some(SOURCE) {
        return Err(());
    }
    Ok(object)
}

/// Trim trailing NUL padding and surrounding ASCII whitespace.
fn trim_outer(data: &[u8]) -> &[u8] {
    let mut start = 0usize;
    let mut end = data.len();
    while start < end && data[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && (data[end - 1] == 0 || data[end - 1].is_ascii_whitespace()) {
        end -= 1;
    }
    &data[start..end]
}
