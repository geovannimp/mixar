//! VirtualDJ settings migration.
//!
//! VirtualDJ stores its options in a `settings.xml` file. The crossfader lives
//! in four options:
//!
//! | option | meaning |
//! |--------|---------|
//! | `crossfaderCurve` | preset name, `custom`, or a `0..=1` amount |
//! | `crossfaderCustom` | point script used when the curve is `custom` |
//! | `crossfaderHamster` | swap deck A and deck B (`yes` / `no`) |
//! | `crossfaderDisable` | bypass the curve (`yes` / `no`) |
//!
//! [`VirtualDjCrossfaderConfig`] mirrors those four options and can be built
//! from any `settings.xml`-derived key/value map with
//! [`VirtualDjCrossfaderConfig::from_options`]. [`import_crossfader`] maps them
//! to the native two-channel [`CurveGroup`](curve_core::CurveGroup), its
//! [`CurveGroupRef`](curve_core::CurveGroupRef), and the
//! [`CrossfaderCurveBehavior`](audio_core::CrossfaderCurveBehavior) flags; [`export_crossfader`] writes them back.
//!
//! ```
//! use config_adapters::virtualdj::{
//!     export_crossfader, import_crossfader, VirtualDjCrossfaderConfig,
//! };
//!
//! let config = VirtualDjCrossfaderConfig {
//!     crossfader_curve: "0.3".to_string(),
//!     crossfader_hamster: true,
//!     ..VirtualDjCrossfaderConfig::default()
//! };
//!
//! let native = import_crossfader(&config)?;
//! assert_eq!(native.group_ref.id(), "fast_cut");
//! assert!(native.behavior.hamster);
//!
//! let exported = export_crossfader(&native)?;
//! assert_eq!(exported.crossfader_curve, "Custom");
//! assert!(!exported.crossfader_custom.is_empty());
//! # Ok::<(), config_adapters::VirtualDjError>(())
//! ```
//!
//! The crate never runs in the audio path: it is called while a settings
//! migration is prepared, and the resolved [`CurveGroup`](curve_core::CurveGroup) is installed by the
//! caller like any other curve.
//!
//! # Preset mapping
//!
//! VirtualDJ's preset geometries are not public, so this adapter pins a fixed,
//! documented mapping. The native ids match `engine-core`'s crossfader preset
//! registry (`constant_power`, `linear`, `fast_cut`).
//!
//! | VirtualDJ | native | notes |
//! |-----------|--------|-------|
//! | `Smooth` | `constant_power` | equal-power law |
//! | `Default`, `Linear` | `linear` | `A = 1 - x`, `B = x` |
//! | `Cut` | `fast_cut(0.15)` | fades over the last/first 15% |
//! | `Scratch` | `fast_cut(0.05)` | fades over the last/first 5% |
//! | `Full` | `fast_cut(0.01)` | approximate hard cut |
//! | `Disabled` | `constant_power` + `disabled` | curve bypassed |
//! | amount `v` | `fast_cut(1 - v)` | exact VirtualDJ amount law |
//! | `Custom` | `virtualdj_custom` group | parsed from `crossfaderCustom` |
//!
//! A numeric `crossfaderCurve` is the fraction of the travel during which the
//! outgoing deck stays at full gain, exactly the `fast_cut` fade length
//! `cut = 1 - v`. `0` is the linear law and `1` is a full cut (imported as the
//! `Full` approximation, because the native preset rejects a zero-length fade).
//! Percent values such as `30%` are accepted too.
//!
//! # Custom point syntax
//!
//! The `crossfaderCustom` script lists `position=[gainA,gainB]` points, with
//! optional `/` separators and whitespace anywhere:
//!
//! ```text
//! 0=[1,0] / 0.25=[0.5,0.5] / 1=[0,1]
//! 0=[1,0]0.25=[0.5,0.5]1=[0,1]
//! ```
//!
//! Both forms parse to the same points. Import requires a point at `x=0` and
//! one at `x=1`, strictly increasing positions, and gains in `0..=1`; anything
//! else is rejected with a [`VirtualDjError`].
//!
//! # Export lossiness
//!
//! A curve that matches a named VirtualDJ preset exports as that preset name.
//! Everything else exports as a custom point script sampled at the **union of
//! the A and B x-points**, so the two decks keep their independent breakpoints.
//! Points are joined with straight lines: piecewise-linear native curves
//! (including `fast_cut`) survive the roundtrip unchanged, while non-linear
//! segments (equal-power Hermite fits, Bézier handles, exponential/logarithmic
//! tapers) are flattened to their sampled values and are lossy.

mod crossfader;
mod points;

pub use crossfader::{
    export_crossfader, import_crossfader, import_crossfader_options, CrossfaderConfig,
    VirtualDjCrossfaderConfig, CONSTANT_POWER_PRESET, CROSSFADER_CURVE_KEY, CROSSFADER_CUSTOM_KEY,
    CROSSFADER_DISABLE_KEY, CROSSFADER_HAMSTER_KEY, CUSTOM_CURVE, CUSTOM_GROUP_ID, CUT_CURVE,
    CUT_FADE, DEFAULT_CURVE, DISABLED_CURVE, FAST_CUT_PRESET, FULL_CURVE, FULL_FADE, LINEAR_PRESET,
    SCRATCH_CURVE, SCRATCH_FADE, SMOOTH_CURVE,
};
pub use points::{format_points, parse_points, CrossfaderPoint};

use curve_core::CurveError;
use thiserror::Error;

/// A VirtualDJ option that could not be imported or exported.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum VirtualDjError {
    #[error(
        "VirtualDJ option `{key}` must be a boolean (`yes`/`no`, `on`/`off`, `true`/`false`, `1`/`0`), got `{value}`"
    )]
    InvalidBoolean { key: String, value: String },

    #[error(
        "VirtualDJ `crossfaderCurve` value `{value}` is not a preset name, a 0..=1 amount, or `custom`"
    )]
    InvalidCurve { value: String },

    #[error("VirtualDJ crossfader amount {value} is outside 0..=1")]
    AmountOutOfRange { value: f32 },

    #[error("VirtualDJ `crossfaderCurve` is `custom` but `crossfaderCustom` is empty")]
    MissingCustomCurve,

    #[error("malformed VirtualDJ crossfader point near `{near}`: {reason}")]
    MalformedPoint { near: String, reason: &'static str },

    #[error("VirtualDJ crossfader position x={x} is outside 0..=1")]
    PositionOutOfRange { x: f32 },

    #[error("VirtualDJ crossfader gain {value} is outside 0..=1")]
    GainOutOfRange { value: f32 },

    #[error("VirtualDJ `crossfaderCustom` needs at least two points, got {count}")]
    TooFewPoints { count: usize },

    #[error("VirtualDJ `crossfaderCustom` must start at x=0, got x={x}")]
    MissingStart { x: f32 },

    #[error("VirtualDJ `crossfaderCustom` must end at x=1, got x={x}")]
    MissingEnd { x: f32 },

    #[error("VirtualDJ crossfader point x={x} does not follow x={previous} in increasing order")]
    NonIncreasingPosition { x: f32, previous: f32 },

    #[error("VirtualDJ crossfader fade length {value} must be in (0, 1]")]
    FadeOutOfRange { value: f32 },

    #[error(transparent)]
    Curve(#[from] CurveError),
}

/// Strip one pair of surrounding quotes and surrounding whitespace.
///
/// VirtualDJ scripts often quote option values (`crossfaderCurve="Cut"`); the
/// quotes are not part of the value.
pub(super) fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        })
        .unwrap_or(trimmed);
    unquoted.trim().to_string()
}

#[cfg(test)]
mod tests;
