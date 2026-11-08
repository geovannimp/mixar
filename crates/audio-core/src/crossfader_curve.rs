//! Resolved crossfader curves shared by the mixer and performance history.
//!
//! A [`ResolvedCrossfaderCurve`] owns the compiled curve tables for the two
//! crossfader gains plus the behavior flags that modify them. Control code
//! resolves ids, compiles the tables, and installs the result; the audio
//! callback only evaluates it.

use curve_core::{
    CompiledCurveGroup, CurveError, CurveGroup, CurvePoint, CurveSpec, ResolvedCurveGroup,
    SegmentKind,
};

/// A crossfader curve group has one output for deck A and one for deck B.
pub const CROSSFADER_CURVE_CHANNELS: usize = 2;

/// Crossfader behavior flags applied after the curve is evaluated.
///
/// `Thru` lane assignments are independent of these flags: a `Thru` lane always
/// bypasses the crossfader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrossfaderCurveBehavior {
    /// Bypass the curve: both sides stay at full gain at every position.
    pub disabled: bool,
    /// Swap the A and B gains (hamster). The position is not mirrored.
    pub hamster: bool,
}

impl CrossfaderCurveBehavior {
    /// The default behavior: curve active, no swap.
    pub const NORMAL: Self = Self {
        disabled: false,
        hamster: false,
    };

    /// Curve bypassed, both sides at full gain.
    pub fn disabled() -> Self {
        Self {
            disabled: true,
            hamster: false,
        }
    }

    /// A and B gains swapped, curve active.
    pub fn hamster() -> Self {
        Self {
            disabled: false,
            hamster: true,
        }
    }

    /// No flag set.
    pub fn is_normal(self) -> bool {
        self == Self::NORMAL
    }
}

/// A resolved crossfader gain source installed in the mixer and history.
///
/// `gains` holds the compiled two-channel tables (one member for a single
/// curve, several for a curve set) and the morph position across them. The
/// default is the equal-power `constant_power` law.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedCrossfaderCurve {
    /// Compiled gain tables, one two-channel group per curve-set member.
    pub gains: ResolvedCurveGroup,
    /// Post-curve behavior flags.
    pub behavior: CrossfaderCurveBehavior,
}

impl ResolvedCrossfaderCurve {
    /// Compile one two-channel curve group.
    pub fn from_group(group: CurveGroup) -> Result<Self, CurveError> {
        Self::from_compiled(vec![CompiledCurveGroup::compile(&group)?])
    }

    /// Compile every member of a two-channel curve set.
    pub fn from_groups(groups: &[CurveGroup]) -> Result<Self, CurveError> {
        let members = groups
            .iter()
            .map(CompiledCurveGroup::compile)
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_compiled(members)
    }

    /// Wrap already compiled curve-set members.
    pub fn from_compiled(members: Vec<CompiledCurveGroup>) -> Result<Self, CurveError> {
        let gains = ResolvedCurveGroup::from_compiled(members)?;
        if gains.channels() != CROSSFADER_CURVE_CHANNELS {
            return Err(CurveError::ChannelCount {
                expected: CROSSFADER_CURVE_CHANNELS,
                count: gains.channels(),
            });
        }
        Ok(Self {
            gains,
            behavior: CrossfaderCurveBehavior::NORMAL,
        })
    }

    /// The default equal-power crossfader curve.
    pub fn constant_power() -> Self {
        Self::from_group(constant_power_group())
            .expect("the constant-power crossfader group is always valid")
    }

    /// The compiled gain tables.
    pub fn gains(&self) -> &ResolvedCurveGroup {
        &self.gains
    }

    /// Current morph position across the curve set.
    pub fn morph(&self) -> f32 {
        self.gains.morph()
    }

    /// Set the curve-set morph position, clamped to `0..=1`.
    pub fn set_morph(&mut self, morph: f32) {
        self.gains.set_morph(morph);
    }

    /// Current behavior flags.
    pub fn behavior(&self) -> CrossfaderCurveBehavior {
        self.behavior
    }

    /// Replace the behavior flags.
    pub fn set_behavior(&mut self, behavior: CrossfaderCurveBehavior) {
        self.behavior = behavior;
    }

    /// Builder form of [`Self::set_behavior`].
    pub fn with_behavior(mut self, behavior: CrossfaderCurveBehavior) -> Self {
        self.set_behavior(behavior);
        self
    }

    /// Per-block A/B gains for a normalized crossfader `position`.
    ///
    /// Evaluation reads the compiled tables and returns a plain pair; there is
    /// no parsing, allocation, or fallible work on the audio thread.
    pub fn gains_at(&self, position: f32) -> (f32, f32) {
        let mut out = [0.0f32; CROSSFADER_CURVE_CHANNELS];
        if self.gains.evaluate_into(position, &mut out).is_err() {
            // The channel-count invariant is checked at construction; keep a
            // benign fallback rather than panicking inside a callback.
            return (1.0, 1.0);
        }

        let [mut gain_a, mut gain_b] = out;
        if self.behavior.disabled {
            return (1.0, 1.0);
        }
        if self.behavior.hamster {
            std::mem::swap(&mut gain_a, &mut gain_b);
        }
        (gain_a, gain_b)
    }
}

impl Default for ResolvedCrossfaderCurve {
    fn default() -> Self {
        Self::constant_power()
    }
}

/// The equal-power crossfader group: `(cos(x * π/2), sin(x * π/2))`.
pub fn constant_power_group() -> CurveGroup {
    CurveGroup::new(constant_power_curves().to_vec())
        .expect("the constant-power crossfader group is always valid")
}

/// Number of cubic Hermite segments in the equal-power fit.
const CONSTANT_POWER_SEGMENTS: usize = 64;

/// Build the equal-power curves as cubic Hermite segments.
///
/// A cubic Hermite fit of `cos`/`sin` keeps the error below f32 rounding while
/// every segment's control handles stay inside `curve_core`'s normalized range.
fn constant_power_curves() -> [CurveSpec; 2] {
    [
        hermite_curve(
            |x| (x * std::f32::consts::FRAC_PI_2).cos(),
            |x| -(x * std::f32::consts::FRAC_PI_2).sin() * std::f32::consts::FRAC_PI_2,
            1.0,
            0.0,
        ),
        hermite_curve(
            |x| (x * std::f32::consts::FRAC_PI_2).sin(),
            |x| (x * std::f32::consts::FRAC_PI_2).cos() * std::f32::consts::FRAC_PI_2,
            0.0,
            1.0,
        ),
    ]
}

fn hermite_curve(
    value: impl Fn(f32) -> f32,
    derivative: impl Fn(f32) -> f32,
    start: f32,
    end: f32,
) -> CurveSpec {
    let steps = CONSTANT_POWER_SEGMENTS;
    let mut points = Vec::with_capacity(steps + 1);
    for index in 0..=steps {
        let x = index as f32 / steps as f32;
        let y = match index {
            0 => start,
            index if index == steps => end,
            _ => value(x).clamp(0.0, 1.0),
        };
        points.push(CurvePoint::new(x, y));
    }

    let mut segments = Vec::with_capacity(steps);
    for index in 0..steps {
        let start_point = points[index];
        let end_point = points[index + 1];
        let span = end_point.x - start_point.x;
        let rise = end_point.y - start_point.y;
        if span <= 0.0 || rise.abs() < f32::EPSILON {
            segments.push(SegmentKind::Linear);
            continue;
        }
        // Normalize the endpoints' slopes into the segment's local 0..=1 shape.
        let slope_start = derivative(start_point.x) * span / rise;
        let slope_end = derivative(end_point.x) * span / rise;
        segments.push(SegmentKind::CubicBezier {
            c1: [1.0 / 3.0, (slope_start / 3.0).clamp(0.0, 1.0)],
            c2: [2.0 / 3.0, (1.0 - slope_end / 3.0).clamp(0.0, 1.0)],
        });
    }

    CurveSpec::new(points, segments)
        .expect("the constant-power Hermite curve is always a valid normalized curve")
}

#[cfg(test)]
mod tests {
    use super::*;
    use curve_core::CurvePoint;

    fn linear(y0: f32, y1: f32) -> CurveSpec {
        CurveSpec::new(
            vec![CurvePoint::new(0.0, y0), CurvePoint::new(1.0, y1)],
            vec![SegmentKind::Linear],
        )
        .expect("valid curve")
    }

    fn linear_group() -> CurveGroup {
        CurveGroup::new(vec![linear(1.0, 0.0), linear(0.0, 1.0)]).expect("valid group")
    }

    #[test]
    fn constant_power_endpoints_match_the_legacy_law() {
        let curve = ResolvedCrossfaderCurve::constant_power();
        let (a, b) = curve.gains_at(0.0);
        assert!((a - 1.0).abs() < 1e-6);
        assert!(b.abs() < 1e-6);

        let (a, b) = curve.gains_at(1.0);
        assert!(a.abs() < 1e-6);
        assert!((b - 1.0).abs() < 1e-6);

        let (a, b) = curve.gains_at(0.5);
        assert!((a - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-4);
        assert!((a - b).abs() < 1e-6);

        for step in 0..=100 {
            let x = step as f32 / 100.0;
            let angle = x * std::f32::consts::FRAC_PI_2;
            let (a, b) = curve.gains_at(x);
            assert!((a - angle.cos()).abs() < 1e-4, "A mismatch at {x}");
            assert!((b - angle.sin()).abs() < 1e-4, "B mismatch at {x}");
        }
    }

    #[test]
    fn custom_curve_replaces_the_constant_power_gains() {
        let curve = ResolvedCrossfaderCurve::from_group(linear_group()).expect("resolves");
        let (a, b) = curve.gains_at(0.25);
        assert!((a - 0.75).abs() < 1e-4);
        assert!((b - 0.25).abs() < 1e-4);
    }

    #[test]
    fn hamster_swaps_gains_without_mirroring_position() {
        let curve = ResolvedCrossfaderCurve::constant_power()
            .with_behavior(CrossfaderCurveBehavior::hamster());
        let (a, b) = curve.gains_at(0.0);
        assert!((a - 0.0).abs() < 1e-6);
        assert!((b - 1.0).abs() < 1e-6);

        let (a, b) = curve.gains_at(0.25);
        let angle = 0.25 * std::f32::consts::FRAC_PI_2;
        assert!((a - angle.sin()).abs() < 1e-4);
        assert!((b - angle.cos()).abs() < 1e-4);
    }

    #[test]
    fn disabled_bypasses_the_curve() {
        let curve = ResolvedCrossfaderCurve::from_group(linear_group())
            .expect("resolves")
            .with_behavior(CrossfaderCurveBehavior::disabled());
        assert_eq!(curve.gains_at(0.0), (1.0, 1.0));
        assert_eq!(curve.gains_at(0.75), (1.0, 1.0));
    }

    #[test]
    fn curve_set_morph_blends_members_at_runtime() {
        let curve = ResolvedCrossfaderCurve::from_groups(&[linear_group(), linear_group()])
            .expect("curve set");
        let (a, b) = curve.gains_at(0.5);
        assert!((a - 0.5).abs() < 1e-4);
        assert!((b - 0.5).abs() < 1e-4);
    }

    #[test]
    fn rejects_scalar_groups() {
        let scalar = CurveGroup::scalar(linear(0.0, 1.0)).expect("scalar");
        assert_eq!(
            ResolvedCrossfaderCurve::from_group(scalar),
            Err(CurveError::ChannelCount {
                expected: 2,
                count: 1,
            })
        );
    }
}
