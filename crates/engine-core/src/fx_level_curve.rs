//! FX level/depth curve resolution, selection, and installation.
//!
//! The DSP FX chain stores one compiled [`ResolvedCurveGroup`] bound to a
//! normalized `0..1` control position, and evaluates it when the wet/dry mix is
//! read or applied. This module is the control-thread layer that turns a
//! settings/controller id into that compiled table. It owns no curve math: the
//! shapes come from `curve-core`, and custom groups come from the shared curve
//! library.
//!
//! Resolution order for a selection id:
//!
//! 1. the `linear` identity default,
//! 2. a named one-channel custom group in the library.
//!
//! A multi-channel group is rejected for this scalar target. Unknown ids fall
//! back to `linear` so playback never fails to start or select.

use anyhow::Result;
use bus_api::DEFAULT_FX_LEVEL_CURVE_ID;
use curve_core::{CurveLibrary, ResolvedCurveGroup};

/// An FX level control drives exactly one curve output.
pub const FX_LEVEL_CURVE_CHANNELS: usize = engine_dsp::FX_LEVEL_CHANNELS;

/// The active FX level curve selection as the control thread remembers it.
#[derive(Clone, Debug, PartialEq)]
pub struct FxLevelCurveSelection {
    /// Canonical id of the active `linear` default or custom group.
    pub id: String,
    /// Human-readable name for settings and logs.
    pub display_name: String,
}

impl Default for FxLevelCurveSelection {
    fn default() -> Self {
        Self::linear()
    }
}

impl FxLevelCurveSelection {
    /// The identity selection used when none is configured.
    pub fn linear() -> Self {
        Self {
            id: DEFAULT_FX_LEVEL_CURVE_ID.to_string(),
            display_name: "Linear".to_string(),
        }
    }
}

/// A compiled FX level curve plus the selection it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedFxLevelCurve {
    pub curve: ResolvedCurveGroup,
    pub selection: FxLevelCurveSelection,
}

/// The default identity FX level curve.
pub fn linear_fx_level_curve() -> ResolvedFxLevelCurve {
    ResolvedFxLevelCurve {
        curve: engine_dsp::linear_fx_level_curve(),
        selection: FxLevelCurveSelection::linear(),
    }
}

/// Resolve a preset id or custom group id to a compiled FX level curve.
///
/// The returned curve always has [`FX_LEVEL_CURVE_CHANNELS`] outputs.
pub fn resolve_fx_level_curve(
    id: &str,
    library: &CurveLibrary,
) -> Result<ResolvedFxLevelCurve, String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("FX level curve id is empty".to_string());
    }
    if id.eq_ignore_ascii_case(DEFAULT_FX_LEVEL_CURVE_ID) {
        return Ok(linear_fx_level_curve());
    }

    let group = library
        .group(id)
        .ok_or_else(|| format!("unknown FX level curve `{id}`"))?;
    group
        .group
        .expect_channels(FX_LEVEL_CURVE_CHANNELS)
        .map_err(|error| format!("curve group `{id}`: {error}"))?;
    let curve = ResolvedCurveGroup::single(&group.group)
        .map_err(|error| format!("curve group `{id}`: {error}"))?;
    let display_name = if group.display_name.is_empty() {
        group.id.clone()
    } else {
        group.display_name.clone()
    };
    Ok(ResolvedFxLevelCurve {
        curve,
        selection: FxLevelCurveSelection {
            id: group.id.clone(),
            display_name,
        },
    })
}

impl crate::Engine {
    /// Install an already-compiled FX level curve on the shared unit.
    ///
    /// Resolve and compile before calling; this only swaps the prepared value
    /// under the DSP lock, so a running session changes without a restart.
    pub fn set_fx_level_curve(&mut self, curve: ResolvedCurveGroup) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        dsp.mixer_mut().set_fx_level_curve(curve)?;
        Ok(())
    }

    /// Select a preset or custom one-channel group by id.
    ///
    /// Returns `Some(message)` when the id did not resolve and `linear` was
    /// installed instead; callers publish that as a notice. Playback never fails
    /// because of a stale id.
    pub fn select_fx_level_curve(&mut self, id: &str) -> Result<Option<String>> {
        let resolved = match resolve_fx_level_curve(id, &self.curve_library) {
            Ok(resolved) => resolved,
            Err(error) => {
                let fallback = linear_fx_level_curve();
                self.set_fx_level_curve(fallback.curve)?;
                self.fx_level_selection = fallback.selection;
                return Ok(Some(format!("{error}; using {DEFAULT_FX_LEVEL_CURVE_ID}")));
            }
        };
        self.set_fx_level_curve(resolved.curve)?;
        self.fx_level_selection = resolved.selection;
        Ok(None)
    }

    /// The active FX level curve selection id and display name.
    pub fn fx_level_curve_selection(&self) -> &FxLevelCurveSelection {
        &self.fx_level_selection
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus_api::{
        CurveGroupSpec, CurveLibrarySpec, CurvePointSpec, CurveSegmentKind, CurveSegmentSpec,
        CurveSpecSpec,
    };

    fn linear_spec(y0: f32, y1: f32) -> CurveSpecSpec {
        CurveSpecSpec {
            points: vec![
                CurvePointSpec { x: 0.0, y: y0 },
                CurvePointSpec { x: 1.0, y: y1 },
            ],
            segments: vec![CurveSegmentSpec {
                kind: CurveSegmentKind::Linear,
                rate: 0.0,
                c1x: 0.0,
                c1y: 0.0,
                c2x: 0.0,
                c2y: 0.0,
            }],
        }
    }

    fn library_with(group_id: &str, channels: usize) -> CurveLibrary {
        let curves = (0..channels)
            .map(|_| linear_spec(0.0, 1.0))
            .collect::<Vec<_>>();
        let spec = CurveLibrarySpec {
            groups: vec![CurveGroupSpec {
                id: group_id.to_string(),
                display_name: "Custom FX Level".to_string(),
                curves,
            }],
            sets: vec![],
        };
        crate::curve_library::library_from_spec(&spec).expect("valid library")
    }

    #[test]
    fn default_linear_id_resolves_without_a_custom_group() {
        let resolved = resolve_fx_level_curve("linear", &CurveLibrary::default()).expect("linear");
        assert_eq!(resolved.curve.channels(), FX_LEVEL_CURVE_CHANNELS);
        assert_eq!(resolved.selection, FxLevelCurveSelection::linear());
        assert!((resolved.curve.evaluate(0.25).unwrap()[0] - 0.25).abs() < 1e-5);
    }

    #[test]
    fn custom_one_channel_group_resolves_and_rejects_channel_counts() {
        let library = library_with("taper", 1);
        let resolved = resolve_fx_level_curve("taper", &library).expect("custom group");
        assert_eq!(resolved.selection.id, "taper");
        assert_eq!(resolved.selection.display_name, "Custom FX Level");

        let two_channel = library_with("pair", 2);
        let error = resolve_fx_level_curve("pair", &two_channel).unwrap_err();
        assert!(error.contains("needs 1 curves"), "{error}");
    }

    #[test]
    fn unknown_id_is_an_error() {
        let error = resolve_fx_level_curve("nope", &CurveLibrary::default()).unwrap_err();
        assert!(error.contains("unknown FX level curve"), "{error}");
    }
}
