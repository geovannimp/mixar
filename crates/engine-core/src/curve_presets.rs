//! Crossfader preset registry and catalog.
//!
//! Generic curve *shapes* live in [`curve_core::presets`]. This module owns the
//! crossfader-specific presets: it resolves a stable snake_case id (or a display
//! alias such as `Constant Power`) to the two-channel [`CurveGroup`] installed
//! by the engine, and validates the result with the crossfader endpoint rules.
//!
//! The registry depends only on `curve-core`, so Settings can list the catalog
//! without pulling in `engine-dsp`.
//!
//! # First-slice presets
//!
//! | id | gain A | gain B |
//! |----|--------|--------|
//! | `constant_power` | `cos(x * π/2)` | `sin(x * π/2)` |
//! | `linear` | `1 - x` | `x` |
//! | `fast_cut(cut = 0.15)` | `clamp((1 - x) / cut, 0, 1)` | `clamp(x / cut, 0, 1)` |
//!
//! Ids are canonical snake_case. Display aliases may contain spaces and are
//! matched case-insensitively. The names `full`, `intermediate`, `slow_fade`,
//! `slow_cut`, `transition`, and `logarithmic` are reserved: they appear in the
//! catalog with [`CrossfaderPresetInfo::available`] set to `false` and resolve to
//! [`CrossfaderPresetError::Reserved`].

use curve_core::{CurveError, CurveGroup, CurveGroupRef, CurvePoint, CurveSpec, SegmentKind};
use std::str::FromStr;
use thiserror::Error;

/// Canonical id of the equal-power crossfader preset.
pub const CONSTANT_POWER: &str = "constant_power";
/// Canonical id of the linear crossfader preset.
pub const LINEAR: &str = "linear";
/// Canonical id of the parameterized fast-cut crossfader preset.
pub const FAST_CUT: &str = "fast_cut";
/// Canonical id reserved for a future full-range crossfader preset.
pub const FULL: &str = "full";
/// Canonical id reserved for a future intermediate crossfader preset.
pub const INTERMEDIATE: &str = "intermediate";
/// Canonical id reserved for a future slow-fade crossfader preset.
pub const SLOW_FADE: &str = "slow_fade";
/// Canonical id reserved for a future slow-cut crossfader preset.
pub const SLOW_CUT: &str = "slow_cut";
/// Canonical id reserved for a future transition crossfader preset.
pub const TRANSITION: &str = "transition";
/// Canonical id reserved for a future logarithmic crossfader preset.
pub const LOGARITHMIC: &str = "logarithmic";

/// Canonical id used when no preset is configured.
pub const DEFAULT_PRESET_ID: &str = CONSTANT_POWER;
/// Default `cut` parameter for [`FAST_CUT`].
pub const DEFAULT_CUT: f32 = 0.15;
/// Exclusive lower bound of the `cut` parameter.
pub const CUT_MIN: f32 = 0.0;
/// Inclusive upper bound of the `cut` parameter.
pub const CUT_MAX: f32 = 1.0;

/// A crossfader curve group has one curve for deck A and one for deck B.
pub const CROSSFADER_CHANNELS: usize = 2;
/// Absolute tolerance for the endpoint rules.
pub const ENDPOINT_TOLERANCE: f32 = 1e-5;

/// Names reserved for future crossfader presets.
///
/// They are present in the catalog as unavailable entries instead of being
/// silently treated as unknown ids.
pub const RESERVED_PRESET_IDS: [&str; 6] = [
    FULL,
    INTERMEDIATE,
    SLOW_FADE,
    SLOW_CUT,
    TRANSITION,
    LOGARITHMIC,
];

/// Samples used when validating a group's normalized output range.
const VALIDATION_STEPS: usize = 256;

/// Optional numeric parameter accepted by a crossfader preset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossfaderPresetParameter {
    /// Parameter name as used in expressions such as `fast_cut(cut: 0.1)`.
    pub name: &'static str,
    /// Value used when the parameter is omitted.
    pub default: f32,
    /// Lower bound.
    pub min: f32,
    /// Whether the lower bound itself is rejected.
    pub min_exclusive: bool,
    /// Upper bound.
    pub max: f32,
}

impl CrossfaderPresetParameter {
    /// The `fast_cut` `cut` parameter.
    pub const CUT: Self = Self {
        name: "cut",
        default: DEFAULT_CUT,
        min: CUT_MIN,
        min_exclusive: true,
        max: CUT_MAX,
    };

    /// Whether `value` is a valid parameter value.
    pub fn accepts(self, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        let above_min = if self.min_exclusive {
            value > self.min
        } else {
            value >= self.min
        };
        above_min && value <= self.max
    }
}

/// One entry in the crossfader preset catalog.
///
/// The canonical `id` is snake_case; `display_name` and `aliases` are UI-only
/// values and may contain spaces. `available` is `false` for reserved names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossfaderPresetInfo {
    /// Canonical snake_case id used by settings, mappings, and logs.
    pub id: &'static str,
    /// Human-readable name for Settings.
    pub display_name: &'static str,
    /// Additional display aliases accepted by [`canonical_id`].
    pub aliases: &'static [&'static str],
    /// Numeric parameter metadata, if the preset accepts one.
    pub parameter: Option<CrossfaderPresetParameter>,
    /// Whether the preset can be resolved and built today.
    pub available: bool,
}

impl CrossfaderPresetInfo {
    /// Whether this catalog entry matches a raw id or alias.
    pub fn matches(&self, value: &str) -> bool {
        let key = alias_key(value);
        alias_key(self.id) == key
            || alias_key(self.display_name) == key
            || self.aliases.iter().any(|alias| alias_key(alias) == key)
    }
}

/// Every crossfader preset and reserved name.
pub const PRESET_CATALOG: [CrossfaderPresetInfo; 9] = [
    CrossfaderPresetInfo {
        id: CONSTANT_POWER,
        display_name: "Constant Power",
        aliases: &["Constant Power"],
        parameter: None,
        available: true,
    },
    CrossfaderPresetInfo {
        id: LINEAR,
        display_name: "Linear",
        aliases: &["Linear"],
        parameter: None,
        available: true,
    },
    CrossfaderPresetInfo {
        id: FAST_CUT,
        display_name: "Fast Cut",
        aliases: &["Fast Cut"],
        parameter: Some(CrossfaderPresetParameter::CUT),
        available: true,
    },
    CrossfaderPresetInfo {
        id: FULL,
        display_name: "Full",
        aliases: &["Full"],
        parameter: None,
        available: false,
    },
    CrossfaderPresetInfo {
        id: INTERMEDIATE,
        display_name: "Intermediate",
        aliases: &["Intermediate"],
        parameter: None,
        available: false,
    },
    CrossfaderPresetInfo {
        id: SLOW_FADE,
        display_name: "Slow Fade",
        aliases: &["Slow Fade"],
        parameter: None,
        available: false,
    },
    CrossfaderPresetInfo {
        id: SLOW_CUT,
        display_name: "Slow Cut",
        aliases: &["Slow Cut"],
        parameter: None,
        available: false,
    },
    CrossfaderPresetInfo {
        id: TRANSITION,
        display_name: "Transition",
        aliases: &["Transition"],
        parameter: None,
        available: false,
    },
    CrossfaderPresetInfo {
        id: LOGARITHMIC,
        display_name: "Logarithmic",
        aliases: &["Logarithmic"],
        parameter: None,
        available: false,
    },
];

/// The crossfader preset catalog, including unavailable reserved names.
pub fn catalog() -> &'static [CrossfaderPresetInfo] {
    &PRESET_CATALOG
}

/// Iterate over presets that can be selected today.
pub fn available_presets() -> impl Iterator<Item = &'static CrossfaderPresetInfo> {
    PRESET_CATALOG.iter().filter(|preset| preset.available)
}

/// Look up a catalog entry by canonical id, display name, or alias.
///
/// Reserved names return their entry with `available == false`.
pub fn preset_info(value: &str) -> Option<&'static CrossfaderPresetInfo> {
    PRESET_CATALOG.iter().find(|preset| preset.matches(value))
}

/// Canonicalize a raw id or display alias to its snake_case id.
///
/// Returns reserved ids too; check [`preset_info`] for availability. Unknown
/// values return `None`.
pub fn canonical_id(value: &str) -> Option<&'static str> {
    let name = value.split(['(', ':']).next().unwrap_or(value).trim();
    preset_info(name).map(|preset| preset.id)
}

/// Whether `value` names a reserved, unavailable crossfader preset.
pub fn is_reserved(value: &str) -> bool {
    preset_info(value).is_some_and(|preset| !preset.available)
}

/// A validated crossfader preset selection.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CrossfaderPreset {
    /// Equal-power law: `A = cos(x * π/2)`, `B = sin(x * π/2)`.
    #[default]
    ConstantPower,
    /// Linear law: `A = 1 - x`, `B = x`.
    Linear,
    /// Fast cut with a configurable cut length.
    FastCut {
        /// Length of the fade region at each end, in `(0, 1]`.
        cut: f32,
    },
}

impl CrossfaderPreset {
    /// The default selection used when none is configured.
    pub const DEFAULT: Self = Self::ConstantPower;

    /// Canonical snake_case id.
    pub fn id(self) -> &'static str {
        match self {
            Self::ConstantPower => CONSTANT_POWER,
            Self::Linear => LINEAR,
            Self::FastCut { .. } => FAST_CUT,
        }
    }

    /// The numeric parameter, if this preset carries one.
    pub fn parameter(self) -> Option<f32> {
        match self {
            Self::FastCut { cut } => Some(cut),
            Self::ConstantPower | Self::Linear => None,
        }
    }

    /// Validate the selection without building its curves.
    pub fn validate(self) -> Result<(), CrossfaderPresetError> {
        match self {
            Self::FastCut { cut } => validate_cut(cut),
            Self::ConstantPower | Self::Linear => Ok(()),
        }
    }

    /// Build and validate the two-channel crossfader curve group.
    pub fn curve_group(self) -> Result<CurveGroup, CrossfaderPresetError> {
        self.validate()?;
        let curves = match self {
            Self::ConstantPower => audio_core::constant_power_group().curves,
            Self::Linear => linear_curves().to_vec(),
            Self::FastCut { cut } => vec![fast_cut_curve(cut, true), fast_cut_curve(cut, false)],
        };
        let group = CurveGroup::new(curves).map_err(CrossfaderCurveError::from)?;
        validate_crossfader_group(&group)?;
        Ok(group)
    }

    /// A reference suitable for settings and controller mappings.
    pub fn group_ref(self) -> CurveGroupRef {
        match self {
            Self::FastCut { cut } => CurveGroupRef::preset_with_param(FAST_CUT, cut),
            Self::ConstantPower => CurveGroupRef::preset(CONSTANT_POWER),
            Self::Linear => CurveGroupRef::preset(LINEAR),
        }
    }

    /// Resolve an id, display alias, or parameterized expression.
    ///
    /// Accepted forms include `constant_power`, `Constant Power`, `fast_cut`,
    /// `fast_cut(0.1)`, `fast_cut(cut: 0.1)`, and `fast_cut(cut = 0.1)`.
    pub fn resolve(value: &str) -> Result<Self, CrossfaderPresetError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CrossfaderPresetError::Unknown {
                id: value.to_string(),
            });
        }

        if let Some(open) = trimmed.find('(') {
            if !trimmed.ends_with(')') {
                return Err(CrossfaderPresetError::InvalidParameter {
                    id: trimmed[..open].trim().to_string(),
                    expression: trimmed.to_string(),
                });
            }
            let id = trimmed[..open].trim();
            let raw = trimmed[open + 1..trimmed.len() - 1].trim();
            let parameter = parse_parameter(id, raw)?;
            return Self::resolve_with_param(id, parameter);
        }

        if let Some((id, raw)) = trimmed.split_once(':') {
            let parameter = parse_parameter(id.trim(), raw.trim())?;
            return Self::resolve_with_param(id.trim(), parameter);
        }

        Self::resolve_with_param(trimmed, None)
    }

    /// Resolve a canonical id or alias with an optional numeric parameter.
    ///
    /// A missing parameter uses the preset default (`fast_cut` uses
    /// [`DEFAULT_CUT`]).
    pub fn resolve_with_param(
        id: &str,
        parameter: Option<f32>,
    ) -> Result<Self, CrossfaderPresetError> {
        let info =
            preset_info(id).ok_or_else(|| CrossfaderPresetError::Unknown { id: id.to_string() })?;
        if !info.available {
            return Err(CrossfaderPresetError::Reserved {
                id: info.id.to_string(),
            });
        }

        match info.id {
            CONSTANT_POWER => {
                reject_parameter(info.id, parameter)?;
                Ok(Self::ConstantPower)
            }
            LINEAR => {
                reject_parameter(info.id, parameter)?;
                Ok(Self::Linear)
            }
            FAST_CUT => {
                let cut = parameter.unwrap_or(DEFAULT_CUT);
                validate_cut(cut)?;
                Ok(Self::FastCut { cut })
            }
            other => Err(CrossfaderPresetError::Unknown {
                id: other.to_string(),
            }),
        }
    }

    /// Resolve an id or alias, falling back to [`CrossfaderPreset::DEFAULT`].
    ///
    /// Settings files and mappings may outlive a preset name or carry a typo;
    /// playback keeps its equal-power law instead of failing to load.
    pub fn resolve_or_default(value: &str) -> Self {
        Self::resolve(value).unwrap_or(Self::DEFAULT)
    }

    /// Alias for [`CrossfaderPreset::resolve`].
    pub fn from_id(value: &str) -> Result<Self, CrossfaderPresetError> {
        Self::resolve(value)
    }

    /// Alias for [`CrossfaderPreset::resolve_or_default`].
    pub fn from_id_or_default(value: &str) -> Self {
        Self::resolve_or_default(value)
    }
}

impl FromStr for CrossfaderPreset {
    type Err = CrossfaderPresetError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::resolve(value)
    }
}

/// The default crossfader preset used when none is configured.
pub fn default_preset() -> CrossfaderPreset {
    CrossfaderPreset::DEFAULT
}

/// Equal-power crossfader curve group: `(cos(x * π/2), sin(x * π/2))`.
///
/// This is the law used by the mixer's default crossfader gain path.
pub fn constant_power() -> CurveGroup {
    CrossfaderPreset::ConstantPower
        .curve_group()
        .expect("the constant_power preset always produces a valid crossfader group")
}

/// Linear crossfader curve group: `(1 - x, x)`.
pub fn linear() -> CurveGroup {
    CrossfaderPreset::Linear
        .curve_group()
        .expect("the linear preset always produces a valid crossfader group")
}

/// Fast-cut crossfader curve group with an explicit `cut` parameter.
///
/// `cut` must be finite and in `(0, 1]`.
pub fn fast_cut(cut: f32) -> Result<CurveGroup, CrossfaderPresetError> {
    CrossfaderPreset::FastCut { cut }.curve_group()
}

/// Fast-cut crossfader curve group using [`DEFAULT_CUT`].
pub fn fast_cut_default() -> CurveGroup {
    fast_cut(DEFAULT_CUT).expect("the default fast_cut parameter is valid")
}

/// Resolve an id, display alias, or parameterized expression.
pub fn resolve(id: &str) -> Result<CrossfaderPreset, CrossfaderPresetError> {
    CrossfaderPreset::resolve(id)
}

/// Resolve an id or alias and build its curve group.
pub fn preset_group(id: &str) -> Result<CurveGroup, CrossfaderPresetError> {
    CrossfaderPreset::resolve(id)?.curve_group()
}

/// Alias for [`preset_group`].
pub fn preset(id: &str) -> Result<CurveGroup, CrossfaderPresetError> {
    preset_group(id)
}

/// Resolve an id or alias and build its curve group, falling back to the default.
pub fn preset_group_or_default(id: &str) -> CurveGroup {
    CrossfaderPreset::resolve_or_default(id)
        .curve_group()
        .expect("the default crossfader preset is always valid")
}

/// Validate a group against the crossfader endpoint rules.
///
/// The group must hold exactly two curves, both curves must be valid and stay
/// in `0..=1`, and the gains must reach `(A, B) = (1, 0)` at `x = 0` and
/// `(0, 1)` at `x = 1` within [`ENDPOINT_TOLERANCE`].
pub fn validate_crossfader_group(group: &CurveGroup) -> Result<(), CrossfaderCurveError> {
    if group.channels() != CROSSFADER_CHANNELS {
        return Err(CrossfaderCurveError::ChannelCount {
            count: group.channels(),
        });
    }
    group.validate()?;

    for channel in 0..CROSSFADER_CHANNELS {
        for step in 0..=VALIDATION_STEPS {
            let x = step as f32 / VALIDATION_STEPS as f32;
            let value =
                group
                    .evaluate_channel(channel, x)
                    .ok_or(CrossfaderCurveError::ChannelCount {
                        count: group.channels(),
                    })?;
            if !value.is_finite() {
                return Err(CrossfaderCurveError::NonFinite { channel, x });
            }
            if value < -ENDPOINT_TOLERANCE || value > 1.0 + ENDPOINT_TOLERANCE {
                return Err(CrossfaderCurveError::OutOfRange { channel, x, value });
            }
        }
    }

    check_endpoint(group, 0, 0.0, 1.0)?;
    check_endpoint(group, 1, 0.0, 0.0)?;
    check_endpoint(group, 0, 1.0, 0.0)?;
    check_endpoint(group, 1, 1.0, 1.0)?;
    Ok(())
}

fn check_endpoint(
    group: &CurveGroup,
    channel: usize,
    x: f32,
    expected: f32,
) -> Result<(), CrossfaderCurveError> {
    let actual = group
        .evaluate_channel(channel, x)
        .ok_or(CrossfaderCurveError::ChannelCount {
            count: group.channels(),
        })?;
    if !actual.is_finite() || (actual - expected).abs() > ENDPOINT_TOLERANCE {
        return Err(CrossfaderCurveError::Endpoint {
            channel,
            x,
            expected,
            actual,
        });
    }
    Ok(())
}

fn validate_cut(cut: f32) -> Result<(), CrossfaderPresetError> {
    let parameter = CrossfaderPresetParameter::CUT;
    if parameter.accepts(cut) {
        Ok(())
    } else {
        Err(CrossfaderPresetError::ParameterOutOfRange {
            id: FAST_CUT.to_string(),
            parameter: parameter.name,
            value: cut,
            min: parameter.min,
            min_exclusive: parameter.min_exclusive,
            max: parameter.max,
        })
    }
}

fn reject_parameter(id: &str, parameter: Option<f32>) -> Result<(), CrossfaderPresetError> {
    match parameter {
        None => Ok(()),
        Some(_) => Err(CrossfaderPresetError::UnexpectedParameter { id: id.to_string() }),
    }
}

fn parse_parameter(id: &str, raw: &str) -> Result<Option<f32>, CrossfaderPresetError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }

    let raw = raw
        .strip_prefix(CrossfaderPresetParameter::CUT.name)
        .map(str::trim)
        .unwrap_or(raw);
    let raw = raw
        .strip_prefix('=')
        .or_else(|| raw.strip_prefix(':'))
        .unwrap_or(raw)
        .trim();

    raw.parse::<f32>()
        .map(Some)
        .map_err(|_| CrossfaderPresetError::InvalidParameter {
            id: id.to_string(),
            expression: raw.to_string(),
        })
}

fn linear_curves() -> [CurveSpec; 2] {
    [two_point(|x| 1.0 - x), two_point(|x| x)]
}

fn fast_cut_curve(cut: f32, deck_a: bool) -> CurveSpec {
    let mut xs = vec![0.0, cut, 1.0 - cut, 1.0];
    if cut < 0.5 && 1.0 - cut >= 1.0 {
        // For a tiny cut the `1 - cut` breakpoint rounds to 1.0. Keep the last
        // representable x before 1 so the drop stays a step rather than a long
        // interpolation across the whole range.
        xs.push(f32::from_bits(1.0f32.to_bits() - 1));
    }
    xs.sort_by(f32::total_cmp);
    xs.dedup();

    let points = xs
        .iter()
        .map(|&x| {
            let y = if deck_a {
                ((1.0 - x) / cut).clamp(0.0, 1.0)
            } else {
                (x / cut).clamp(0.0, 1.0)
            };
            CurvePoint::new(x, y)
        })
        .collect::<Vec<_>>();
    let segments = vec![SegmentKind::Linear; points.len() - 1];
    CurveSpec::new(points, segments)
        .expect("a validated fast_cut parameter always produces a valid piecewise-linear curve")
}

fn two_point(value: impl Fn(f32) -> f32) -> CurveSpec {
    CurveSpec::new(
        vec![
            CurvePoint::new(0.0, value(0.0)),
            CurvePoint::new(1.0, value(1.0)),
        ],
        vec![SegmentKind::Linear],
    )
    .expect("a normalized two-point linear curve is always valid")
}

/// Case- and separator-insensitive alias key (`Constant Power` → `constantpower`).
fn alias_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

/// A crossfader preset could not be resolved, parameterized, or built.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CrossfaderPresetError {
    /// No catalog entry matches the id or alias.
    #[error("unknown crossfader preset `{id}`")]
    Unknown { id: String },

    /// The name is reserved for a future preset.
    #[error("crossfader preset `{id}` is reserved and not available yet")]
    Reserved { id: String },

    /// A parameter was supplied to a preset that does not take one.
    #[error("crossfader preset `{id}` does not take a parameter")]
    UnexpectedParameter { id: String },

    /// The parameter expression could not be parsed.
    #[error("crossfader preset `{id}` has an invalid parameter expression `{expression}`")]
    InvalidParameter { id: String, expression: String },

    /// The parameter is missing, non-finite, or outside its bounds.
    #[error(
        "crossfader preset `{id}` parameter `{parameter}` must be finite and in \
         {min}..{max} (min exclusive: {min_exclusive}), got {value}"
    )]
    ParameterOutOfRange {
        id: String,
        parameter: &'static str,
        value: f32,
        min: f32,
        min_exclusive: bool,
        max: f32,
    },

    /// The generated group failed crossfader validation.
    #[error(transparent)]
    CurveGroup(#[from] CrossfaderCurveError),
}

/// A curve group failed the crossfader endpoint rules.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CrossfaderCurveError {
    /// The underlying curve spec is invalid.
    #[error(transparent)]
    Curve(#[from] CurveError),

    /// A crossfader group must have exactly two curves.
    #[error("a crossfader curve group needs exactly 2 curves, got {count}")]
    ChannelCount { count: usize },

    /// A curve evaluated to `NaN` or infinity.
    #[error("crossfader curve {channel} is not finite at x={x}")]
    NonFinite { channel: usize, x: f32 },

    /// A curve left the normalized output range.
    #[error("crossfader curve {channel} is outside 0..=1 at x={x}: {value}")]
    OutOfRange { channel: usize, x: f32, value: f32 },

    /// A curve did not meet an endpoint rule.
    #[error("crossfader curve {channel} must be {expected} at x={x}, got {actual}")]
    Endpoint {
        channel: usize,
        x: f32,
        expected: f32,
        actual: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f32, expected: f32) -> bool {
        (actual - expected).abs() <= 1e-5
    }

    #[test]
    fn constant_power_matches_the_equal_power_law() {
        let group = constant_power();
        let steps = 1_000;
        for step in 0..=steps {
            let x = step as f32 / steps as f32;
            let angle = x * std::f32::consts::FRAC_PI_2;
            let a = group.evaluate_channel(0, x).expect("channel A");
            let b = group.evaluate_channel(1, x).expect("channel B");
            assert!(close(a, angle.cos()), "A at {x}: {a} != {}", angle.cos());
            assert!(close(b, angle.sin()), "B at {x}: {b} != {}", angle.sin());
            assert!(close(a * a + b * b, 1.0), "equal-power parity at {x}");
            assert!((a - angle.cos()).abs() <= 1e-6, "A precision at {x}: {a}");
            assert!((b - angle.sin()).abs() <= 1e-6, "B precision at {x}: {b}");
        }
        assert_eq!(group.evaluate_channel(0, 0.0), Some(1.0));
        assert_eq!(group.evaluate_channel(1, 0.0), Some(0.0));
        assert_eq!(group.evaluate_channel(0, 1.0), Some(0.0));
        assert_eq!(group.evaluate_channel(1, 1.0), Some(1.0));
    }

    #[test]
    fn linear_is_the_identity_complement() {
        let group = linear();
        for step in 0..=100 {
            let x = step as f32 / 100.0;
            assert!(close(
                group.evaluate_channel(0, x).expect("channel A"),
                1.0 - x
            ));
            assert!(close(group.evaluate_channel(1, x).expect("channel B"), x));
        }
        assert_eq!(group.evaluate_channel(0, 0.0), Some(1.0));
        assert_eq!(group.evaluate_channel(1, 1.0), Some(1.0));
    }

    #[test]
    fn fast_cut_uses_the_configured_cut() {
        let cut = 0.15;
        let group = fast_cut(cut).expect("valid cut");
        assert!(close(group.evaluate_channel(0, 0.0).expect("A"), 1.0));
        assert!(close(group.evaluate_channel(1, 0.0).expect("B"), 0.0));
        assert!(close(group.evaluate_channel(0, 1.0).expect("A"), 0.0));
        assert!(close(group.evaluate_channel(1, 1.0).expect("B"), 1.0));

        assert!(close(group.evaluate_channel(1, cut / 2.0).expect("B"), 0.5));
        assert!(close(
            group.evaluate_channel(0, 1.0 - cut / 2.0).expect("A"),
            0.5
        ));
        assert!(close(group.evaluate_channel(0, cut / 2.0).expect("A"), 1.0));
        assert!(close(
            group.evaluate_channel(1, 1.0 - cut / 2.0).expect("B"),
            1.0
        ));

        for step in 0..=200 {
            let x = step as f32 / 200.0;
            assert!(close(
                group.evaluate_channel(0, x).expect("A"),
                ((1.0 - x) / cut).clamp(0.0, 1.0)
            ));
            assert!(close(
                group.evaluate_channel(1, x).expect("B"),
                (x / cut).clamp(0.0, 1.0)
            ));
        }
    }

    #[test]
    fn fast_cut_tiny_cut_keeps_both_decks_symmetric() {
        let group = fast_cut(f32::MIN_POSITIVE).expect("tiny positive cut");
        for step in 0..=200 {
            let x = step as f32 / 200.0;
            let a = group.evaluate_channel(0, x).expect("channel A");
            let b = group.evaluate_channel(1, 1.0 - x).expect("channel B");
            assert!(close(a, b), "A({x}) = {a}, B({}) = {b}", 1.0 - x);
        }
    }

    #[test]
    fn fast_cut_default_uses_the_documented_cut() {
        assert_eq!(DEFAULT_CUT, 0.15);
        assert_eq!(
            fast_cut_default(),
            fast_cut(DEFAULT_CUT).expect("default cut is valid")
        );
    }

    #[test]
    fn aliases_canonicalize_to_snake_case_ids() {
        assert_eq!(canonical_id("Constant Power"), Some(CONSTANT_POWER));
        assert_eq!(canonical_id("constant power"), Some(CONSTANT_POWER));
        assert_eq!(canonical_id("CONSTANT-POWER"), Some(CONSTANT_POWER));
        assert_eq!(canonical_id("Fast Cut"), Some(FAST_CUT));
        assert_eq!(canonical_id("slow fade"), Some(SLOW_FADE));
        assert_eq!(canonical_id("linear"), Some(LINEAR));
        assert_eq!(canonical_id("fast_cut(cut: 0.1)"), Some(FAST_CUT));
        assert_eq!(canonical_id("nope"), None);

        assert_eq!(
            CrossfaderPreset::resolve("Constant Power").expect("alias resolves"),
            CrossfaderPreset::ConstantPower
        );
        assert_eq!(
            CrossfaderPreset::resolve("Fast-Cut").expect("alias resolves"),
            CrossfaderPreset::FastCut { cut: DEFAULT_CUT }
        );
        assert_eq!(
            "Constant Power"
                .parse::<CrossfaderPreset>()
                .expect("FromStr alias"),
            CrossfaderPreset::ConstantPower
        );
    }

    #[test]
    fn parameterized_expressions_resolve() {
        assert_eq!(
            CrossfaderPreset::resolve("fast_cut(0.1)").expect("positional"),
            CrossfaderPreset::FastCut { cut: 0.1 }
        );
        assert_eq!(
            CrossfaderPreset::resolve("fast_cut(cut: 0.1)").expect("named"),
            CrossfaderPreset::FastCut { cut: 0.1 }
        );
        assert_eq!(
            CrossfaderPreset::resolve("fast_cut(cut = 0.1)").expect("named with equals"),
            CrossfaderPreset::FastCut { cut: 0.1 }
        );
        assert_eq!(
            CrossfaderPreset::resolve_with_param("fast_cut", Some(0.1)).expect("explicit"),
            CrossfaderPreset::FastCut { cut: 0.1 }
        );
        assert!(CrossfaderPreset::resolve("fast_cut()").is_ok());
    }

    #[test]
    fn parameter_bounds_are_enforced() {
        for cut in [0.0, -0.1, f32::NAN, f32::INFINITY, 1.01] {
            assert!(
                fast_cut(cut).is_err(),
                "cut {cut} must be rejected as out of bounds"
            );
        }
        for cut in [0.01, 0.1, 0.5, 1.0] {
            assert!(fast_cut(cut).is_ok(), "cut {cut} must be accepted");
        }
        assert_eq!(
            CrossfaderPreset::resolve("fast_cut(cut: 2.0)"),
            Err(CrossfaderPresetError::ParameterOutOfRange {
                id: FAST_CUT.to_string(),
                parameter: "cut",
                value: 2.0,
                min: CUT_MIN,
                min_exclusive: true,
                max: CUT_MAX,
            })
        );
        assert!(matches!(
            CrossfaderPreset::resolve("constant_power(0.1)"),
            Err(CrossfaderPresetError::UnexpectedParameter { .. })
        ));
        assert!(matches!(
            CrossfaderPreset::resolve("fast_cut(cut: nope)"),
            Err(CrossfaderPresetError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn reserved_names_are_explicitly_unavailable() {
        for id in RESERVED_PRESET_IDS {
            let info = preset_info(id).expect("reserved names are catalogued");
            assert!(!info.available, "`{id}` must be unavailable");
            assert_eq!(canonical_id(id), Some(id));
            assert!(is_reserved(id));
            assert_eq!(
                CrossfaderPreset::resolve(id),
                Err(CrossfaderPresetError::Reserved { id: id.to_string() })
            );
        }
        assert!(is_reserved("Slow Fade"));
        assert!(!is_reserved(CONSTANT_POWER));
    }

    #[test]
    fn catalog_lists_the_first_slice_and_settings_default() {
        let available = available_presets()
            .map(|preset| preset.id)
            .collect::<Vec<_>>();
        assert_eq!(available, vec![CONSTANT_POWER, LINEAR, FAST_CUT]);
        assert_eq!(catalog().len(), PRESET_CATALOG.len());
        assert_eq!(DEFAULT_PRESET_ID, CONSTANT_POWER);
        assert_eq!(CrossfaderPreset::DEFAULT.id(), DEFAULT_PRESET_ID);
        assert_eq!(default_preset(), CrossfaderPreset::ConstantPower);
        assert_eq!(
            preset_info("Constant Power")
                .expect("catalog entry")
                .display_name,
            "Constant Power"
        );
        let fast_cut = preset_info(FAST_CUT).expect("catalog entry");
        assert_eq!(
            fast_cut.parameter.expect("fast_cut is parameterized").name,
            "cut"
        );
    }

    #[test]
    fn unknown_ids_fall_back_to_the_default() {
        assert_eq!(
            CrossfaderPreset::resolve_or_default("does_not_exist"),
            CrossfaderPreset::ConstantPower
        );
        assert_eq!(preset_group_or_default("does_not_exist"), constant_power());
        assert_eq!(
            CrossfaderPreset::resolve_or_default("full"),
            CrossfaderPreset::ConstantPower
        );
    }

    #[test]
    fn curve_groups_pass_crossfader_endpoints() {
        for preset in [
            CrossfaderPreset::ConstantPower,
            CrossfaderPreset::Linear,
            CrossfaderPreset::FastCut { cut: 0.15 },
            CrossfaderPreset::FastCut { cut: 0.5 },
            CrossfaderPreset::FastCut { cut: 1.0 },
        ] {
            let group = preset.curve_group().expect("preset builds");
            assert_eq!(group.channels(), CROSSFADER_CHANNELS);
            validate_crossfader_group(&group).expect("valid endpoints");
        }
    }

    #[test]
    fn crossfader_validation_rejects_bad_endpoints_and_cardinality() {
        let swapped = CurveGroup::new(vec![linear_curves()[1].clone(), linear_curves()[0].clone()])
            .expect("two valid curves");
        assert!(matches!(
            validate_crossfader_group(&swapped),
            Err(CrossfaderCurveError::Endpoint {
                channel: 0,
                x: 0.0,
                ..
            })
        ));

        let scalar = CurveGroup::new(vec![linear_curves()[0].clone()]).expect("one curve");
        assert_eq!(
            validate_crossfader_group(&scalar),
            Err(CrossfaderCurveError::ChannelCount { count: 1 })
        );
    }

    #[test]
    fn group_refs_use_canonical_ids_and_parameters() {
        assert_eq!(
            CrossfaderPreset::ConstantPower.group_ref(),
            CurveGroupRef::preset(CONSTANT_POWER)
        );
        assert_eq!(
            CrossfaderPreset::FastCut { cut: 0.1 }.group_ref(),
            CurveGroupRef::preset_with_param(FAST_CUT, 0.1)
        );
        assert_eq!(
            CrossfaderPreset::ConstantPower.group_ref().id(),
            CONSTANT_POWER
        );
    }
}
