use crate::{presets, CurveError};

/// Maximum length of a curve group or curve set id.
pub const MAX_ID_LEN: usize = 64;

/// What a [`CurveGroupRef`] points at.
#[derive(Clone, Debug, PartialEq)]
pub enum CurveRefKind {
    /// A built-in shape preset id, optionally with one numeric parameter
    /// (for example an exponential rate).
    Preset { id: String, param: Option<f32> },
    /// A named custom group in a [`CurveLibrary`].
    ///
    /// [`CurveLibrary`]: crate::CurveLibrary
    Custom { id: String },
}

/// A stable reference to a curve group.
///
/// Ids are canonical snake_case so they can be used verbatim by controller
/// mappings and settings. A reference resolves to a custom group through
/// [`CurveLibrary`](crate::CurveLibrary); presets are resolved by the layer that
/// owns the preset registry.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveGroupRef {
    pub kind: CurveRefKind,
}

impl CurveGroupRef {
    /// Reference a built-in preset without a parameter.
    pub fn preset(id: impl Into<String>) -> Self {
        Self {
            kind: CurveRefKind::Preset {
                id: id.into(),
                param: None,
            },
        }
    }

    /// Reference a built-in preset with one numeric parameter.
    pub fn preset_with_param(id: impl Into<String>, param: f32) -> Self {
        Self {
            kind: CurveRefKind::Preset {
                id: id.into(),
                param: Some(param),
            },
        }
    }

    /// Reference a named custom group.
    pub fn custom(id: impl Into<String>) -> Self {
        Self {
            kind: CurveRefKind::Custom { id: id.into() },
        }
    }

    /// The referenced id.
    pub fn id(&self) -> &str {
        match &self.kind {
            CurveRefKind::Preset { id, .. } | CurveRefKind::Custom { id } => id,
        }
    }

    /// Whether this reference points at a preset rather than a custom group.
    pub fn is_preset(&self) -> bool {
        matches!(self.kind, CurveRefKind::Preset { .. })
    }

    /// Validate the id and preset parameter.
    ///
    /// Custom ids may not shadow a built-in preset id. Preset ids are not
    /// resolved here; that belongs to the preset registry.
    pub fn validate(&self) -> Result<(), CurveError> {
        validate_id(self.id())?;
        match &self.kind {
            CurveRefKind::Preset { id, param } => {
                if let Some(param) = param {
                    if !param.is_finite() {
                        return Err(CurveError::NonFiniteParameter {
                            id: id.clone(),
                            param: *param,
                        });
                    }
                }
                Ok(())
            }
            CurveRefKind::Custom { id } => {
                if presets::is_preset_id(id) {
                    return Err(CurveError::ReservedId { id: id.clone() });
                }
                Ok(())
            }
        }
    }
}

pub(crate) fn validate_id(id: &str) -> Result<(), CurveError> {
    let invalid = || CurveError::InvalidId { id: id.to_string() };
    if id.is_empty() || id.len() > MAX_ID_LEN {
        return Err(invalid());
    }
    let mut chars = id.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return Err(invalid()),
    }
    if !chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_') {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_snake_case_ids() {
        for id in ["a", "linear", "curve_1", "crossfader_a"] {
            assert!(validate_id(id).is_ok(), "{id} should be valid");
        }
    }

    #[test]
    fn rejects_malformed_ids() {
        for id in ["", "1curve", "Curve", "curve-1", "curve 1", "curve.a"] {
            assert!(
                matches!(validate_id(id), Err(CurveError::InvalidId { .. })),
                "{id:?} should be rejected"
            );
        }
        let long = "a".repeat(MAX_ID_LEN + 1);
        assert!(matches!(
            validate_id(&long),
            Err(CurveError::InvalidId { .. })
        ));
    }

    #[test]
    fn custom_refs_cannot_shadow_presets() {
        let reference = CurveGroupRef::custom("linear");
        assert!(matches!(
            reference.validate(),
            Err(CurveError::ReservedId { .. })
        ));
        assert_eq!(CurveGroupRef::preset("linear").validate(), Ok(()));
        assert_eq!(CurveGroupRef::custom("my_curve").validate(), Ok(()));
    }

    #[test]
    fn preset_parameters_must_be_finite() {
        assert!(matches!(
            CurveGroupRef::preset_with_param("exponential", f32::NAN).validate(),
            Err(CurveError::NonFiniteParameter { .. })
        ));
        assert_eq!(
            CurveGroupRef::preset_with_param("exponential", 4.0).validate(),
            Ok(())
        );
    }

    #[test]
    fn exposes_the_referenced_id() {
        let preset = CurveGroupRef::preset("exponential");
        assert_eq!(preset.id(), "exponential");
        assert!(preset.is_preset());
        assert!(!CurveGroupRef::custom("mine").is_preset());
    }
}
