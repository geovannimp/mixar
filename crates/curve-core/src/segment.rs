use crate::CurveError;

/// Largest absolute [`SegmentKind::Exponential`] / [`SegmentKind::Logarithmic`]
/// rate accepted by validation.
pub const MAX_SEGMENT_RATE: f32 = 20.0;

/// Rates below this magnitude are treated as linear to avoid the `0 / 0`
/// limit of the exponential taper.
const RATE_EPSILON: f32 = 1e-4;

/// Interpolation rule for the span between two adjacent [`CurvePoint`]s.
///
/// Every shape is normalized so `f(0) = 0` and `f(1) = 1`; the segment then
/// applies that shape to the span between its endpoint `y` values.
///
/// [`CurvePoint`]: crate::CurvePoint
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SegmentKind {
    /// Straight line between the endpoints.
    Linear,
    /// Exponential taper. Positive rates stay near the first point and rise
    /// late; negative rates rise early and flatten out.
    Exponential { rate: f32 },
    /// Logarithmic taper, the inverse shape of [`SegmentKind::Exponential`].
    Logarithmic { rate: f32 },
    /// CSS-like cubic Bézier with control points in `0..=1`.
    ///
    /// `c1` and `c2` are `[x, y]` handles; the implicit endpoints are `(0, 0)`
    /// and `(1, 1)`. `x` handles must be in `0..=1` so the curve stays a
    /// function of the input.
    CubicBezier { c1: [f32; 2], c2: [f32; 2] },
}

impl SegmentKind {
    pub(crate) fn validate(&self, index: usize) -> Result<(), CurveError> {
        match *self {
            Self::Linear => Ok(()),
            Self::Exponential { rate } | Self::Logarithmic { rate } => {
                if !rate.is_finite() || rate.abs() > MAX_SEGMENT_RATE {
                    return Err(CurveError::RateOutOfRange { index, rate });
                }
                Ok(())
            }
            Self::CubicBezier { c1, c2 } => {
                for point in [c1, c2] {
                    if !point[0].is_finite()
                        || !point[1].is_finite()
                        || !(0.0..=1.0).contains(&point[0])
                        || !(0.0..=1.0).contains(&point[1])
                    {
                        return Err(CurveError::BezierOutOfRange {
                            index,
                            x: point[0],
                            y: point[1],
                        });
                    }
                }
                Ok(())
            }
        }
    }

    /// Normalized shape at local position `t` in `0..=1`.
    ///
    /// The result is exactly `0` at `t = 0` and `1` at `t = 1` for every
    /// variant, so consecutive segments meet without a jump.
    pub fn shape(&self, t: f32) -> f32 {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        if t <= 0.0 {
            return 0.0;
        }
        if t >= 1.0 {
            return 1.0;
        }
        match *self {
            Self::Linear => t,
            Self::Exponential { rate } => exponential_shape(rate, t),
            Self::Logarithmic { rate } => logarithmic_shape(rate, t),
            Self::CubicBezier { c1, c2 } => bezier_shape(t, c1, c2),
        }
    }

    /// Interpolate between `y0` and `y1` at local position `t`.
    pub fn evaluate(&self, y0: f32, y1: f32, t: f32) -> f32 {
        y0 + (y1 - y0) * self.shape(t)
    }
}

fn exponential_shape(rate: f32, t: f32) -> f32 {
    if !rate.is_finite() || rate.abs() < RATE_EPSILON {
        return t;
    }
    let denominator = rate.exp() - 1.0;
    if denominator == 0.0 || !denominator.is_finite() {
        return t;
    }
    ((rate * t).exp() - 1.0) / denominator
}

fn logarithmic_shape(rate: f32, t: f32) -> f32 {
    if !rate.is_finite() || rate.abs() < RATE_EPSILON {
        return t;
    }
    let factor = rate.exp() - 1.0;
    if !factor.is_finite() {
        return t;
    }
    let inner = 1.0 + factor * t;
    if inner <= 0.0 {
        return t;
    }
    (inner.ln() / rate).clamp(0.0, 1.0)
}

fn bezier_shape(t: f32, c1: [f32; 2], c2: [f32; 2]) -> f32 {
    // The x handles are validated into `0..=1`, which makes x(u) monotonic, so
    // a fixed-iteration bisection is deterministic and cannot diverge.
    let mut low = 0.0;
    let mut high = 1.0;
    let mut u = t;
    for _ in 0..32 {
        let x = bezier_axis(u, c1[0], c2[0]);
        if (x - t).abs() < 1e-6 {
            break;
        }
        if x < t {
            low = u;
        } else {
            high = u;
        }
        u = 0.5 * (low + high);
    }
    bezier_axis(u, c1[1], c2[1])
}

fn bezier_axis(u: f32, p1: f32, p2: f32) -> f32 {
    let one_minus = 1.0 - u;
    3.0 * one_minus * one_minus * u * p1 + 3.0 * one_minus * u * u * p2 + u * u * u
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5
    }

    #[test]
    fn linear_shape_is_the_identity() {
        let segment = SegmentKind::Linear;
        assert_eq!(segment.shape(0.0), 0.0);
        assert_eq!(segment.shape(1.0), 1.0);
        assert!(close(segment.shape(0.25), 0.25));
        assert!(close(segment.shape(0.75), 0.75));
    }

    #[test]
    fn exponential_has_exact_endpoints_and_expected_curvature() {
        let segment = SegmentKind::Exponential { rate: 4.0 };
        assert_eq!(segment.shape(0.0), 0.0);
        assert_eq!(segment.shape(1.0), 1.0);
        assert!(segment.shape(0.5) < 0.5, "positive rate rises late");

        let reversed = SegmentKind::Exponential { rate: -4.0 };
        assert!(reversed.shape(0.5) > 0.5, "negative rate rises early");

        let mut previous = 0.0;
        for step in 0..=100 {
            let value = segment.shape(step as f32 / 100.0);
            assert!(value >= previous, "exponential must be monotonic");
            previous = value;
        }
    }

    #[test]
    fn near_zero_rate_falls_back_to_linear() {
        for rate in [0.0, RATE_EPSILON / 2.0, -RATE_EPSILON / 2.0] {
            let segment = SegmentKind::Exponential { rate };
            assert!(close(segment.shape(0.3), 0.3));
            assert!(close(segment.shape(0.7), 0.7));
        }
    }

    #[test]
    fn logarithmic_inverts_exponential() {
        let exponential = SegmentKind::Exponential { rate: 4.0 };
        let logarithmic = SegmentKind::Logarithmic { rate: 4.0 };
        assert_eq!(logarithmic.shape(0.0), 0.0);
        assert_eq!(logarithmic.shape(1.0), 1.0);
        assert!(logarithmic.shape(0.5) > 0.5, "log rises early");
        for step in 0..=20 {
            let t = step as f32 / 20.0;
            assert!(
                close(logarithmic.shape(exponential.shape(t)), t),
                "log(exp(t)) must equal t at t={t}"
            );
        }
    }

    #[test]
    fn cubic_bezier_diagonal_controls_are_linear() {
        let segment = SegmentKind::CubicBezier {
            c1: [1.0 / 3.0, 1.0 / 3.0],
            c2: [2.0 / 3.0, 2.0 / 3.0],
        };
        for step in 0..=20 {
            let t = step as f32 / 20.0;
            assert!(close(segment.shape(t), t), "diagonal controls at t={t}");
        }
    }

    #[test]
    fn cubic_bezier_symmetric_controls_are_symmetric() {
        let segment = SegmentKind::CubicBezier {
            c1: [0.42, 0.0],
            c2: [0.58, 1.0],
        };
        assert_eq!(segment.shape(0.0), 0.0);
        assert_eq!(segment.shape(1.0), 1.0);
        assert!(close(segment.shape(0.5), 0.5));
        for step in 0..=10 {
            let t = step as f32 / 10.0;
            assert!(close(segment.shape(t) + segment.shape(1.0 - t), 1.0));
        }
    }

    #[test]
    fn segment_evaluate_maps_the_y_span() {
        let segment = SegmentKind::Linear;
        assert!(close(segment.evaluate(0.2, 0.8, 0.5), 0.5));
        assert!(close(segment.evaluate(0.8, 0.2, 0.25), 0.65));
    }

    #[test]
    fn validation_rejects_bad_rates_and_bezier_handles() {
        assert!(matches!(
            SegmentKind::Exponential { rate: 21.0 }.validate(0),
            Err(CurveError::RateOutOfRange { index: 0, .. })
        ));
        assert!(matches!(
            SegmentKind::Logarithmic { rate: f32::NAN }.validate(1),
            Err(CurveError::RateOutOfRange { index: 1, .. })
        ));
        assert!(matches!(
            SegmentKind::CubicBezier {
                c1: [-0.1, 0.0],
                c2: [0.5, 0.5],
            }
            .validate(2),
            Err(CurveError::BezierOutOfRange { index: 2, .. })
        ));
        assert!(matches!(
            SegmentKind::CubicBezier {
                c1: [0.2, 0.2],
                c2: [0.5, 1.5],
            }
            .validate(3),
            Err(CurveError::BezierOutOfRange { index: 3, .. })
        ));
    }
}
