//! Short MIDI message parse (note / CC / CC14).

use serde::{Deserialize, Serialize};

/// How a MIDI endpoint may be used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    In,
    Out,
    #[default]
    Inout,
}

impl Direction {
    pub fn allows_input(self) -> bool {
        matches!(self, Self::In | Self::Inout)
    }

    pub fn allows_output(self) -> bool {
        matches!(self, Self::Out | Self::Inout)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MidiMsgType {
    Note,
    Cc,
    Cc14,
}

/// 7-bit CC number or 14-bit MSB/LSB pair under `cc = …`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CcField {
    SevenBit(u8),
    FourteenBit { msb: u8, lsb: u8 },
}

/// Device alias MIDI endpoint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidiEndpoint {
    #[serde(rename = "type")]
    pub msg_type: MidiMsgType,
    pub channel: u8,
    #[serde(default)]
    pub note: Option<u8>,
    #[serde(default)]
    pub cc: Option<CcField>,
    #[serde(default)]
    pub velocity: Option<u8>,
    #[serde(default)]
    pub value: Option<u8>,
    #[serde(default)]
    pub direction: Direction,
}

impl MidiEndpoint {
    pub fn validate(&self, path: &str) -> Result<(), String> {
        if !(1..=16).contains(&self.channel) {
            return Err(format!(
                "{path}: channel must be 1..=16, got {}",
                self.channel
            ));
        }
        match self.msg_type {
            MidiMsgType::Note => {
                if self.note.is_none() {
                    return Err(format!("{path}: note message requires `note`"));
                }
            }
            MidiMsgType::Cc => match &self.cc {
                Some(CcField::SevenBit(_)) => {}
                Some(CcField::FourteenBit { .. }) => {
                    return Err(format!(
                        "{path}: type=cc expects `cc = <u8>`; use type=cc14 for msb/lsb"
                    ));
                }
                None => return Err(format!("{path}: cc message requires `cc`")),
            },
            MidiMsgType::Cc14 => match &self.cc {
                Some(CcField::FourteenBit { msb, lsb }) => {
                    if msb == lsb {
                        return Err(format!("{path}: cc14 msb and lsb must differ"));
                    }
                }
                Some(CcField::SevenBit(_)) => {
                    return Err(format!(
                        "{path}: type=cc14 expects `cc = {{ msb = …, lsb = … }}`"
                    ));
                }
                None => {
                    return Err(format!(
                        "{path}: cc14 requires `cc = {{ msb = …, lsb = … }}`"
                    ));
                }
            },
        }
        Ok(())
    }

    pub fn cc14_pair(&self) -> Option<(u8, u8)> {
        match (&self.msg_type, &self.cc) {
            (MidiMsgType::Cc14, Some(CcField::FourteenBit { msb, lsb })) => Some((*msb, *lsb)),
            _ => None,
        }
    }

    pub fn is_cc14(&self) -> bool {
        self.msg_type == MidiMsgType::Cc14
    }

    /// Encode as 3-byte short MIDI (status + data1 + data2).
    /// For `cc14`, sends the MSB CC (outputs are 7-bit on/off style).
    pub fn to_bytes(&self, data2_override: Option<u8>) -> [u8; 3] {
        let ch = self.channel.saturating_sub(1) & 0x0F;
        match self.msg_type {
            MidiMsgType::Note => {
                let note = self.note.unwrap_or(0);
                let vel = data2_override.or(self.velocity).unwrap_or(0x7F);
                [0x90 | ch, note & 0x7F, vel & 0x7F]
            }
            MidiMsgType::Cc => {
                let cc = match &self.cc {
                    Some(CcField::SevenBit(n)) => *n,
                    _ => 0,
                };
                let val = data2_override.or(self.value).unwrap_or(0x7F);
                [0xB0 | ch, cc & 0x7F, val & 0x7F]
            }
            MidiMsgType::Cc14 => {
                let msb = match &self.cc {
                    Some(CcField::FourteenBit { msb, .. }) => *msb,
                    _ => 0,
                };
                let val = data2_override.or(self.value).unwrap_or(0x7F);
                [0xB0 | ch, msb & 0x7F, val & 0x7F]
            }
        }
    }

    /// All MIDI identities this endpoint claims (one for note/cc, two for cc14).
    pub fn match_keys(&self) -> Vec<MatchKey> {
        match self.msg_type {
            MidiMsgType::Note => vec![MatchKey::Note {
                channel: self.channel,
                note: self.note.unwrap_or(0),
            }],
            MidiMsgType::Cc => {
                let cc = match &self.cc {
                    Some(CcField::SevenBit(n)) => *n,
                    _ => 0,
                };
                vec![MatchKey::Cc {
                    channel: self.channel,
                    cc,
                }]
            }
            MidiMsgType::Cc14 => match &self.cc {
                Some(CcField::FourteenBit { msb, lsb }) => vec![
                    MatchKey::Cc {
                        channel: self.channel,
                        cc: *msb,
                    },
                    MatchKey::Cc {
                        channel: self.channel,
                        cc: *lsb,
                    },
                ],
                _ => Vec::new(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MatchKey {
    Note { channel: u8, note: u8 },
    Cc { channel: u8, cc: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortMsg {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8, velocity: u8 },
    Cc { channel: u8, cc: u8, value: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParsedMidi {
    pub msg: ShortMsg,
    pub match_key: MatchKey,
    /// Normalized 0..1 from velocity/value (note-off → 0). 7-bit CC only;
    /// session replaces this for cc14 after pairing.
    pub norm: f32,
    /// True for note-on with vel>0 or CC; false for note-off / note-on vel0.
    pub active: bool,
}

/// Parse a short MIDI message (2–3 bytes). Returns None for unsupported/sysex/etc.
pub fn parse_short(bytes: &[u8]) -> Option<ParsedMidi> {
    if bytes.len() < 2 {
        return None;
    }
    let status = bytes[0];
    let kind = status & 0xF0;
    let channel = (status & 0x0F) + 1; // 1..=16
    match kind {
        0x80 => {
            if bytes.len() < 3 {
                return None;
            }
            let note = bytes[1] & 0x7F;
            let velocity = bytes[2] & 0x7F;
            Some(ParsedMidi {
                msg: ShortMsg::NoteOff {
                    channel,
                    note,
                    velocity,
                },
                match_key: MatchKey::Note { channel, note },
                norm: 0.0,
                active: false,
            })
        }
        0x90 => {
            if bytes.len() < 3 {
                return None;
            }
            let note = bytes[1] & 0x7F;
            let velocity = bytes[2] & 0x7F;
            if velocity == 0 {
                Some(ParsedMidi {
                    msg: ShortMsg::NoteOff {
                        channel,
                        note,
                        velocity: 0,
                    },
                    match_key: MatchKey::Note { channel, note },
                    norm: 0.0,
                    active: false,
                })
            } else {
                Some(ParsedMidi {
                    msg: ShortMsg::NoteOn {
                        channel,
                        note,
                        velocity,
                    },
                    match_key: MatchKey::Note { channel, note },
                    norm: velocity as f32 / 127.0,
                    active: true,
                })
            }
        }
        0xB0 => {
            if bytes.len() < 3 {
                return None;
            }
            let cc = bytes[1] & 0x7F;
            let value = bytes[2] & 0x7F;
            Some(ParsedMidi {
                msg: ShortMsg::Cc { channel, cc, value },
                match_key: MatchKey::Cc { channel, cc },
                norm: value as f32 / 127.0,
                active: value > 0,
            })
        }
        _ => None,
    }
}

/// Combine 14-bit MIDI into 0..1.
pub fn norm_from_cc14(msb: u8, lsb: u8) -> f32 {
    let v = (u16::from(msb & 0x7F) << 7) | u16::from(lsb & 0x7F);
    v as f32 / 16383.0
}

/// USB / name identity for autoload matching.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MidiIdentity {
    pub usb_vid: Option<u16>,
    pub usb_pid: Option<u16>,
    pub port_name: String,
}

/// Returns true if this identity matches the device file fields.
pub fn match_device(
    identity: &MidiIdentity,
    usb_vid: Option<u16>,
    usb_pid: Option<u16>,
    name_contains: &[String],
) -> bool {
    let usb_ok = match (usb_vid, usb_pid, identity.usb_vid, identity.usb_pid) {
        (Some(v), Some(p), Some(iv), Some(ip)) => v == iv && p == ip,
        (Some(v), None, Some(iv), _) => v == iv,
        (None, Some(p), _, Some(ip)) => p == ip,
        (Some(_), Some(_), None, _) | (Some(_), Some(_), _, None) => false,
        _ => false,
    };
    let name_ok = !name_contains.is_empty()
        && name_contains.iter().any(|frag| {
            identity
                .port_name
                .to_ascii_lowercase()
                .contains(&frag.to_ascii_lowercase())
        });
    usb_ok || name_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_note_on_and_cc() {
        let n = parse_short(&[0x90, 0x0B, 0x7F]).unwrap();
        assert!(n.active);
        assert_eq!(
            n.match_key,
            MatchKey::Note {
                channel: 1,
                note: 0x0B
            }
        );

        let c = parse_short(&[0xB0, 0x13, 64]).unwrap();
        assert!((c.norm - 64.0 / 127.0).abs() < 1e-6);
    }

    #[test]
    fn note_on_vel0_is_off() {
        let n = parse_short(&[0x90, 0x0B, 0x00]).unwrap();
        assert!(!n.active);
    }

    #[test]
    fn cc14_endpoint_parses_and_lists_both_keys() {
        let toml = r#"
            type = "cc14"
            channel = 1
            cc = { msb = 0x13, lsb = 0x33 }
        "#;
        let ep: MidiEndpoint = toml::from_str(toml).unwrap();
        ep.validate("volume").unwrap();
        let keys = ep.match_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&MatchKey::Cc {
            channel: 1,
            cc: 0x13
        }));
        assert!(keys.contains(&MatchKey::Cc {
            channel: 1,
            cc: 0x33
        }));
        assert!((norm_from_cc14(0x40, 0x00) - (0x40u16 << 7) as f32 / 16383.0).abs() < 1e-6);
    }
}
