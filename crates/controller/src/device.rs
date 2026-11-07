//! `device.toml` types + parse.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::catalog::{is_closed_input_alias, is_snake_case};
use crate::error::LoadError;
use crate::midi::{MatchKey, MidiEndpoint};

pub const SECTION_MASTER: &str = "master";
pub const SECTION_SAMPLER: &str = "sampler";
pub const SECTION_CUSTOM: &str = "custom";

pub type SectionName = String;

/// One stereo pair of output channels (1-based), as Settings → Audio shows them.
pub type OutputChannels = [u16; 2];

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct AudioHints {
    /// Substrings matched (case-insensitively) against OS output device names.
    #[serde(default)]
    pub output_name_contains: Vec<String>,
    /// Channels carrying the master bus on the matched device, e.g. `[1, 2]`.
    #[serde(default)]
    pub master_channels: Option<OutputChannels>,
    /// Channels carrying the headphone / cue bus, e.g. `[3, 4]`.
    #[serde(default)]
    pub headphone_channels: Option<OutputChannels>,
    /// Rate to open the matched device at. `None` follows the device default.
    #[serde(default)]
    pub sample_rate: Option<u32>,
}

impl AudioHints {
    /// True when the bundle maps at least one bus onto the controller's output.
    pub fn has_output_routes(&self) -> bool {
        self.master_channels.is_some() || self.headphone_channels.is_some()
    }

    /// True when the block carries any usable hint worth surfacing.
    pub fn is_empty(&self) -> bool {
        self.output_name_contains.is_empty() && !self.has_output_routes()
    }

    fn validate(&self) -> Result<(), LoadError> {
        for name in &self.output_name_contains {
            if name.trim().is_empty() {
                return Err(LoadError::Validation(
                    "audio.output_name_contains entries must be non-empty".into(),
                ));
            }
        }
        for (field, pair) in [
            ("audio.master_channels", self.master_channels),
            ("audio.headphone_channels", self.headphone_channels),
        ] {
            let Some([left, right]) = pair else {
                continue;
            };
            if left == 0 || right == 0 {
                return Err(LoadError::Validation(format!(
                    "{field} channels are 1-based and must be non-zero"
                )));
            }
            if left == right {
                return Err(LoadError::Validation(format!(
                    "{field} left and right channels must differ"
                )));
            }
        }
        if self.has_output_routes() && self.output_name_contains.is_empty() {
            return Err(LoadError::Validation(
                "audio output channels require output_name_contains".into(),
            ));
        }
        if let (Some(master), Some(headphones)) = (self.master_channels, self.headphone_channels) {
            if master.iter().any(|channel| headphones.contains(channel)) {
                return Err(LoadError::Validation(
                    "audio master_channels and headphone_channels must not share channels".into(),
                ));
            }
        }
        if let Some(rate) = self.sample_rate {
            if !(8_000..=384_000).contains(&rate) {
                return Err(LoadError::Validation(format!(
                    "audio.sample_rate {rate} is outside 8000..=384000"
                )));
            }
        }
        Ok(())
    }
}

/// Reserved `[toml-schema]` metadata (editor aid). Ignored at runtime.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct TomlSchemaRef {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
}

impl TomlSchemaRef {
    pub(crate) fn try_from_value(value: toml::Value) -> Result<Self, toml::de::Error> {
        value.try_into()
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DeviceFile {
    pub schema_version: u32,
    pub id: String,
    pub vendor_name: String,
    pub product_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub usb_vid: Option<u16>,
    #[serde(default)]
    pub usb_pid: Option<u16>,
    #[serde(default)]
    pub midi_name_contains: Vec<String>,
    #[serde(default)]
    pub mapping_version: Option<String>,
    #[serde(default)]
    pub audio: AudioHints,
    /// Editor schema pointer; never used after deserialize.
    /// Populated in [`Self::parse`] after peeling `[toml-schema]` off the table
    /// (avoid serde flatten ordering issues). Not deserialized in-place.
    #[serde(default, skip)]
    pub toml_schema: Option<TomlSchemaRef>,
    /// Section → alias → endpoint. Keys: `deck_1`..`deck_4`, `master`, `sampler`, `custom`.
    #[serde(flatten)]
    pub sections: BTreeMap<String, BTreeMap<String, MidiEndpoint>>,
}

impl DeviceFile {
    /// UI / offer label: `"Vendor Product"` (skips empty parts).
    pub fn display_name(&self) -> String {
        [self.vendor_name.as_str(), self.product_name.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn parse(text: &str, path: &Path) -> Result<Self, LoadError> {
        let mut table: toml::Table = toml::from_str(text).map_err(|source| LoadError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        // Flatten + `[toml-schema]` before scalars confuses toml/serde; peel it off first.
        let toml_schema = table
            .remove("toml-schema")
            .map(TomlSchemaRef::try_from_value)
            .transpose()
            .map_err(|source| LoadError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        let mut device: DeviceFile = table.try_into().map_err(|source| LoadError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        device.toml_schema = toml_schema;
        // Flatten may swallow known top-level keys if mistyped; strip non-section maps.
        device.sections.retain(|k, _| is_section_key(k));
        if device.schema_version != 1 {
            return Err(LoadError::Schema {
                version: device.schema_version,
            });
        }
        device.validate()?;
        Ok(device)
    }

    pub fn validate(&self) -> Result<(), LoadError> {
        if self.id.is_empty() {
            return Err(LoadError::Validation("device.id must be non-empty".into()));
        }
        if self.product_name.is_empty() {
            return Err(LoadError::Validation(
                "device.product_name must be non-empty".into(),
            ));
        }
        self.audio.validate()?;
        let mut input_keys: BTreeMap<MatchKey, String> = BTreeMap::new();
        for (section, aliases) in &self.sections {
            if !is_section_key(section) {
                return Err(LoadError::Validation(format!(
                    "unknown device section `{section}`"
                )));
            }
            if let Some(n) = deck_index(section) {
                if n > 4 {
                    return Err(LoadError::Validation(format!(
                        "deck index {n} out of v1 range 1..=4"
                    )));
                }
            }
            for (alias, ep) in aliases {
                if !is_snake_case(alias) {
                    return Err(LoadError::Validation(format!(
                        "{section}.{alias}: alias must be snake_case"
                    )));
                }
                ep.validate(&format!("{section}.{alias}"))
                    .map_err(LoadError::Validation)?;
                // Closed catalog only for input-eligible endpoints used as map inputs;
                // extra out-only names are allowed on deck/master/sampler.
                if section != SECTION_CUSTOM
                    && ep.direction.allows_input()
                    && !is_closed_input_alias(section, alias)
                {
                    // Extra names OK only if not claimed as inputs later; device allows
                    // any snake_case for out endpoints. Input-eligible extras are OK in
                    // device.toml but cannot be used as [inputs.*] keys (checked in map).
                }
                if ep.direction.allows_input() {
                    let path = format!("{section}.{alias}");
                    for key in ep.match_keys() {
                        if let Some(prev) = input_keys.insert(key, path.clone()) {
                            return Err(LoadError::Validation(format!(
                                "input MIDI clash: `{prev}` and `{path}` share the same type/channel/note|cc"
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn endpoint(&self, section: &str, alias: &str) -> Option<&MidiEndpoint> {
        self.sections.get(section)?.get(alias)
    }

    /// Resolve `section.alias` reference.
    pub fn resolve_ref<'a>(
        &'a self,
        reference: &'a str,
    ) -> Option<(&'a str, &'a str, &'a MidiEndpoint)> {
        let (section, alias) = reference.split_once('.')?;
        let ep = self.endpoint(section, alias)?;
        Some((section, alias, ep))
    }

    pub fn find_input_match(&self, key: MatchKey) -> Option<(&str, &str, &MidiEndpoint)> {
        for (section, aliases) in &self.sections {
            for (alias, ep) in aliases {
                if ep.direction.allows_input() && ep.match_keys().contains(&key) {
                    return Some((section.as_str(), alias.as_str(), ep));
                }
            }
        }
        None
    }
}

pub fn is_section_key(name: &str) -> bool {
    name == SECTION_MASTER
        || name == SECTION_SAMPLER
        || name == SECTION_CUSTOM
        || deck_index(name).is_some()
}

pub fn deck_index(section: &str) -> Option<u16> {
    section
        .strip_prefix("deck_")
        .and_then(|n| n.parse::<u16>().ok())
        .filter(|&n| n >= 1)
}

/// `deck_1` → Origin deck index 0.
pub fn origin_deck_id(section: &str) -> Option<u16> {
    deck_index(section).map(|n| n - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_device(audio: &str) -> Result<DeviceFile, LoadError> {
        let text = format!(
            "schema_version = 1\nid = \"test.audio\"\nvendor_name = \"Test\"\n\
             product_name = \"Audio\"\n{audio}\n\
             [deck_1]\nplay_pause = {{ type = \"note\", channel = 1, note = 0x0B }}\n"
        );
        DeviceFile::parse(&text, Path::new("device.toml"))
    }

    #[test]
    fn parses_output_device_and_channel_routes() {
        let device = parse_device(
            "[audio]\noutput_name_contains = [\"DDJ-400\"]\n\
             master_channels = [1, 2]\nheadphone_channels = [3, 4]\nsample_rate = 48000\n",
        )
        .unwrap();
        assert_eq!(device.audio.output_name_contains, ["DDJ-400"]);
        assert_eq!(device.audio.master_channels, Some([1, 2]));
        assert_eq!(device.audio.headphone_channels, Some([3, 4]));
        assert_eq!(device.audio.sample_rate, Some(48_000));
        assert!(device.audio.has_output_routes());
    }

    #[test]
    fn audio_names_without_routes_remain_valid() {
        // Bundles authored before output routing existed must still load.
        let device = parse_device("[audio]\noutput_name_contains = [\"DDJ-400\"]\n").unwrap();
        assert!(!device.audio.has_output_routes());
        assert!(!device.audio.is_empty());
    }

    #[test]
    fn rejects_zero_based_or_equal_channel_pairs() {
        let zero = parse_device(
            "[audio]\noutput_name_contains = [\"DDJ-400\"]\nmaster_channels = [0, 2]\n",
        );
        assert!(zero.is_err(), "channel numbers are 1-based");
        let equal = parse_device(
            "[audio]\noutput_name_contains = [\"DDJ-400\"]\nmaster_channels = [2, 2]\n",
        );
        assert!(equal.is_err(), "left and right must differ");
    }

    #[test]
    fn rejects_routes_without_a_device_name_hint() {
        let err = parse_device("[audio]\nmaster_channels = [1, 2]\n").unwrap_err();
        assert!(
            err.to_string().contains("output_name_contains"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_overlapping_master_and_headphone_channels() {
        let err = parse_device(
            "[audio]\noutput_name_contains = [\"DDJ-400\"]\n\
             master_channels = [1, 2]\nheadphone_channels = [2, 3]\n",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("must not share channels"),
            "unexpected error: {err}"
        );
    }
}
