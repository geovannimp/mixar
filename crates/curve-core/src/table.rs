use crate::{CurveError, CurveSpec};

/// Samples stored in a compiled [`CurveTable`] (`0` through `1` inclusive).
pub const TABLE_SAMPLES: usize = 2049;

/// A curve sampled at [`TABLE_SAMPLES`] evenly spaced positions.
///
/// Evaluation interpolates linearly between the two neighboring samples.
/// Compile on a control thread; the resulting table is immutable and cheap to
/// evaluate on a realtime thread.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveTable {
    samples: Vec<f32>,
}

impl CurveTable {
    /// Validate `spec` and sample it into a table.
    pub fn compile(spec: &CurveSpec) -> Result<Self, CurveError> {
        spec.validate()?;
        let last = (TABLE_SAMPLES - 1) as f32;
        let samples = (0..TABLE_SAMPLES)
            .map(|index| spec.evaluate(index as f32 / last))
            .collect();
        Ok(Self { samples })
    }

    /// Number of stored samples ([`TABLE_SAMPLES`]).
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Always `false`; present for the `len` lint convention.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The raw samples, ordered from `x = 0` to `x = 1`.
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// One sample by index, or `None` when out of range.
    pub fn sample(&self, index: usize) -> Option<f32> {
        self.samples.get(index).copied()
    }

    /// Evaluate at normalized `x` with linear interpolation.
    ///
    /// `x` is clamped to `0..=1`; `NaN` reads as `0`.
    pub fn evaluate(&self, x: f32) -> f32 {
        let Some(&first) = self.samples.first() else {
            return 0.0;
        };
        if x.is_nan() {
            return first;
        }
        let x = x.clamp(0.0, 1.0);
        let last_index = self.samples.len() - 1;
        let position = x * last_index as f32;
        let index = position as usize;
        if index >= last_index {
            return self.samples[last_index];
        }
        let fraction = position - index as f32;
        let start = self.samples[index];
        let end = self.samples[index + 1];
        start + (end - start) * fraction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CurvePoint, SegmentKind};

    fn close(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() <= tolerance
    }

    fn mixed_spec() -> CurveSpec {
        CurveSpec::new(
            vec![
                CurvePoint::new(0.0, 0.0),
                CurvePoint::new(0.25, 0.8),
                CurvePoint::new(0.6, 0.9),
                CurvePoint::new(1.0, 1.0),
            ],
            vec![
                SegmentKind::Exponential { rate: 4.0 },
                SegmentKind::Linear,
                SegmentKind::Exponential { rate: 0.35 },
            ],
        )
        .expect("valid curve")
    }

    #[test]
    fn compiles_the_documented_sample_count() {
        let table = CurveTable::compile(&mixed_spec()).expect("valid curve");
        assert_eq!(table.len(), TABLE_SAMPLES);
        assert_eq!(table.samples().len(), TABLE_SAMPLES);
        assert!(!table.is_empty());
    }

    #[test]
    fn table_matches_direct_evaluation() {
        let spec = mixed_spec();
        let table = CurveTable::compile(&spec).expect("valid curve");
        for step in 0..=10_000 {
            let x = step as f32 / 10_000.0;
            let direct = spec.evaluate(x);
            let sampled = table.evaluate(x);
            assert!(
                close(direct, sampled, 2.5e-3),
                "x={x}: direct={direct}, table={sampled}"
            );
        }
    }

    #[test]
    fn table_matches_direct_evaluation_for_every_segment_kind() {
        let spec = CurveSpec::new(
            vec![
                CurvePoint::new(0.0, 0.0),
                CurvePoint::new(0.3, 0.9),
                CurvePoint::new(0.6, 0.2),
                CurvePoint::new(0.8, 0.6),
                CurvePoint::new(1.0, 1.0),
            ],
            vec![
                SegmentKind::Linear,
                SegmentKind::Exponential { rate: -3.0 },
                SegmentKind::Logarithmic { rate: 3.0 },
                SegmentKind::CubicBezier {
                    c1: [0.25, 0.1],
                    c2: [0.75, 0.9],
                },
            ],
        )
        .expect("valid curve");
        let table = CurveTable::compile(&spec).expect("valid curve");
        for step in 0..=10_000 {
            let x = step as f32 / 10_000.0;
            assert!(close(spec.evaluate(x), table.evaluate(x), 2.5e-3), "x={x}");
        }
    }

    #[test]
    fn table_preserves_endpoints_exactly() {
        let spec = mixed_spec();
        let table = CurveTable::compile(&spec).expect("valid curve");
        assert_eq!(table.evaluate(0.0), 0.0);
        assert_eq!(table.evaluate(1.0), 1.0);
        assert_eq!(table.sample(0), Some(0.0));
        assert_eq!(table.sample(TABLE_SAMPLES - 1), Some(1.0));
        assert_eq!(table.sample(TABLE_SAMPLES), None);
    }

    #[test]
    fn table_evaluate_clamps_and_interpolates() {
        let spec = CurveSpec::new(
            vec![CurvePoint::new(0.0, 0.0), CurvePoint::new(1.0, 1.0)],
            vec![SegmentKind::Linear],
        )
        .expect("valid curve");
        let table = CurveTable::compile(&spec).expect("valid curve");
        assert_eq!(table.evaluate(-0.5), 0.0);
        assert_eq!(table.evaluate(1.5), 1.0);
        assert_eq!(table.evaluate(f32::NAN), 0.0);
        assert!(close(table.evaluate(0.5), 0.5, 1e-6));
        assert!(close(table.evaluate(0.123_456), 0.123_456, 1e-4));
    }

    #[test]
    fn compile_rejects_invalid_specs() {
        let invalid = CurveSpec {
            points: vec![CurvePoint::new(0.0, 0.0)],
            segments: Vec::new(),
        };
        assert!(matches!(
            CurveTable::compile(&invalid),
            Err(CurveError::PointCount { count: 1 })
        ));
    }
}
