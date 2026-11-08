use crate::CurveError;

/// One point on a curve, in the normalized domain.
///
/// `x` is the input position and `y` the output value; both live in `0..=1`.
/// The first point has `x = 0`, the last `x = 1`, and `x` is strictly
/// increasing across the list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurvePoint {
    pub x: f32,
    pub y: f32,
}

impl CurvePoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub(crate) fn validate(&self, index: usize) -> Result<(), CurveError> {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(CurveError::NonFinitePoint {
                index,
                x: self.x,
                y: self.y,
            });
        }
        if !(0.0..=1.0).contains(&self.x) {
            return Err(CurveError::XOutOfRange { index, x: self.x });
        }
        if !(0.0..=1.0).contains(&self.y) {
            return Err(CurveError::YOutOfRange { index, y: self.y });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normalized_points() {
        assert_eq!(CurvePoint::new(0.0, 0.0).validate(0), Ok(()));
        assert_eq!(CurvePoint::new(1.0, 0.5).validate(3), Ok(()));
    }

    #[test]
    fn rejects_non_finite_points() {
        assert!(matches!(
            CurvePoint::new(f32::NAN, 0.0).validate(0),
            Err(CurveError::NonFinitePoint { index: 0, .. })
        ));
    }

    #[test]
    fn rejects_out_of_range_coordinates() {
        assert!(matches!(
            CurvePoint::new(-0.1, 0.0).validate(1),
            Err(CurveError::XOutOfRange { index: 1, .. })
        ));
        assert!(matches!(
            CurvePoint::new(0.5, 1.1).validate(2),
            Err(CurveError::YOutOfRange { index: 2, .. })
        ));
    }
}
