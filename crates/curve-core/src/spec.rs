use crate::{CurveError, CurvePoint, CurveTable, SegmentKind};

/// Minimum points in a validated curve.
pub const MIN_POINTS: usize = 2;
/// Maximum points in a validated curve.
pub const MAX_POINTS: usize = 256;

/// A piecewise normalized curve.
///
/// `points` has `2..=256` entries with strictly increasing `x` from `0` to `1`
/// and `y` in `0..=1`. `segments` holds exactly one [`SegmentKind`] for each
/// adjacent pair, so `segments.len() == points.len() - 1`.
///
/// Build with [`CurveSpec::new`] to validate up front. The fields stay public so
/// higher layers can serialize and edit a spec; call [`CurveSpec::validate`]
/// after mutating them directly.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveSpec {
    pub points: Vec<CurvePoint>,
    pub segments: Vec<SegmentKind>,
}

impl CurveSpec {
    /// Validate and construct a curve.
    pub fn new(points: Vec<CurvePoint>, segments: Vec<SegmentKind>) -> Result<Self, CurveError> {
        let spec = Self { points, segments };
        spec.validate()?;
        Ok(spec)
    }

    /// Validate point count, endpoints, ordering, values, and segments.
    pub fn validate(&self) -> Result<(), CurveError> {
        if !(MIN_POINTS..=MAX_POINTS).contains(&self.points.len()) {
            return Err(CurveError::PointCount {
                count: self.points.len(),
            });
        }
        let expected = self.points.len() - 1;
        if self.segments.len() != expected {
            return Err(CurveError::SegmentCount {
                points: self.points.len(),
                expected,
                count: self.segments.len(),
            });
        }

        for (index, point) in self.points.iter().enumerate() {
            point.validate(index)?;
        }

        let first = self.points[0];
        if first.x != 0.0 {
            return Err(CurveError::MissingStart { x: first.x });
        }
        let last = self.points[self.points.len() - 1];
        if last.x != 1.0 {
            return Err(CurveError::MissingEnd { x: last.x });
        }

        let mut previous = first.x;
        for (index, point) in self.points.iter().enumerate().skip(1) {
            if point.x <= previous {
                return Err(CurveError::NonIncreasingX {
                    index,
                    x: point.x,
                    previous,
                });
            }
            previous = point.x;
        }

        for (index, segment) in self.segments.iter().enumerate() {
            segment.validate(index)?;
        }
        Ok(())
    }

    /// Evaluate the curve at normalized `x`.
    ///
    /// `x` is clamped to `0..=1`; `NaN` reads as `0`. A validated spec always
    /// returns a value in `0..=1`. Invalid specs are evaluated defensively
    /// instead of panicking, but callers should validate first.
    pub fn evaluate(&self, x: f32) -> f32 {
        let Some(first) = self.points.first().copied() else {
            return 0.0;
        };
        if x.is_nan() {
            return first.y;
        }
        let x = x.clamp(0.0, 1.0);
        let Some(last) = self.points.last().copied() else {
            return first.y;
        };
        if x <= first.x {
            return first.y;
        }
        if x >= last.x {
            return last.y;
        }

        let next = self.points.partition_point(|point| point.x <= x);
        let index = next.saturating_sub(1).min(self.points.len() - 2);
        let start = self.points[index];
        let end = self.points[index + 1];
        let span = end.x - start.x;
        let t = if span > 0.0 {
            (x - start.x) / span
        } else {
            0.0
        };
        let segment = self
            .segments
            .get(index)
            .copied()
            .unwrap_or(SegmentKind::Linear);
        segment.evaluate(start.y, end.y, t)
    }

    /// Compile this curve into a [`CurveTable`].
    pub fn compile(&self) -> Result<CurveTable, CurveError> {
        CurveTable::compile(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5
    }

    fn points(raw: &[(f32, f32)]) -> Vec<CurvePoint> {
        raw.iter().map(|&(x, y)| CurvePoint::new(x, y)).collect()
    }

    #[test]
    fn linear_spec_is_the_identity() {
        let spec = CurveSpec::new(points(&[(0.0, 0.0), (1.0, 1.0)]), vec![SegmentKind::Linear])
            .expect("valid curve");
        for step in 0..=20 {
            let x = step as f32 / 20.0;
            assert!(close(spec.evaluate(x), x));
        }
        assert_eq!(spec.evaluate(-1.0), 0.0);
        assert_eq!(spec.evaluate(2.0), 1.0);
        assert_eq!(spec.evaluate(f32::NAN), 0.0);
    }

    #[test]
    fn mixed_segment_curve_is_continuous_at_points() {
        let spec = CurveSpec::new(
            points(&[(0.0, 0.0), (0.25, 0.8), (0.6, 0.9), (1.0, 1.0)]),
            vec![
                SegmentKind::Exponential { rate: 4.0 },
                SegmentKind::Linear,
                SegmentKind::Exponential { rate: 0.35 },
            ],
        )
        .expect("valid curve");

        assert_eq!(spec.evaluate(0.0), 0.0);
        assert!(close(spec.evaluate(0.25), 0.8));
        assert!(close(spec.evaluate(0.6), 0.9));
        assert_eq!(spec.evaluate(1.0), 1.0);

        let mut previous = 0.0;
        for step in 0..=1000 {
            let value = spec.evaluate(step as f32 / 1000.0);
            assert!((0.0..=1.0).contains(&value));
            assert!(value >= previous - 1e-5, "monotonic points must not dip");
            previous = value;
        }
    }

    #[test]
    fn exact_point_x_returns_its_y() {
        let spec = CurveSpec::new(
            points(&[(0.0, 0.0), (0.4, 0.2), (1.0, 1.0)]),
            vec![
                SegmentKind::CubicBezier {
                    c1: [0.3, 0.1],
                    c2: [0.7, 0.5],
                },
                SegmentKind::Logarithmic { rate: 2.0 },
            ],
        )
        .expect("valid curve");
        assert!(close(spec.evaluate(0.4), 0.2));
    }

    #[test]
    fn rejects_wrong_point_count() {
        assert!(matches!(
            CurveSpec::new(points(&[(0.0, 0.0)]), vec![]),
            Err(CurveError::PointCount { count: 1 })
        ));
        let many: Vec<CurvePoint> = (0..257)
            .map(|index| CurvePoint::new(index as f32 / 256.0, 0.0))
            .collect();
        assert!(matches!(
            CurveSpec::new(many, vec![SegmentKind::Linear; 256]),
            Err(CurveError::PointCount { count: 257 })
        ));
    }

    #[test]
    fn rejects_wrong_segment_count() {
        assert!(matches!(
            CurveSpec::new(
                points(&[(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)]),
                vec![SegmentKind::Linear]
            ),
            Err(CurveError::SegmentCount {
                points: 3,
                expected: 2,
                count: 1
            })
        ));
    }

    #[test]
    fn rejects_bad_endpoints_and_order() {
        assert!(matches!(
            CurveSpec::new(points(&[(0.1, 0.0), (1.0, 1.0)]), vec![SegmentKind::Linear]),
            Err(CurveError::MissingStart { .. })
        ));
        assert!(matches!(
            CurveSpec::new(points(&[(0.0, 0.0), (0.9, 1.0)]), vec![SegmentKind::Linear]),
            Err(CurveError::MissingEnd { .. })
        ));
        assert!(matches!(
            CurveSpec::new(
                points(&[(0.0, 0.0), (0.5, 0.5), (0.5, 0.6), (1.0, 1.0)]),
                vec![SegmentKind::Linear; 3]
            ),
            Err(CurveError::NonIncreasingX { index: 2, .. })
        ));
    }

    #[test]
    fn rejects_values_outside_the_normalized_domain() {
        assert!(matches!(
            CurveSpec::new(
                points(&[(0.0, -0.1), (1.0, 1.0)]),
                vec![SegmentKind::Linear]
            ),
            Err(CurveError::YOutOfRange { index: 0, .. })
        ));
    }

    #[test]
    fn invalid_specs_evaluate_without_panicking() {
        let empty = CurveSpec {
            points: Vec::new(),
            segments: Vec::new(),
        };
        assert_eq!(empty.evaluate(0.5), 0.0);

        let incomplete = CurveSpec {
            points: points(&[(0.0, 0.2), (1.0, 0.8)]),
            segments: Vec::new(),
        };
        assert!(close(incomplete.evaluate(0.5), 0.5));
    }
}
