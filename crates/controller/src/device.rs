//! `device.toml` types + parse.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::catalog::{is_closed_input_alias, is_snake_case};
use crate::error::LoadError;
use crate::midi::{MatchKey, MidiEndpoint};

pub const SECTION_MASTER: &str = "master";
pub const SECTION_SAMPLER: &str = "sampler";
pub const SECTION_CUSTOM: &str = "custom";

pub type SectionName = String;

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct AudioHints {
    #[serde(default)]
    pub output_name_contains: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DeviceFile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub usb_vid: Option<u16>,
    #[serde(default)]
    pub usb_pid: Option<u16>,
    #[serde(default)]
    pub midi_name_contains: Vec<String>,
    #[serde(default)]
    pub audio: AudioHints,
    /// Section → alias → endpoint. Keys: `deck_1`..`deck_4`, `master`, `sampler`, `custom`.
    #[serde(flatten)]
    pub sections: BTreeMap<String, BTreeMap<String, MidiEndpoint>>,
}

impl DeviceFile {
    pub fn parse(text: &str, path: &Path) -> Result<Self, LoadError> {
        let mut device: DeviceFile = toml::from_str(text).map_err(|source| LoadError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
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
                    let key = ep.match_key();
                    let path = format!("{section}.{alias}");
                    if let Some(prev) = input_keys.insert(key, path.clone()) {
                        return Err(LoadError::Validation(format!(
                            "input MIDI clash: `{prev}` and `{path}` share the same type/channel/note|cc"
                        )));
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
                if ep.direction.allows_input() && ep.match_key() == key {
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
