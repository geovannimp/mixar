use crate::{CurveError, CurveSpec, CurveTable};

/// Maximum curves (outputs) a single group may hold.
pub const MAX_GROUP_CHANNELS: usize = 16;

/// One or more curves evaluated from the same normalized input.
///
/// A scalar control uses one curve; a two-channel control (for example the two
/// gains of a crossfader) uses two. Groups are validated for 1..=16 curves
/// here; each consumer validates its exact cardinality with
/// [`CurveGroup::expect_channels`].
#[derive(Clone, Debug, PartialEq)]
pub struct CurveGroup {
    pub curves: Vec<CurveSpec>,
}

impl CurveGroup {
    /// Validate and construct a group.
    pub fn new(curves: Vec<CurveSpec>) -> Result<Self, CurveError> {
        let group = Self { curves };
        group.validate()?;
        Ok(group)
    }

    /// A one-curve group.
    pub fn scalar(curve: CurveSpec) -> Result<Self, CurveError> {
        Self::new(vec![curve])
    }

    /// Validate cardinality and every curve in the group.
    pub fn validate(&self) -> Result<(), CurveError> {
        if self.curves.is_empty() || self.curves.len() > MAX_GROUP_CHANNELS {
            return Err(CurveError::GroupCardinality {
                count: self.curves.len(),
            });
        }
        for (index, curve) in self.curves.iter().enumerate() {
            curve.validate().map_err(|source| CurveError::Curve {
                index,
                source: Box::new(source),
            })?;
        }
        Ok(())
    }

    /// Require an exact output count (a scalar target expects 1, an A/B target
    /// expects 2).
    pub fn expect_channels(&self, expected: usize) -> Result<(), CurveError> {
        if self.curves.len() == expected {
            Ok(())
        } else {
            Err(CurveError::ChannelCount {
                expected,
                count: self.curves.len(),
            })
        }
    }

    /// Number of curves (outputs).
    pub fn channels(&self) -> usize {
        self.curves.len()
    }

    /// Evaluate one output at normalized `x`.
    pub fn evaluate_channel(&self, channel: usize, x: f32) -> Option<f32> {
        self.curves.get(channel).map(|curve| curve.evaluate(x))
    }

    /// Evaluate every output into `out`, which must match the channel count.
    pub fn evaluate_into(&self, x: f32, out: &mut [f32]) -> Result<(), CurveError> {
        if out.len() != self.curves.len() {
            return Err(CurveError::OutputLength {
                expected: self.curves.len(),
                actual: out.len(),
            });
        }
        for (slot, curve) in out.iter_mut().zip(&self.curves) {
            *slot = curve.evaluate(x);
        }
        Ok(())
    }

    /// Evaluate every output, allocating a result vector.
    pub fn evaluate(&self, x: f32) -> Vec<f32> {
        self.curves.iter().map(|curve| curve.evaluate(x)).collect()
    }

    /// Compile every curve into a table for realtime evaluation.
    pub fn compile(&self) -> Result<CompiledCurveGroup, CurveError> {
        CompiledCurveGroup::compile(self)
    }
}

/// A [`CurveGroup`] compiled to one [`CurveTable`] per output.
#[derive(Clone, Debug, PartialEq)]
pub struct CompiledCurveGroup {
    tables: Vec<CurveTable>,
}

impl CompiledCurveGroup {
    /// Validate `group` and compile all of its curves.
    pub fn compile(group: &CurveGroup) -> Result<Self, CurveError> {
        group.validate()?;
        let tables = group
            .curves
            .iter()
            .map(CurveTable::compile)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { tables })
    }

    /// Number of compiled outputs.
    pub fn channels(&self) -> usize {
        self.tables.len()
    }

    /// One compiled table by output index.
    pub fn table(&self, channel: usize) -> Option<&CurveTable> {
        self.tables.get(channel)
    }

    /// Evaluate one compiled output at normalized `x`.
    pub fn evaluate_channel(&self, channel: usize, x: f32) -> Option<f32> {
        self.tables.get(channel).map(|table| table.evaluate(x))
    }

    /// Evaluate every compiled output into `out`, which must match the channel
    /// count.
    pub fn evaluate_into(&self, x: f32, out: &mut [f32]) -> Result<(), CurveError> {
        if out.len() != self.tables.len() {
            return Err(CurveError::OutputLength {
                expected: self.tables.len(),
                actual: out.len(),
            });
        }
        for (slot, table) in out.iter_mut().zip(&self.tables) {
            *slot = table.evaluate(x);
        }
        Ok(())
    }

    /// Evaluate every compiled output, allocating a result vector.
    pub fn evaluate(&self, x: f32) -> Vec<f32> {
        self.tables.iter().map(|table| table.evaluate(x)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CurvePoint, SegmentKind};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-4
    }

    fn linear(y0: f32, y1: f32) -> CurveSpec {
        CurveSpec::new(
            vec![CurvePoint::new(0.0, y0), CurvePoint::new(1.0, y1)],
            vec![SegmentKind::Linear],
        )
        .expect("valid curve")
    }

    #[test]
    fn scalar_group_compiles_and_evaluates() {
        let group = CurveGroup::scalar(linear(0.0, 1.0)).expect("valid group");
        assert_eq!(group.channels(), 1);
        assert_eq!(group.expect_channels(1), Ok(()));

        let compiled = group.compile().expect("compiles");
        assert_eq!(compiled.channels(), 1);
        assert!(close(
            compiled.evaluate_channel(0, 0.25).expect("channel"),
            0.25
        ));
        assert_eq!(compiled.evaluate_channel(1, 0.25), None);
    }

    #[test]
    fn two_channel_group_compiles_and_evaluates() {
        let group = CurveGroup::new(vec![linear(1.0, 0.0), linear(0.0, 1.0)]).expect("valid group");
        assert_eq!(group.channels(), 2);
        assert_eq!(group.expect_channels(2), Ok(()));

        let mut direct = [0.0; 2];
        group.evaluate_into(0.25, &mut direct).expect("buffer");
        assert!(close(direct[0], 0.75));
        assert!(close(direct[1], 0.25));

        let compiled = group.compile().expect("compiles");
        let mut sampled = [0.0; 2];
        compiled.evaluate_into(0.25, &mut sampled).expect("buffer");
        assert!(close(sampled[0], 0.75));
        assert!(close(sampled[1], 0.25));
        assert_eq!(compiled.evaluate(0.5).len(), 2);
    }

    #[test]
    fn rejects_group_cardinality() {
        assert!(matches!(
            CurveGroup::new(Vec::new()),
            Err(CurveError::GroupCardinality { count: 0 })
        ));
        let curves = vec![linear(0.0, 1.0); MAX_GROUP_CHANNELS + 1];
        assert!(matches!(
            CurveGroup::new(curves),
            Err(CurveError::GroupCardinality { count: 17 })
        ));
    }

    #[test]
    fn rejects_wrong_channel_count_and_buffer() {
        let group = CurveGroup::scalar(linear(0.0, 1.0)).expect("valid group");
        assert!(matches!(
            group.expect_channels(2),
            Err(CurveError::ChannelCount {
                expected: 2,
                count: 1
            })
        ));
        assert!(matches!(
            group.evaluate_into(0.5, &mut [0.0; 2]),
            Err(CurveError::OutputLength {
                expected: 1,
                actual: 2
            })
        ));
    }

    #[test]
    fn reports_which_curve_is_invalid() {
        let invalid = CurveSpec {
            points: vec![CurvePoint::new(0.0, 0.0)],
            segments: Vec::new(),
        };
        match CurveGroup::new(vec![linear(0.0, 1.0), invalid]) {
            Err(CurveError::Curve { index, source }) => {
                assert_eq!(index, 1);
                assert!(matches!(*source, CurveError::PointCount { count: 1 }));
            }
            other => panic!("expected a curve error, got {other:?}"),
        }
    }
}
