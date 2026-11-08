//! Generic two-point shape presets.
//!
//! These are the building blocks shared by every configurable control. They
//! only describe shapes; target-specific preset registries (for example
//! crossfader presets) live in the layer that owns that target.

use crate::{CurveError, CurvePoint, CurveSpec, SegmentKind};

/// Id of the linear shape preset.
pub const LINEAR: &str = "linear";
/// Id of the exponential shape preset.
pub const EXPONENTIAL: &str = "exponential";
/// Id of the logarithmic shape preset.
pub const LOGARITHMIC: &str = "logarithmic";
/// Id of the cubic Bézier shape preset.
pub const CUBIC_BEZIER: &str = "cubic_bezier";

/// Every generic shape preset id.
pub const PRESET_IDS: [&str; 4] = [LINEAR, EXPONENTIAL, LOGARITHMIC, CUBIC_BEZIER];

/// Whether `id` names a generic shape preset.
///
/// Custom group ids may not use these names.
pub fn is_preset_id(id: &str) -> bool {
    PRESET_IDS.contains(&id)
}

/// The identity curve `y = x`.
pub fn linear() -> CurveSpec {
    two_point(SegmentKind::Linear).expect("the linear shape preset is always valid")
}

/// An exponential taper between `(0, 0)` and `(1, 1)`.
///
/// Positive rates stay low and rise late; negative rates rise early and
/// flatten out. The rate must be finite with `|rate| <= 20`; near zero the
/// segment behaves linearly.
pub fn exponential(rate: f32) -> Result<CurveSpec, CurveError> {
    two_point(SegmentKind::Exponential { rate })
}

/// A logarithmic taper between `(0, 0)` and `(1, 1)`.
///
/// This is the inverse shape of [`exponential`] with the same rate: an
/// exponential ramp followed by a logarithmic ramp of equal rate reconstructs
/// the input. The rate must be finite with `|rate| <= 20`.
pub fn logarithmic(rate: f32) -> Result<CurveSpec, CurveError> {
    two_point(SegmentKind::Logarithmic { rate })
}

/// A CSS-like cubic Bézier between `(0, 0)` and `(1, 1)`.
///
/// `c1` and `c2` are `[x, y]` handles and every coordinate must be in `0..=1`.
/// Diagonal handles at `[1/3, 1/3]` and `[2/3, 2/3]` reproduce [`linear`].
pub fn cubic_bezier(c1: [f32; 2], c2: [f32; 2]) -> Result<CurveSpec, CurveError> {
    two_point(SegmentKind::CubicBezier { c1, c2 })
}

fn two_point(segment: SegmentKind) -> Result<CurveSpec, CurveError> {
    CurveSpec::new(
        vec![CurvePoint::new(0.0, 0.0), CurvePoint::new(1.0, 1.0)],
        vec![segment],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5
    }

    #[test]
    fn linear_is_the_identity() {
        let curve = linear();
        for step in 0..=10 {
            let x = step as f32 / 10.0;
            assert!(close(curve.evaluate(x), x));
        }
    }

    #[test]
    fn exponential_and_logarithmic_are_inverse_shapes() {
        let exp = exponential(4.0).expect("valid rate");
        let log = logarithmic(4.0).expect("valid rate");
        assert!(exp.evaluate(0.5) < 0.5);
        assert!(log.evaluate(0.5) > 0.5);
        assert_eq!(exp.evaluate(0.0), 0.0);
        assert_eq!(exp.evaluate(1.0), 1.0);
        assert_eq!(log.evaluate(0.0), 0.0);
        assert_eq!(log.evaluate(1.0), 1.0);
    }

    #[test]
    fn bezier_preset_builds_a_valid_curve() {
        let curve = cubic_bezier([0.42, 0.0], [0.58, 1.0]).expect("valid handles");
        assert!(close(curve.evaluate(0.5), 0.5));

        let identity =
            cubic_bezier([1.0 / 3.0, 1.0 / 3.0], [2.0 / 3.0, 2.0 / 3.0]).expect("valid handles");
        assert!(close(identity.evaluate(0.25), 0.25));
    }

    #[test]
    fn presets_reject_invalid_parameters() {
        assert!(matches!(
            exponential(21.0),
            Err(CurveError::RateOutOfRange { .. })
        ));
        assert!(matches!(
            logarithmic(f32::NAN),
            Err(CurveError::RateOutOfRange { .. })
        ));
        assert!(matches!(
            cubic_bezier([0.0, -0.1], [1.0, 1.0]),
            Err(CurveError::BezierOutOfRange { .. })
        ));
    }

    #[test]
    fn preset_ids_are_recognized() {
        for id in PRESET_IDS {
            assert!(is_preset_id(id));
        }
        assert!(!is_preset_id("constant_power"));
    }
}
