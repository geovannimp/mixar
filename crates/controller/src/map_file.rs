//! `map.toml` types + parse.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::catalog::{is_absolute_action, is_closed_input_alias, is_known_action};
use crate::device::{is_section_key, DeviceFile, TomlSchemaRef, SECTION_CUSTOM};
use crate::error::LoadError;
use crate::midi::MidiEndpoint;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum RawBinding {
    Action(String),
    Table(InputBinding),
    List(Vec<InputBinding>),
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct InputBinding {
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub modifier: Option<String>,
    #[serde(default)]
    pub soft_takeover: Option<bool>,
    #[serde(default)]
    pub script: Option<String>,
}

impl InputBinding {
    pub fn from_action(action: &str) -> Self {
        Self {
            action: Some(action.to_string()),
            modifier: None,
            soft_takeover: None,
            script: None,
        }
    }

    pub fn validate(&self, path: &str) -> Result<(), LoadError> {
        let has_action = self.action.as_ref().is_some_and(|s| !s.is_empty());
        let has_script = self.script.as_ref().is_some_and(|s| !s.is_empty());
        if has_action == has_script {
            return Err(LoadError::Validation(format!(
                "{path}: binding needs exactly one of `action` or `script`"
            )));
        }
        if let Some(action) = &self.action {
            if !is_known_action(action) {
                return Err(LoadError::Validation(format!(
                    "{path}: unknown action `{action}`"
                )));
            }
        }
        if let Some(m) = &self.modifier {
            if !m.starts_with("custom.") {
                return Err(LoadError::Validation(format!(
                    "{path}: modifier must be `custom.*`, got `{m}`"
                )));
            }
        }
        Ok(())
    }

    pub fn soft_takeover_effective(&self) -> bool {
        if let Some(v) = self.soft_takeover {
            return v;
        }
        self.action.as_deref().is_some_and(is_absolute_action)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum OutputTarget {
    Alias(String),
    Inline(MidiEndpoint),
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct OutputBinding {
    pub on: OutputTarget,
    pub off: OutputTarget,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleHooks {
    #[serde(default)]
    pub on_init: Option<String>,
    #[serde(default)]
    pub on_shutdown: Option<String>,
    #[serde(default)]
    pub idle_heartbeat: Option<String>,
}

impl LifecycleHooks {
    pub fn fn_for(&self, event: &str) -> Option<&str> {
        let name = match event {
            "on_init" => self.on_init.as_deref(),
            "on_shutdown" => self.on_shutdown.as_deref(),
            "idle_heartbeat" => self.idle_heartbeat.as_deref(),
            _ => None,
        }?;
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    }

    fn validate(&self) -> Result<(), LoadError> {
        for (key, value) in [
            ("on_init", &self.on_init),
            ("on_shutdown", &self.on_shutdown),
            ("idle_heartbeat", &self.idle_heartbeat),
        ] {
            if let Some(name) = value {
                if name.is_empty() {
                    return Err(LoadError::Validation(format!(
                        "lifecycle.{key}: function name must be non-empty"
                    )));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct MapFile {
    pub schema_version: u32,
    /// Editor schema pointer; never used after deserialize.
    /// Populated in [`Self::parse`] after peeling `[toml-schema]` off the table.
    #[serde(default, skip)]
    pub toml_schema: Option<TomlSchemaRef>,
    #[serde(default)]
    pub lifecycle: LifecycleHooks,
    #[serde(default)]
    pub inputs: BTreeMap<String, BTreeMap<String, RawBinding>>,
    #[serde(default)]
    pub outputs: BTreeMap<String, BTreeMap<String, OutputBinding>>,
}

impl MapFile {
    pub fn parse(text: &str, path: &Path) -> Result<Self, LoadError> {
        let mut table: toml::Table = toml::from_str(text).map_err(|source| LoadError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        let toml_schema = table
            .remove("toml-schema")
            .map(TomlSchemaRef::try_from_value)
            .transpose()
            .map_err(|source| LoadError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        let mut map: MapFile = table.try_into().map_err(|source| LoadError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        map.toml_schema = toml_schema;
        if map.schema_version != 1 {
            return Err(LoadError::Schema {
                version: map.schema_version,
            });
        }
        Ok(map)
    }

    pub fn bindings_for(&self, section: &str, alias: &str) -> Vec<InputBinding> {
        let Some(sec) = self.inputs.get(section) else {
            return Vec::new();
        };
        let Some(raw) = sec.get(alias) else {
            return Vec::new();
        };
        match raw {
            RawBinding::Action(a) => vec![InputBinding::from_action(a)],
            RawBinding::Table(t) => vec![t.clone()],
            RawBinding::List(list) => list.clone(),
        }
    }

    pub fn validate_against(&self, device: &DeviceFile, has_script: bool) -> Result<(), LoadError> {
        self.lifecycle.validate()?;
        if (self.lifecycle.on_init.is_some()
            || self.lifecycle.on_shutdown.is_some()
            || self.lifecycle.idle_heartbeat.is_some())
            && !has_script
        {
            return Err(LoadError::Validation(
                "[lifecycle] requires script.rhai in the mapping bundle".into(),
            ));
        }
        for (section, aliases) in &self.inputs {
            if !is_section_key(section) {
                return Err(LoadError::Validation(format!(
                    "unknown inputs section `{section}`"
                )));
            }
            if section == SECTION_CUSTOM {
                return Err(LoadError::Validation(
                    "custom aliases cannot bind declarative engine actions under [inputs.custom]"
                        .into(),
                ));
            }
            for (alias, raw) in aliases {
                if !is_closed_input_alias(section, alias) {
                    return Err(LoadError::Validation(format!(
                        "inputs.{section}.{alias}: not in closed catalog for `{section}`"
                    )));
                }
                if device.endpoint(section, alias).is_none() {
                    return Err(LoadError::Validation(format!(
                        "inputs.{section}.{alias}: no matching endpoint in device.toml"
                    )));
                }
                let bindings = match raw {
                    RawBinding::Action(a) => vec![InputBinding::from_action(a)],
                    RawBinding::Table(t) => vec![t.clone()],
                    RawBinding::List(list) => list.clone(),
                };
                for (i, b) in bindings.iter().enumerate() {
                    let path = format!("inputs.{section}.{alias}[{i}]");
                    b.validate(&path)?;
                    if let Some(m) = &b.modifier {
                        if device.resolve_ref(m).is_none() {
                            return Err(LoadError::Validation(format!(
                                "{path}: modifier `{m}` missing from device.toml"
                            )));
                        }
                    }
                    if let Some(script) = &b.script {
                        if !has_script {
                            return Err(LoadError::MissingScript(script.clone()));
                        }
                    }
                }
            }
        }

        for (section, aliases) in &self.outputs {
            if !is_section_key(section) {
                return Err(LoadError::Validation(format!(
                    "unknown outputs section `{section}`"
                )));
            }
            for (alias, out) in aliases {
                // Watch key must be closed catalog (signal) for the section.
                if section != SECTION_CUSTOM && !is_closed_input_alias(section, alias) {
                    return Err(LoadError::Validation(format!(
                        "outputs.{section}.{alias}: watch key not in closed catalog"
                    )));
                }
                validate_output_target(device, &out.on, &format!("outputs.{section}.{alias}.on"))?;
                validate_output_target(
                    device,
                    &out.off,
                    &format!("outputs.{section}.{alias}.off"),
                )?;
            }
        }
        Ok(())
    }
}

fn validate_output_target(
    device: &DeviceFile,
    target: &OutputTarget,
    path: &str,
) -> Result<(), LoadError> {
    match target {
        OutputTarget::Alias(name) => {
            // Prefer section-local bare name, else section.alias form.
            let resolved = if name.contains('.') {
                device.resolve_ref(name).map(|_| ())
            } else {
                // Search all sections for bare alias.
                device
                    .sections
                    .values()
                    .any(|m| m.contains_key(name))
                    .then_some(())
            };
            if resolved.is_none() {
                return Err(LoadError::Validation(format!(
                    "{path}: alias `{name}` missing from device.toml"
                )));
            }
        }
        OutputTarget::Inline(ep) => {
            ep.validate(path).map_err(LoadError::Validation)?;
        }
    }
    Ok(())
}

/// Marker for docs / tests around soft-takeover defaults.
pub struct SoftTakeoverDefault;
