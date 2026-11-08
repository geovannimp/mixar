//! Crossfader curve selection, custom curve library, and behavior wiring.
//!
//! Settings, bus events, controller mappings, and [`crate::EngineStatus`] all
//! speak the plain [`bus_api`] types; this module is the one place that turns
//! them into compiled [`ResolvedCrossfaderCurve`] tables.
//!
//! Resolution order for a selection id:
//!
//! 1. the crossfader preset registry ([`crate::curve_presets`]),
//! 2. a named custom group in the library,
//! 3. a named curve set in the library (installed with morph `0`).
//!
//! Unknown ids fall back to `constant_power` so playback never fails to load.

use anyhow::{anyhow, Result};
use audio_core::{CrossfaderCurveBehavior, ResolvedCrossfaderCurve};
use bus_api::{
    CrossfaderBehaviorSetting, CrossfaderCurveStatus, CurveGroupSpec, CurveLibrarySpec,
    CurveRefSpec, CurveSegmentKind, CurveSegmentSpec, CurveSetSpec, CurveSpecSpec,
    DEFAULT_CROSSFADER_CURVE_ID,
};
use curve_core::{
    CurveGroup, CurveGroupRef, CurveLibrary, CurvePoint, CurveRefKind, CurveSet, CurveSpec,
    NamedCurveGroup, SegmentKind,
};

use crate::curve_presets::{self, CrossfaderPreset, DEFAULT_PRESET_ID};

/// The active crossfader selection as the control thread remembers it.
///
/// Compiled tables lose their ids, so the engine keeps this alongside them for
/// [`crate::EngineStatus`]. The morph position is read from the installed
/// tables, not stored here, so the two cannot disagree.
#[derive(Clone, Debug, PartialEq)]
pub struct CrossfaderCurveSelection {
    /// Canonical id of the active preset, custom group, or curve set.
    pub id: String,
    /// Human-readable name for status and settings.
    pub display_name: String,
}

impl Default for CrossfaderCurveSelection {
    fn default() -> Self {
        Self::constant_power()
    }
}

impl CrossfaderCurveSelection {
    /// The equal-power default selection.
    pub fn constant_power() -> Self {
        Self {
            id: DEFAULT_PRESET_ID.to_string(),
            display_name: "Constant Power".to_string(),
        }
    }

    /// Status row for [`crate::EngineStatus`], with the live morph and flags.
    pub fn status(&self, morph: f32, behavior: CrossfaderCurveBehavior) -> CrossfaderCurveStatus {
        CrossfaderCurveStatus {
            id: self.id.clone(),
            display_name: self.display_name.clone(),
            morph,
            hamster: behavior.hamster,
            disabled: behavior.disabled,
        }
    }
}

/// A resolved curve plus the selection it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedCurveSelection {
    pub curve: ResolvedCrossfaderCurve,
    pub selection: CrossfaderCurveSelection,
}

impl ResolvedCurveSelection {
    /// Whether the resolved selection sweeps more than one curve.
    pub fn is_set(&self) -> bool {
        self.curve.gains().len() > 1
    }
}

/// Convert a persisted spec into a validated [`CurveLibrary`].
///
/// Duplicate group or set ids, malformed curves, unknown set members, and
/// custom ids that shadow a crossfader preset are rejected.
pub fn library_from_spec(spec: &CurveLibrarySpec) -> Result<CurveLibrary, String> {
    let groups = spec
        .groups
        .iter()
        .map(group_from_spec)
        .collect::<Result<Vec<_>, _>>()?;
    let sets = spec
        .sets
        .iter()
        .map(set_from_spec)
        .collect::<Result<Vec<_>, _>>()?;
    CurveLibrary::new(groups, sets).map_err(|error| error.to_string())
}

/// Convert one edited curve group back to its settings form.
///
/// The inverse of [`group_from_spec`]: the editor round-trips a group through
/// settings without losing points, segment kinds, rates, or Bézier handles.
pub fn group_to_spec(id: &str, display_name: &str, group: &CurveGroup) -> CurveGroupSpec {
    CurveGroupSpec {
        id: id.to_string(),
        display_name: display_name.to_string(),
        curves: group.curves.iter().map(curve_to_spec).collect(),
    }
}

fn curve_to_spec(curve: &CurveSpec) -> CurveSpecSpec {
    CurveSpecSpec {
        points: curve
            .points
            .iter()
            .map(|point| bus_api::CurvePointSpec {
                x: point.x,
                y: point.y,
            })
            .collect(),
        segments: curve.segments.iter().map(segment_to_spec).collect(),
    }
}

fn segment_to_spec(segment: &SegmentKind) -> CurveSegmentSpec {
    let mut spec = CurveSegmentSpec {
        kind: CurveSegmentKind::Linear,
        rate: 0.0,
        c1x: 0.0,
        c1y: 0.0,
        c2x: 0.0,
        c2y: 0.0,
    };
    match *segment {
        SegmentKind::Linear => {}
        SegmentKind::Exponential { rate } => {
            spec.kind = CurveSegmentKind::Exponential;
            spec.rate = rate;
        }
        SegmentKind::Logarithmic { rate } => {
            spec.kind = CurveSegmentKind::Logarithmic;
            spec.rate = rate;
        }
        SegmentKind::CubicBezier { c1, c2 } => {
            spec.kind = CurveSegmentKind::CubicBezier;
            spec.c1x = c1[0];
            spec.c1y = c1[1];
            spec.c2x = c2[0];
            spec.c2y = c2[1];
        }
    }
    spec
}

/// Convert one settings group into a validated [`CurveGroup`].
///
/// Exposed for settings-time callers (VirtualDJ import, curve editor previews)
/// that hold a single group rather than a whole library.
pub fn group_from_spec(spec: &CurveGroupSpec) -> Result<NamedCurveGroup, String> {
    reject_preset_shadow(&spec.id, "curve group")?;
    let curves = spec
        .curves
        .iter()
        .map(curve_from_spec)
        .collect::<Result<Vec<_>, _>>()?;
    let group =
        CurveGroup::new(curves).map_err(|error| format!("curve group `{}`: {error}", spec.id))?;
    NamedCurveGroup::new(spec.id.clone(), spec.display_name.clone(), group)
        .map_err(|error| format!("curve group `{}`: {error}", spec.id))
}

fn set_from_spec(spec: &CurveSetSpec) -> Result<CurveSet, String> {
    reject_preset_shadow(&spec.id, "curve set")?;
    if spec.members.len() < 2 {
        return Err(format!(
            "curve set `{}` needs at least two members to sweep",
            spec.id
        ));
    }
    let members = spec
        .members
        .iter()
        .map(reference_from_spec)
        .collect::<Result<Vec<_>, _>>()?;
    CurveSet::new(spec.id.clone(), spec.display_name.clone(), members)
        .map_err(|error| format!("curve set `{}`: {error}", spec.id))
}

fn reject_preset_shadow(id: &str, kind: &str) -> Result<(), String> {
    if curve_presets::preset_info(id).is_some() {
        return Err(format!(
            "{kind} id `{id}` is reserved for a crossfader preset"
        ));
    }
    Ok(())
}

fn curve_from_spec(spec: &CurveSpecSpec) -> Result<CurveSpec, String> {
    let points = spec
        .points
        .iter()
        .map(|point| CurvePoint::new(point.x, point.y))
        .collect::<Vec<_>>();
    let segments = spec
        .segments
        .iter()
        .map(segment_from_spec)
        .collect::<Vec<_>>();
    CurveSpec::new(points, segments).map_err(|error| error.to_string())
}

fn segment_from_spec(spec: &CurveSegmentSpec) -> SegmentKind {
    match spec.kind {
        CurveSegmentKind::Linear => SegmentKind::Linear,
        CurveSegmentKind::Exponential => SegmentKind::Exponential { rate: spec.rate },
        CurveSegmentKind::Logarithmic => SegmentKind::Logarithmic { rate: spec.rate },
        CurveSegmentKind::CubicBezier => SegmentKind::CubicBezier {
            c1: [spec.c1x, spec.c1y],
            c2: [spec.c2x, spec.c2y],
        },
    }
}

fn reference_from_spec(spec: &CurveRefSpec) -> Result<CurveGroupRef, String> {
    if curve_presets::preset_info(&spec.id).is_some() {
        let preset = CrossfaderPreset::resolve_with_param(&spec.id, spec.parameter)
            .map_err(|error| error.to_string())?;
        return Ok(preset.group_ref());
    }
    if spec.parameter.is_some() {
        return Err(format!(
            "curve `{}` is not a preset, so it takes no parameter",
            spec.id
        ));
    }
    let reference = CurveGroupRef::custom(spec.id.clone());
    reference.validate().map_err(|error| error.to_string())?;
    Ok(reference)
}

/// Resolve a selection id against the preset registry and `library`.
///
/// Unknown, reserved, and malformed ids are errors; callers decide the
/// fallback (the engine installs `constant_power` and publishes a notice).
pub fn resolve_selection(
    id: &str,
    library: &CurveLibrary,
) -> Result<ResolvedCurveSelection, String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("crossfader curve id must not be empty".to_string());
    }

    if let Ok(preset) = CrossfaderPreset::resolve(id) {
        let group = preset.curve_group().map_err(|error| error.to_string())?;
        let display_name = curve_presets::preset_info(preset.id())
            .map(|info| info.display_name)
            .unwrap_or(preset.id());
        return Ok(ResolvedCurveSelection {
            curve: ResolvedCrossfaderCurve::from_group(group).map_err(|error| error.to_string())?,
            selection: CrossfaderCurveSelection {
                id: preset.id().to_string(),
                display_name: display_name.to_string(),
            },
        });
    }

    if let Some(group) = library.group(id) {
        return Ok(ResolvedCurveSelection {
            curve: ResolvedCrossfaderCurve::from_group(group.group.clone())
                .map_err(|error| error.to_string())?,
            selection: CrossfaderCurveSelection {
                id: group.id.clone(),
                display_name: group.display_name.clone(),
            },
        });
    }

    if let Some(set) = library.set(id) {
        let groups = set
            .groups
            .iter()
            .map(|reference| resolve_group_ref(reference, library))
            .collect::<Result<Vec<_>, _>>()?;
        let curve =
            ResolvedCrossfaderCurve::from_groups(&groups).map_err(|error| error.to_string())?;
        if curve.gains().len() < 2 {
            return Err(format!(
                "curve set `{id}` must hold at least two members to sweep"
            ));
        }
        return Ok(ResolvedCurveSelection {
            curve,
            selection: CrossfaderCurveSelection {
                id: set.id.clone(),
                display_name: set.display_name.clone(),
            },
        });
    }

    Err(format!("unknown crossfader curve `{id}`"))
}

/// Resolve a set member (preset reference or custom group id).
fn resolve_group_ref(
    reference: &CurveGroupRef,
    library: &CurveLibrary,
) -> Result<CurveGroup, String> {
    match &reference.kind {
        CurveRefKind::Preset { id, param } => {
            let preset = CrossfaderPreset::resolve_with_param(id, *param)
                .map_err(|error| error.to_string())?;
            preset.curve_group().map_err(|error| error.to_string())
        }
        CurveRefKind::Custom { id } => library
            .group(id)
            .map(|group| group.group.clone())
            .ok_or_else(|| format!("curve set references unknown group `{id}`")),
    }
}

/// The equal-power fallback used when a selection id does not resolve.
pub fn constant_power_selection() -> ResolvedCurveSelection {
    let preset =
        CrossfaderPreset::resolve(DEFAULT_CROSSFADER_CURVE_ID).unwrap_or(CrossfaderPreset::DEFAULT);
    ResolvedCurveSelection {
        curve: ResolvedCrossfaderCurve::from_group(
            preset
                .curve_group()
                .expect("the default crossfader preset always builds"),
        )
        .expect("the default crossfader curve always compiles"),
        selection: CrossfaderCurveSelection::constant_power(),
    }
}

/// Map the settings form of the behavior flags onto the audio-core type.
pub fn behavior_from_setting(setting: CrossfaderBehaviorSetting) -> CrossfaderCurveBehavior {
    CrossfaderCurveBehavior {
        disabled: setting.disabled,
        hamster: setting.hamster,
    }
}

/// Map the audio-core behavior flags onto the settings/status form.
pub fn behavior_setting(behavior: CrossfaderCurveBehavior) -> CrossfaderBehaviorSetting {
    CrossfaderBehaviorSetting {
        disabled: behavior.disabled,
        hamster: behavior.hamster,
    }
}

impl crate::Engine {
    /// Replace the custom curve library.
    ///
    /// The library is validated before anything is stored; duplicate ids,
    /// malformed curves, and unknown set members reject the whole update, so
    /// the previous library stays active.
    pub fn set_curve_library(&mut self, spec: &CurveLibrarySpec) -> Result<()> {
        self.curve_library = library_from_spec(spec).map_err(|error| anyhow!(error))?;
        Ok(())
    }

    /// The active custom curve library (empty when none was installed).
    pub fn curve_library(&self) -> &CurveLibrary {
        &self.curve_library
    }

    /// Select a preset, custom group, or curve set by id.
    ///
    /// Returns `Some(message)` when the id did not resolve and
    /// `constant_power` was installed instead; callers publish that as a
    /// notice. Playback never fails because of a stale id.
    pub fn select_crossfader_curve(&mut self, id: &str) -> Result<Option<String>> {
        let resolved = match resolve_selection(id, &self.curve_library) {
            Ok(resolved) => resolved,
            Err(error) => {
                self.install_curve_selection(constant_power_selection())?;
                return Ok(Some(format!(
                    "{error}; using {DEFAULT_CROSSFADER_CURVE_ID}"
                )));
            }
        };
        self.install_curve_selection(resolved)?;
        Ok(None)
    }

    /// Install and record a compiled curve selection, keeping the current
    /// behavior flags and resetting the morph to the first member.
    pub fn install_curve_selection(&mut self, resolved: ResolvedCurveSelection) -> Result<()> {
        let ResolvedCurveSelection { curve, selection } = resolved;
        self.install_curves_keeping_behavior(curve)?;
        self.crossfader_selection = selection;
        Ok(())
    }

    /// Sweep the curve set `set` to morph `amount` (`0..1`).
    ///
    /// Installs `set` first when it is not the active selection. Returns
    /// `Some(message)` when the set did not resolve and `constant_power` was
    /// installed instead.
    pub fn select_crossfader_curve_set(
        &mut self,
        set: &str,
        amount: f32,
        soft_takeover: bool,
    ) -> Result<Option<String>> {
        let mut notice = None;
        if self.crossfader_selection.id != set {
            match resolve_selection(set, &self.curve_library) {
                Ok(resolved) => self.install_curve_selection(resolved)?,
                Err(error) => {
                    self.install_curve_selection(constant_power_selection())?;
                    notice = Some(format!("{error}; using {DEFAULT_CROSSFADER_CURVE_ID}"));
                }
            }
        }

        if self.crossfader_selection.id == DEFAULT_CROSSFADER_CURVE_ID {
            // A single member has no morph; the knob still moves without
            // gambling on a stale selection.
            return Ok(notice);
        }

        let current = self.crossfader_curve_morph().unwrap_or(0.0);
        let key = crate::soft_takeover::key_mixer("crossfader_curve_amount");
        if !self
            .soft_takeover_mut()
            .allow(&key, soft_takeover, current, amount)
        {
            return Ok(notice);
        }
        self.set_crossfader_curve_morph(amount)?;
        Ok(notice)
    }

    /// Replace the crossfader behavior flags without rebuilding the tables.
    pub fn apply_crossfader_behavior(&mut self, behavior: CrossfaderBehaviorSetting) -> Result<()> {
        self.set_crossfader_behavior(behavior_from_setting(behavior))
    }

    /// The active selection id and display name.
    pub fn crossfader_curve_selection(&self) -> &CrossfaderCurveSelection {
        &self.crossfader_selection
    }

    /// Status row for the running engine.
    pub(crate) fn crossfader_curve_status(
        &self,
        dsp: &engine_dsp::DspEngine,
    ) -> CrossfaderCurveStatus {
        let mixer = dsp.mixer();
        self.crossfader_selection.status(
            mixer.crossfader_curve_morph(),
            mixer.crossfader_curve_behavior(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus_api::{CurvePointSpec, CurveRefSpec};

    fn linear_curve(y0: f32, y1: f32) -> CurveSpecSpec {
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

    fn linear_group(id: &str) -> CurveGroupSpec {
        CurveGroupSpec {
            id: id.to_string(),
            display_name: id.to_string(),
            curves: vec![linear_curve(1.0, 0.0), linear_curve(0.0, 1.0)],
        }
    }

    #[test]
    fn library_spec_round_trips_groups_and_sets() {
        let spec = CurveLibrarySpec {
            groups: vec![linear_group("tight")],
            sets: vec![CurveSetSpec {
                id: "sweep".to_string(),
                display_name: "Sweep".to_string(),
                members: vec![
                    CurveRefSpec {
                        id: "tight".to_string(),
                        parameter: None,
                    },
                    CurveRefSpec {
                        id: "linear".to_string(),
                        parameter: None,
                    },
                ],
            }],
        };
        let library = library_from_spec(&spec).expect("valid library");
        assert!(library.group("tight").is_some());
        assert!(library.set("sweep").is_some());

        let resolved = resolve_selection("sweep", &library).expect("set resolves");
        assert_eq!(resolved.selection.id, "sweep");
        assert!(resolved.is_set());
        assert_eq!(resolved.curve.gains().len(), 2);
    }

    #[test]
    fn duplicate_and_shadowing_ids_are_rejected() {
        let duplicate = CurveLibrarySpec {
            groups: vec![linear_group("same"), linear_group("same")],
            sets: vec![],
        };
        let error = library_from_spec(&duplicate).expect_err("duplicates rejected");
        assert!(error.to_lowercase().contains("duplicate"), "{error}");

        let shadow = CurveLibrarySpec {
            groups: vec![linear_group("constant_power")],
            sets: vec![],
        };
        let error = library_from_spec(&shadow).expect_err("preset ids are reserved");
        assert!(error.contains("reserved"), "{error}");

        let duplicate_set = CurveLibrarySpec {
            groups: vec![linear_group("a")],
            sets: vec![
                CurveSetSpec {
                    id: "s".into(),
                    display_name: String::new(),
                    members: vec![
                        CurveRefSpec {
                            id: "a".into(),
                            parameter: None,
                        },
                        CurveRefSpec {
                            id: "linear".into(),
                            parameter: None,
                        },
                    ],
                },
                CurveSetSpec {
                    id: "s".into(),
                    display_name: String::new(),
                    members: vec![
                        CurveRefSpec {
                            id: "a".into(),
                            parameter: None,
                        },
                        CurveRefSpec {
                            id: "linear".into(),
                            parameter: None,
                        },
                    ],
                },
            ],
        };
        let error = library_from_spec(&duplicate_set).expect_err("duplicate sets rejected");
        assert!(error.to_lowercase().contains("duplicate"), "{error}");
    }

    #[test]
    fn unknown_ids_error_and_presets_resolve() {
        let library = CurveLibrary::default();
        assert!(resolve_selection("nope", &library).is_err());
        assert_eq!(
            resolve_selection("linear", &library)
                .expect("preset")
                .selection
                .id,
            "linear"
        );
        // Reserved names stay unavailable instead of becoming custom groups.
        assert!(resolve_selection("full", &library).is_err());
        assert_eq!(constant_power_selection().selection.id, DEFAULT_PRESET_ID);
    }

    #[test]
    fn behavior_round_trips_between_forms() {
        let setting = CrossfaderBehaviorSetting {
            hamster: true,
            disabled: true,
        };
        assert_eq!(behavior_setting(behavior_from_setting(setting)), setting);
    }

    #[test]
    fn curve_sets_need_two_members_to_sweep() {
        let spec = CurveLibrarySpec {
            groups: vec![linear_group("a")],
            sets: vec![CurveSetSpec {
                id: "single".into(),
                display_name: String::new(),
                members: vec![CurveRefSpec {
                    id: "a".into(),
                    parameter: None,
                }],
            }],
        };
        let error = library_from_spec(&spec).expect_err("a one-member set cannot morph");
        assert!(error.contains("at least two"), "{error}");
    }

    #[test]
    fn group_wire_conversion_round_trips_every_segment_kind() {
        let linear = CurveSegmentSpec {
            kind: CurveSegmentKind::Linear,
            rate: 0.0,
            c1x: 0.0,
            c1y: 0.0,
            c2x: 0.0,
            c2y: 0.0,
        };
        let spec = CurveLibrarySpec {
            groups: vec![CurveGroupSpec {
                id: "edited".into(),
                display_name: "Edited".into(),
                curves: vec![CurveSpecSpec {
                    points: vec![
                        CurvePointSpec { x: 0.0, y: 0.0 },
                        CurvePointSpec { x: 0.25, y: 0.4 },
                        CurvePointSpec { x: 0.7, y: 0.6 },
                        CurvePointSpec { x: 1.0, y: 1.0 },
                    ],
                    segments: vec![
                        CurveSegmentSpec {
                            kind: CurveSegmentKind::Exponential,
                            rate: 3.0,
                            ..linear
                        },
                        CurveSegmentSpec {
                            kind: CurveSegmentKind::Logarithmic,
                            rate: -2.0,
                            ..linear
                        },
                        CurveSegmentSpec {
                            kind: CurveSegmentKind::CubicBezier,
                            rate: 0.0,
                            c1x: 0.3,
                            c1y: 0.1,
                            c2x: 0.7,
                            c2y: 0.9,
                        },
                    ],
                }],
            }],
            sets: vec![],
        };
        let named = group_from_spec(&spec.groups[0]).expect("valid group");
        let round_tripped = group_to_spec(&named.id, &named.display_name, &named.group);
        assert_eq!(round_tripped.id, "edited");
        assert_eq!(round_tripped.display_name, "Edited");
        assert_eq!(round_tripped.curves, spec.groups[0].curves);
    }

    #[test]
    fn wire_constants_match_the_preset_catalog() {
        assert_eq!(DEFAULT_CROSSFADER_CURVE_ID, DEFAULT_PRESET_ID);
        let available = crate::curve_presets::available_presets()
            .map(|preset| preset.id)
            .collect::<Vec<_>>();
        assert_eq!(available, bus_api::CROSSFADER_CURVE_CYCLE.to_vec());
    }
}
