//! Qualified map action ids: `OriginTemplate::leaf` or `OriginTemplate::leaf(key:value,…)`.

use std::collections::BTreeMap;

use engine_api::Origin as EngineOrigin;

use crate::device::origin_deck_id;
use crate::error::LoadError;

/// Origin half of a map action before section binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OriginTemplate {
    /// `None` means `Deck(_)` — inherit from section.
    Deck(Option<u16>),
    Mixer,
    Engine,
    LibraryNavigation,
}

/// Concrete origin after binding `Deck(_)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundOrigin {
    Engine(EngineOrigin),
    LibraryNavigation,
}

/// Named leaf argument value.
#[derive(Clone, Debug, PartialEq)]
pub enum ArgValue {
    Int(i64),
    Float(f32),
    Ident(String),
}

/// Named args from `leaf(key:value,…)`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActionArgs {
    map: BTreeMap<String, ArgValue>,
}

impl ActionArgs {
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&ArgValue> {
        self.map.get(key)
    }

    pub fn require_int(&self, key: &str) -> Result<i64, LoadError> {
        match self.map.get(key) {
            Some(ArgValue::Int(v)) => Ok(*v),
            Some(_) => Err(LoadError::Validation(format!(
                "arg `{key}` must be an integer"
            ))),
            None => Err(LoadError::Validation(format!(
                "missing required arg `{key}`"
            ))),
        }
    }

    pub fn require_f32(&self, key: &str) -> Result<f32, LoadError> {
        match self.map.get(key) {
            Some(ArgValue::Float(v)) => Ok(*v),
            Some(ArgValue::Int(v)) => Ok(*v as f32),
            Some(_) => Err(LoadError::Validation(format!(
                "arg `{key}` must be a number"
            ))),
            None => Err(LoadError::Validation(format!(
                "missing required arg `{key}`"
            ))),
        }
    }

    pub fn require_ident(&self, key: &str) -> Result<&str, LoadError> {
        match self.map.get(key) {
            Some(ArgValue::Ident(v)) => Ok(v.as_str()),
            Some(_) => Err(LoadError::Validation(format!(
                "arg `{key}` must be an identifier"
            ))),
            None => Err(LoadError::Validation(format!(
                "missing required arg `{key}`"
            ))),
        }
    }

    pub fn expect_keys_exactly(&self, keys: &[&str]) -> Result<(), LoadError> {
        if self.map.len() != keys.len() || keys.iter().any(|k| !self.map.contains_key(*k)) {
            return Err(LoadError::Validation(format!(
                "expected args [{}], got [{}]",
                keys.join(", "),
                self.map.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        Ok(())
    }

    pub fn expect_empty(&self) -> Result<(), LoadError> {
        if self.map.is_empty() {
            Ok(())
        } else {
            Err(LoadError::Validation(
                "this leaf does not take arguments".into(),
            ))
        }
    }
}

/// Split `Deck(_)::pad(n:3)` → template + leaf + args.
pub fn parse_action_id(action: &str) -> Result<(OriginTemplate, &str, ActionArgs), LoadError> {
    let Some((origin_s, leaf_part)) = action.split_once("::") else {
        return Err(LoadError::Validation(format!(
            "action `{action}` must be OriginTemplate::leaf"
        )));
    };
    if leaf_part.is_empty() || origin_s.is_empty() {
        return Err(LoadError::Validation(format!(
            "action `{action}` has empty origin or leaf"
        )));
    }
    if leaf_part.contains("::") {
        return Err(LoadError::Validation(format!(
            "action `{action}` must contain exactly one `::`"
        )));
    }
    let (leaf, args) = parse_leaf_and_args(leaf_part, action)?;
    let template = parse_origin_template(origin_s)?;
    Ok((template, leaf, args))
}

fn parse_leaf_and_args<'a>(
    leaf_part: &'a str,
    action: &str,
) -> Result<(&'a str, ActionArgs), LoadError> {
    let Some(open) = leaf_part.find('(') else {
        return Ok((leaf_part, ActionArgs::default()));
    };
    if !leaf_part.ends_with(')') {
        return Err(LoadError::Validation(format!(
            "action `{action}`: unclosed argument list"
        )));
    }
    let leaf = &leaf_part[..open];
    if leaf.is_empty() {
        return Err(LoadError::Validation(format!(
            "action `{action}` has empty leaf name"
        )));
    }
    let inner = &leaf_part[open + 1..leaf_part.len() - 1];
    if inner.trim().is_empty() {
        return Err(LoadError::Validation(format!(
            "action `{action}`: empty `()` not allowed; omit parentheses for no-arg leaves"
        )));
    }
    let args = parse_arg_list(inner, action)?;
    Ok((leaf, args))
}

fn parse_arg_list(inner: &str, action: &str) -> Result<ActionArgs, LoadError> {
    let mut map = BTreeMap::new();
    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(LoadError::Validation(format!(
                "action `{action}`: empty argument entry"
            )));
        }
        let Some((key, value)) = part.split_once(':') else {
            return Err(LoadError::Validation(format!(
                "action `{action}`: arg `{part}` must be key:value"
            )));
        };
        let key = key.trim();
        let value = value.trim();
        if !is_ident(key) {
            return Err(LoadError::Validation(format!(
                "action `{action}`: invalid arg key `{key}`"
            )));
        }
        if map.contains_key(key) {
            return Err(LoadError::Validation(format!(
                "action `{action}`: duplicate arg `{key}`"
            )));
        }
        let parsed = parse_arg_value(value).ok_or_else(|| {
            LoadError::Validation(format!("action `{action}`: invalid arg value `{value}`"))
        })?;
        map.insert(key.to_string(), parsed);
    }
    Ok(ActionArgs { map })
}

fn parse_arg_value(value: &str) -> Option<ArgValue> {
    if let Ok(n) = value.parse::<i64>() {
        return Some(ArgValue::Int(n));
    }
    if let Ok(n) = value.parse::<f32>() {
        if n.is_finite() {
            return Some(ArgValue::Float(n));
        }
        return None;
    }
    if is_ident(value) {
        return Some(ArgValue::Ident(value.to_string()));
    }
    None
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn parse_origin_template(s: &str) -> Result<OriginTemplate, LoadError> {
    match s {
        "Mixer" => Ok(OriginTemplate::Mixer),
        "Engine" => Ok(OriginTemplate::Engine),
        "LibraryNavigation" => Ok(OriginTemplate::LibraryNavigation),
        other => {
            let Some(rest) = other
                .strip_prefix("Deck(")
                .and_then(|r| r.strip_suffix(')'))
            else {
                return Err(LoadError::Validation(format!(
                    "unknown origin template `{s}`"
                )));
            };
            if rest == "_" {
                return Ok(OriginTemplate::Deck(None));
            }
            let id: u16 = rest.parse().map_err(|_| {
                LoadError::Validation(format!("invalid deck index in origin template `{s}`"))
            })?;
            if id > 3 {
                return Err(LoadError::Validation(format!(
                    "deck index `{id}` out of range (0..=3) in origin template `{s}`"
                )));
            }
            Ok(OriginTemplate::Deck(Some(id)))
        }
    }
}

/// Bind `Deck(_)` from map section (`deck_1` → deck 0).
pub fn bind_origin(template: OriginTemplate, section: &str) -> Result<BoundOrigin, LoadError> {
    match template {
        OriginTemplate::Mixer => Ok(BoundOrigin::Engine(EngineOrigin::Mixer)),
        OriginTemplate::Engine => Ok(BoundOrigin::Engine(EngineOrigin::Engine)),
        OriginTemplate::LibraryNavigation => Ok(BoundOrigin::LibraryNavigation),
        OriginTemplate::Deck(Some(id)) => {
            if id > 3 {
                return Err(LoadError::Validation(format!(
                    "deck index `{id}` out of range (0..=3)"
                )));
            }
            Ok(BoundOrigin::Engine(EngineOrigin::Deck(id)))
        }
        OriginTemplate::Deck(None) => {
            let Some(id) = origin_deck_id(section) else {
                return Err(LoadError::Validation(format!(
                    "Deck(_) requires a deck_* section, got `{section}`"
                )));
            };
            Ok(BoundOrigin::Engine(EngineOrigin::Deck(id)))
        }
    }
}

/// Format bound origin + leaf for soft-takeover / absolute catalogs.
pub fn format_bound_action(bound: &BoundOrigin, leaf: &str) -> String {
    match bound {
        BoundOrigin::Engine(EngineOrigin::Deck(id)) => format!("Deck({id})::{leaf}"),
        BoundOrigin::Engine(EngineOrigin::Mixer) => format!("Mixer::{leaf}"),
        BoundOrigin::Engine(EngineOrigin::Engine) => format!("Engine::{leaf}"),
        BoundOrigin::LibraryNavigation => format!("LibraryNavigation::{leaf}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_api::Origin;

    #[test]
    fn deck_wildcard_binds_from_section() {
        let (t, leaf, args) = parse_action_id("Deck(_)::set_volume").unwrap();
        assert_eq!(leaf, "set_volume");
        assert!(args.is_empty());
        assert_eq!(
            bind_origin(t, "deck_1").unwrap(),
            BoundOrigin::Engine(Origin::Deck(0))
        );
    }

    #[test]
    fn named_args_parse() {
        let (_, leaf, args) = parse_action_id("Deck(_)::pad(n:3)").unwrap();
        assert_eq!(leaf, "pad");
        assert_eq!(args.require_int("n").unwrap(), 3);
    }

    #[test]
    fn signed_beats_parse() {
        let (_, leaf, args) = parse_action_id("Deck(_)::beat_jump(beats:-2)").unwrap();
        assert_eq!(leaf, "beat_jump");
        assert_eq!(args.require_int("beats").unwrap(), -2);
        assert_eq!(args.require_f32("beats").unwrap(), -2.0);
    }

    #[test]
    fn decimal_beats_parse() {
        let (_, leaf, args) = parse_action_id("Deck(_)::auto_loop(beats:0.25)").unwrap();
        assert_eq!(leaf, "auto_loop");
        assert!((args.require_f32("beats").unwrap() - 0.25).abs() < 1e-6);
        assert!(args.require_int("beats").is_err());
    }

    #[test]
    fn empty_parens_rejected() {
        assert!(parse_action_id("Deck(_)::pad()").is_err());
    }

    #[test]
    fn library_navigation_parses() {
        let (t, leaf, args) = parse_action_id("LibraryNavigation::navigate_next").unwrap();
        assert_eq!(t, OriginTemplate::LibraryNavigation);
        assert_eq!(leaf, "navigate_next");
        assert!(args.is_empty());
        assert_eq!(
            bind_origin(t, "master").unwrap(),
            BoundOrigin::LibraryNavigation
        );
    }

    #[test]
    fn deck_index_out_of_range_rejected() {
        assert!(parse_action_id("Deck(4)::set_volume").is_err());
    }

    #[test]
    fn old_suffix_leaf_still_parses_but_is_unknown_action() {
        let (t, leaf, args) = parse_action_id("Deck(_)::pad_1").unwrap();
        assert_eq!(leaf, "pad_1");
        assert!(args.is_empty());
        assert_eq!(t, OriginTemplate::Deck(None));
        assert!(!crate::catalog::is_known_action("Deck(_)::pad_1"));
        assert!(crate::catalog::is_known_action("Deck(_)::pad(n:1)"));
    }
}
