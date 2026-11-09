use crate::CurveError;

/// How real values are distributed across a normalized axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisScale {
    /// Uniform spacing; signed ranges such as `-8..8` put zero at `0.5`.
    Linear,
    /// Logarithmic spacing; both bounds must be positive.
    Logarithmic,
}

/// Maps a real input or output range onto the normalized `0..=1` curve domain.
///
/// A tempo control can use a signed linear range (`-8..8`), an FX level a
/// `0..1` range, and a filter frequency a logarithmic range (`20..20000`). The
/// curve itself always stays normalized; the map is what gives it real units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisMap {
    pub min: f32,
    pub max: f32,
    pub scale: AxisScale,
}

impl AxisMap {
    /// The normalized `0..=1` linear axis.
    pub const UNIT: Self = Self {
        min: 0.0,
        max: 1.0,
        scale: AxisScale::Linear,
    };

    /// Validate and construct an axis map.
    pub fn new(min: f32, max: f32, scale: AxisScale) -> Result<Self, CurveError> {
        let map = Self { min, max, scale };
        map.validate()?;
        Ok(map)
    }

    /// A uniform axis.
    pub fn linear(min: f32, max: f32) -> Result<Self, CurveError> {
        Self::new(min, max, AxisScale::Linear)
    }

    /// A logarithmic axis; `min` must be positive.
    pub fn logarithmic(min: f32, max: f32) -> Result<Self, CurveError> {
        Self::new(min, max, AxisScale::Logarithmic)
    }

    /// Bounds must be finite with `min < max`; a logarithmic axis also needs
    /// positive bounds.
    pub fn validate(&self) -> Result<(), CurveError> {
        if !self.min.is_finite() || !self.max.is_finite() || self.min >= self.max {
            return Err(CurveError::AxisBounds {
                min: self.min,
                max: self.max,
            });
        }
        if self.scale == AxisScale::Logarithmic && self.min <= 0.0 {
            return Err(CurveError::LogarithmicAxisBounds {
                min: self.min,
                max: self.max,
            });
        }
        Ok(())
    }

    /// Width of the real range.
    pub fn range(&self) -> f32 {
        self.max - self.min
    }

    /// Map a real value into `0..=1`, clamping values outside the range.
    pub fn normalize(&self, value: f32) -> f32 {
        let normalized = match self.scale {
            AxisScale::Linear => (value - self.min) / self.range(),
            AxisScale::Logarithmic => {
                if value <= self.min {
                    0.0
                } else if value >= self.max {
                    1.0
                } else {
                    (value / self.min).ln() / (self.max / self.min).ln()
                }
            }
        };
        if normalized.is_nan() {
            0.0
        } else {
            normalized.clamp(0.0, 1.0)
        }
    }

    /// Map a normalized `0..=1` value back into the real range.
    pub fn denormalize(&self, normalized: f32) -> f32 {
        let t = if normalized.is_nan() {
            0.0
        } else {
            normalized.clamp(0.0, 1.0)
        };
        match self.scale {
            AxisScale::Linear => self.min + t * self.range(),
            AxisScale::Logarithmic => self.min * (self.max / self.min).powf(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-4 * b.abs().max(1.0)
    }

    #[test]
    fn unit_axis_is_the_identity() {
        assert_eq!(AxisMap::UNIT.normalize(0.25), 0.25);
        assert_eq!(AxisMap::UNIT.denormalize(0.25), 0.25);
        assert_eq!(AxisMap::UNIT.normalize(-1.0), 0.0);
        assert_eq!(AxisMap::UNIT.normalize(2.0), 1.0);
        assert_eq!(AxisMap::UNIT.normalize(f32::NAN), 0.0);
    }

    #[test]
    fn signed_tempo_range_anchors_zero_at_the_middle() {
        let tempo = AxisMap::linear(-8.0, 8.0).expect("valid axis");
        assert_eq!(tempo.normalize(-8.0), 0.0);
        assert_eq!(tempo.normalize(0.0), 0.5);
        assert_eq!(tempo.normalize(8.0), 1.0);
        assert!(close(tempo.denormalize(0.5), 0.0));
        for value in [-8.0, -4.0, 0.0, 2.0, 8.0] {
            assert!(close(tempo.denormalize(tempo.normalize(value)), value));
        }
    }

    #[test]
    fn logarithmic_range_uses_geometric_midpoint() {
        let filter = AxisMap::logarithmic(20.0, 20_000.0).expect("valid axis");
        assert!(close(filter.normalize(20.0), 0.0));
        assert!(close(filter.normalize(20_000.0), 1.0));
        assert!(close(filter.normalize(632.455_5), 0.5));
        for value in [20.0, 200.0, 632.455_5, 2_000.0, 20_000.0] {
            assert!(close(filter.denormalize(filter.normalize(value)), value));
        }
        assert_eq!(filter.normalize(0.0), 0.0);
        assert_eq!(filter.normalize(-5.0), 0.0);
    }

    #[test]
    fn rejects_invalid_bounds() {
        assert!(matches!(
            AxisMap::linear(f32::NAN, 1.0),
            Err(CurveError::AxisBounds { .. })
        ));
        assert!(matches!(
            AxisMap::linear(1.0, 1.0),
            Err(CurveError::AxisBounds { .. })
        ));
        assert!(matches!(
            AxisMap::linear(2.0, 1.0),
            Err(CurveError::AxisBounds { .. })
        ));
        assert!(matches!(
            AxisMap::logarithmic(0.0, 100.0),
            Err(CurveError::LogarithmicAxisBounds { .. })
        ));
        assert!(matches!(
            AxisMap::logarithmic(-10.0, 100.0),
            Err(CurveError::LogarithmicAxisBounds { .. })
        ));
    }
}
