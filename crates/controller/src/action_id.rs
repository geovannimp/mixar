//! Qualified map action ids: `OriginTemplate::leaf`.

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

/// Split `Deck(_)::set_volume` → template + leaf.
pub fn parse_action_id(action: &str) -> Result<(OriginTemplate, &str), LoadError> {
    let Some((origin_s, leaf)) = action.split_once("::") else {
        return Err(LoadError::Validation(format!(
            "action `{action}` must be OriginTemplate::leaf"
        )));
    };
    if leaf.is_empty() || origin_s.is_empty() {
        return Err(LoadError::Validation(format!(
            "action `{action}` has empty origin or leaf"
        )));
    }
    if leaf.contains("::") {
        return Err(LoadError::Validation(format!(
            "action `{action}` must contain exactly one `::`"
        )));
    }
    let template = parse_origin_template(origin_s)?;
    Ok((template, leaf))
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
        OriginTemplate::Deck(Some(id)) => Ok(BoundOrigin::Engine(EngineOrigin::Deck(id))),
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
        let (t, leaf) = parse_action_id("Deck(_)::set_volume").unwrap();
        assert_eq!(leaf, "set_volume");
        assert_eq!(
            bind_origin(t, "deck_1").unwrap(),
            BoundOrigin::Engine(Origin::Deck(0))
        );
    }

    #[test]
    fn deck_absolute_ignores_section_index() {
        let (t, _) = parse_action_id("Deck(1)::set_volume").unwrap();
        assert_eq!(
            bind_origin(t, "deck_1").unwrap(),
            BoundOrigin::Engine(Origin::Deck(1))
        );
    }

    #[test]
    fn library_navigation_parses() {
        let (t, leaf) = parse_action_id("LibraryNavigation::navigate_next").unwrap();
        assert_eq!(t, OriginTemplate::LibraryNavigation);
        assert_eq!(leaf, "navigate_next");
        assert_eq!(
            bind_origin(t, "master").unwrap(),
            BoundOrigin::LibraryNavigation
        );
    }
}
