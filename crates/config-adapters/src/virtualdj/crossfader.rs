//! VirtualDJ crossfader options: import into native curves and export back.

use std::collections::BTreeMap;

use audio_core::{constant_power_group, CrossfaderCurveBehavior, CROSSFADER_CURVE_CHANNELS};
use curve_core::{CurveGroup, CurveGroupRef, CurvePoint, CurveRefKind, CurveSpec, SegmentKind};

use super::points::{curves_from_points, format_points, parse_points, CrossfaderPoint};
use super::{unquote, VirtualDjError};

/// VirtualDJ option key for the crossfader curve.
pub const CROSSFADER_CURVE_KEY: &str = "crossfaderCurve";
/// VirtualDJ option key for the custom crossfader point script.
pub const CROSSFADER_CUSTOM_KEY: &str = "crossfaderCustom";
/// VirtualDJ option key for the A/B swap.
pub const CROSSFADER_HAMSTER_KEY: &str = "crossfaderHamster";
/// VirtualDJ option key for the crossfader bypass.
pub const CROSSFADER_DISABLE_KEY: &str = "crossfaderDisable";

/// Native id of the equal-power crossfader preset.
///
/// Must match `engine-core`'s preset registry so a migrated settings value
/// resolves to the same curve.
pub const CONSTANT_POWER_PRESET: &str = "constant_power";
/// Native id of the linear crossfader preset.
pub const LINEAR_PRESET: &str = "linear";
/// Native id of the parameterized fast-cut crossfader preset.
pub const FAST_CUT_PRESET: &str = "fast_cut";

/// Fade length used for VirtualDJ's `Cut` preset.
pub const CUT_FADE: f32 = 0.15;
/// Fade length used for VirtualDJ's `Scratch` preset.
pub const SCRATCH_FADE: f32 = 0.05;
/// Fade length used for VirtualDJ's `Full` preset.
///
/// A true full cut has a zero-length fade, which the native `fast_cut` preset
/// cannot express, so `Full` is approximated with a 1% fade.
pub const FULL_FADE: f32 = 0.01;

/// Curve-group id under which [`CurveGroupRef::custom`] points at curves
/// imported from `crossfaderCustom`.
///
/// The caller installs [`CrossfaderConfig::group`] in its curve library under
/// this id.
pub const CUSTOM_GROUP_ID: &str = "virtualdj_custom";

/// VirtualDJ preset name for the equal-power curve.
pub const SMOOTH_CURVE: &str = "Smooth";
/// VirtualDJ preset name for the linear curve.
pub const DEFAULT_CURVE: &str = "Default";
/// VirtualDJ preset name for a 15% fast cut.
pub const CUT_CURVE: &str = "Cut";
/// VirtualDJ preset name for a 5% fast cut.
pub const SCRATCH_CURVE: &str = "Scratch";
/// VirtualDJ preset name for an (approximate) hard cut.
pub const FULL_CURVE: &str = "Full";
/// VirtualDJ curve name that bypasses the crossfader.
pub const DISABLED_CURVE: &str = "Disabled";
/// VirtualDJ curve name that selects the point script.
pub const CUSTOM_CURVE: &str = "Custom";

/// VirtualDJ compares a fade length against the named presets with this
/// tolerance, so hand-written parameters close to a preset still export by
/// name.
const FADE_TOLERANCE: f32 = 1e-6;

/// The four VirtualDJ crossfader options.
///
/// Build it from a `settings.xml`-derived map with [`Self::from_options`] or
/// field by field; missing options default to "not set".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VirtualDjCrossfaderConfig {
    /// `crossfaderCurve`: a preset name, `custom`, or a `0..=1` amount.
    pub crossfader_curve: String,
    /// `crossfaderCustom`: the `x=[a,b]/…` point script.
    pub crossfader_custom: String,
    /// `crossfaderHamster`: whether deck A and deck B are swapped.
    pub crossfader_hamster: bool,
    /// `crossfaderDisable`: whether the curve is bypassed.
    pub crossfader_disable: bool,
}

impl VirtualDjCrossfaderConfig {
    /// Read the four crossfader options out of a `settings.xml`-derived map.
    ///
    /// Absent keys keep their defaults (empty curve and script, flags off).
    /// Surrounding quotes are stripped, boolean values accept `yes`/`no`,
    /// `on`/`off`, `true`/`false`, and `1`/`0` (case-insensitive), and an
    /// unknown boolean is rejected. Unknown keys are ignored so a whole
    /// settings map can be passed in.
    pub fn from_options<I, K, V>(options: I) -> Result<Self, VirtualDjError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let options: BTreeMap<String, String> = options
            .into_iter()
            .map(|(key, value)| (key.as_ref().to_string(), value.as_ref().to_string()))
            .collect();

        Ok(Self {
            crossfader_curve: option_value(&options, CROSSFADER_CURVE_KEY),
            crossfader_custom: option_value(&options, CROSSFADER_CUSTOM_KEY),
            crossfader_hamster: option_bool(&options, CROSSFADER_HAMSTER_KEY)?,
            crossfader_disable: option_bool(&options, CROSSFADER_DISABLE_KEY)?,
        })
    }

    /// Write the four options back as a VirtualDJ settings map.
    pub fn to_options(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                CROSSFADER_CURVE_KEY.to_string(),
                self.crossfader_curve.clone(),
            ),
            (
                CROSSFADER_CUSTOM_KEY.to_string(),
                self.crossfader_custom.clone(),
            ),
            (
                CROSSFADER_HAMSTER_KEY.to_string(),
                yes_no(self.crossfader_hamster),
            ),
            (
                CROSSFADER_DISABLE_KEY.to_string(),
                yes_no(self.crossfader_disable),
            ),
        ])
    }
}

/// A crossfader migrated from VirtualDJ into native types.
///
/// `group` holds the two-channel curve (channel 0 is deck A, channel 1 is deck
/// B). `group_ref` is the stable reference to store in settings: a preset id,
/// or [`CUSTOM_GROUP_ID`] when the curve came from `crossfaderCustom` — install
/// `group` in the curve library under that id.
#[derive(Clone, Debug, PartialEq)]
pub struct CrossfaderConfig {
    /// The resolved two-channel curve, ready to compile.
    pub group: CurveGroup,
    /// The reference to persist in settings.
    pub group_ref: CurveGroupRef,
    /// Hamster and bypass flags.
    pub behavior: CrossfaderCurveBehavior,
}

/// Import VirtualDJ crossfader options into native curves and flags.
///
/// A non-empty `crossfaderCustom` is used when `crossfaderCurve` is empty or
/// `custom`; when a preset or amount is selected, a stale point script is
/// ignored, as VirtualDJ does. `crossfaderCurve = Disabled` selects the default
/// curve with the bypass flag set.
pub fn import_crossfader(
    config: &VirtualDjCrossfaderConfig,
) -> Result<CrossfaderConfig, VirtualDjError> {
    let (group, group_ref, curve_disabled) =
        import_curve(&config.crossfader_curve, &config.crossfader_custom)?;
    Ok(CrossfaderConfig {
        group,
        group_ref,
        behavior: CrossfaderCurveBehavior {
            disabled: config.crossfader_disable || curve_disabled,
            hamster: config.crossfader_hamster,
        },
    })
}

/// Import straight from a `settings.xml`-derived options map.
pub fn import_crossfader_options(
    options: &BTreeMap<String, String>,
) -> Result<CrossfaderConfig, VirtualDjError> {
    import_crossfader(&VirtualDjCrossfaderConfig::from_options(options)?)
}

/// Export a native crossfader configuration as VirtualDJ options.
///
/// Named native presets map back to their VirtualDJ names. Any other curve —
/// a custom group, an arbitrary `fast_cut` parameter, or a preset VirtualDJ
/// does not know — is written as a custom point script sampled at the union of
/// the A and B breakpoints. Straight-line segments survive the roundtrip;
/// non-linear segments are flattened and therefore lossy.
pub fn export_crossfader(
    config: &CrossfaderConfig,
) -> Result<VirtualDjCrossfaderConfig, VirtualDjError> {
    config.group.expect_channels(CROSSFADER_CURVE_CHANNELS)?;
    let (crossfader_curve, crossfader_custom) = export_curve(&config.group_ref, &config.group)?;
    Ok(VirtualDjCrossfaderConfig {
        crossfader_curve,
        crossfader_custom,
        crossfader_hamster: config.behavior.hamster,
        crossfader_disable: config.behavior.disabled,
    })
}

fn import_curve(
    curve: &str,
    custom: &str,
) -> Result<(CurveGroup, CurveGroupRef, bool), VirtualDjError> {
    let name = unquote(curve);
    if name.is_empty() {
        if custom.trim().is_empty() {
            return Ok((
                constant_power_group(),
                CurveGroupRef::preset(CONSTANT_POWER_PRESET),
                false,
            ));
        }
        return Ok((
            custom_group(custom)?,
            CurveGroupRef::custom(CUSTOM_GROUP_ID),
            false,
        ));
    }

    if name.eq_ignore_ascii_case(CUSTOM_CURVE) {
        if custom.trim().is_empty() {
            return Err(VirtualDjError::MissingCustomCurve);
        }
        return Ok((
            custom_group(custom)?,
            CurveGroupRef::custom(CUSTOM_GROUP_ID),
            false,
        ));
    }
    if name.eq_ignore_ascii_case(DISABLED_CURVE) {
        return Ok((
            constant_power_group(),
            CurveGroupRef::preset(CONSTANT_POWER_PRESET),
            true,
        ));
    }
    if name.eq_ignore_ascii_case(SMOOTH_CURVE) {
        return Ok((
            constant_power_group(),
            CurveGroupRef::preset(CONSTANT_POWER_PRESET),
            false,
        ));
    }
    if name.eq_ignore_ascii_case(DEFAULT_CURVE) || name.eq_ignore_ascii_case(LINEAR_PRESET) {
        return Ok((linear_group()?, CurveGroupRef::preset(LINEAR_PRESET), false));
    }
    for (preset, fade) in [
        (CUT_CURVE, CUT_FADE),
        (SCRATCH_CURVE, SCRATCH_FADE),
        (FULL_CURVE, FULL_FADE),
    ] {
        if name.eq_ignore_ascii_case(preset) {
            return Ok((
                fast_cut_group(fade)?,
                CurveGroupRef::preset_with_param(FAST_CUT_PRESET, fade),
                false,
            ));
        }
    }

    let amount = parse_amount(&name)?;
    let (group, reference) = amount_curve(amount)?;
    Ok((group, reference, false))
}

fn export_curve(
    reference: &CurveGroupRef,
    group: &CurveGroup,
) -> Result<(String, String), VirtualDjError> {
    if let CurveRefKind::Preset { id, param } = &reference.kind {
        if param.is_none() && id.eq_ignore_ascii_case(CONSTANT_POWER_PRESET) {
            return Ok((SMOOTH_CURVE.to_string(), String::new()));
        }
        if param.is_none() && id.eq_ignore_ascii_case(LINEAR_PRESET) {
            return Ok((DEFAULT_CURVE.to_string(), String::new()));
        }
        if id.eq_ignore_ascii_case(FAST_CUT_PRESET) {
            let fade = param.unwrap_or(CUT_FADE);
            let preset = if approx(fade, CUT_FADE) {
                Some(CUT_CURVE)
            } else if approx(fade, SCRATCH_FADE) {
                Some(SCRATCH_CURVE)
            } else if approx(fade, FULL_FADE) {
                Some(FULL_CURVE)
            } else {
                None
            };
            if let Some(preset) = preset {
                return Ok((preset.to_string(), String::new()));
            }
        }
    }

    let points = sample_points(group)?;
    Ok((CUSTOM_CURVE.to_string(), format_points(&points)))
}

/// Evaluate both channels at the union of the A and B breakpoints.
fn sample_points(group: &CurveGroup) -> Result<Vec<CrossfaderPoint>, VirtualDjError> {
    group.expect_channels(CROSSFADER_CURVE_CHANNELS)?;
    let mut positions: Vec<f32> = group
        .curves
        .iter()
        .flat_map(|curve| curve.points.iter().map(|point| point.x))
        .collect();
    positions.sort_by(f32::total_cmp);
    positions.dedup();

    let a = &group.curves[0];
    let b = &group.curves[1];
    Ok(positions
        .into_iter()
        .map(|x| {
            CrossfaderPoint::new(
                x,
                a.evaluate(x).clamp(0.0, 1.0),
                b.evaluate(x).clamp(0.0, 1.0),
            )
        })
        .collect())
}

/// Map a VirtualDJ amount to the native fast-cut preset.
///
/// `amount` is the fraction of travel at full gain, so the fade length is
/// `1 - amount`. `0` is the linear law and `1` is a full cut, which the native
/// preset can only approximate.
fn amount_curve(amount: f32) -> Result<(CurveGroup, CurveGroupRef), VirtualDjError> {
    if amount == 0.0 {
        return Ok((linear_group()?, CurveGroupRef::preset(LINEAR_PRESET)));
    }
    if amount >= 1.0 {
        return Ok((
            fast_cut_group(FULL_FADE)?,
            CurveGroupRef::preset_with_param(FAST_CUT_PRESET, FULL_FADE),
        ));
    }
    let fade = 1.0 - amount;
    Ok((
        fast_cut_group(fade)?,
        CurveGroupRef::preset_with_param(FAST_CUT_PRESET, fade),
    ))
}

fn parse_amount(value: &str) -> Result<f32, VirtualDjError> {
    let text = value.trim();
    let (number, divisor) = match text.strip_suffix('%') {
        Some(percent) => (percent.trim(), 100.0),
        None => (text, 1.0),
    };
    let parsed = number
        .parse::<f32>()
        .map_err(|_| VirtualDjError::InvalidCurve {
            value: value.to_string(),
        })?;
    let amount = parsed / divisor;
    if !amount.is_finite() || !(0.0..=1.0).contains(&amount) {
        return Err(VirtualDjError::AmountOutOfRange { value: amount });
    }
    Ok(amount)
}

fn custom_group(script: &str) -> Result<CurveGroup, VirtualDjError> {
    let points = parse_points(script)?;
    let (a, b) = curves_from_points(&points)?;
    crossfader_group(vec![a, b])
}

fn linear_group() -> Result<CurveGroup, VirtualDjError> {
    let a = CurveSpec::new(
        vec![CurvePoint::new(0.0, 1.0), CurvePoint::new(1.0, 0.0)],
        vec![SegmentKind::Linear],
    )?;
    let b = CurveSpec::new(
        vec![CurvePoint::new(0.0, 0.0), CurvePoint::new(1.0, 1.0)],
        vec![SegmentKind::Linear],
    )?;
    crossfader_group(vec![a, b])
}

/// Build the native `fast_cut(cut)` preset shape.
///
/// Mirrors `engine-core`'s registry: deck A stays at full gain until
/// `x = 1 - cut` and then fades to zero; deck B is the mirror image. Keeping
/// the two in sync is what makes the `fast_cut` reference resolvable.
fn fast_cut_group(cut: f32) -> Result<CurveGroup, VirtualDjError> {
    if !cut.is_finite() || cut <= 0.0 || cut > 1.0 {
        return Err(VirtualDjError::FadeOutOfRange { value: cut });
    }
    crossfader_group(vec![
        fast_cut_curve(cut, true)?,
        fast_cut_curve(cut, false)?,
    ])
}

fn fast_cut_curve(cut: f32, deck_a: bool) -> Result<CurveSpec, VirtualDjError> {
    let mut positions = vec![0.0, cut, 1.0 - cut, 1.0];
    positions.sort_by(f32::total_cmp);
    positions.dedup();

    let points: Vec<CurvePoint> = positions
        .into_iter()
        .map(|x| {
            let y = if deck_a {
                ((1.0 - x) / cut).clamp(0.0, 1.0)
            } else {
                (x / cut).clamp(0.0, 1.0)
            };
            CurvePoint::new(x, y)
        })
        .collect();
    let segments = vec![SegmentKind::Linear; points.len() - 1];
    Ok(CurveSpec::new(points, segments)?)
}

fn crossfader_group(curves: Vec<CurveSpec>) -> Result<CurveGroup, VirtualDjError> {
    let group = CurveGroup::new(curves)?;
    group.expect_channels(CROSSFADER_CURVE_CHANNELS)?;
    Ok(group)
}

fn approx(left: f32, right: f32) -> bool {
    (left - right).abs() <= FADE_TOLERANCE
}

fn option_value(options: &BTreeMap<String, String>, key: &str) -> String {
    options
        .get(key)
        .map(|value| unquote(value))
        .unwrap_or_default()
}

fn option_bool(options: &BTreeMap<String, String>, key: &str) -> Result<bool, VirtualDjError> {
    match options.get(key) {
        Some(value) => parse_bool(key, &unquote(value)),
        None => Ok(false),
    }
}

fn parse_bool(key: &str, value: &str) -> Result<bool, VirtualDjError> {
    match value.to_ascii_lowercase().as_str() {
        "" | "no" | "off" | "false" | "0" => Ok(false),
        "yes" | "on" | "true" | "1" => Ok(true),
        _ => Err(VirtualDjError::InvalidBoolean {
            key: key.to_string(),
            value: value.to_string(),
        }),
    }
}

fn yes_no(value: bool) -> String {
    if value { "yes" } else { "no" }.to_string()
}
