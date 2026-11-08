//! FX level/depth curve: a normalized control position bound to a `0..1` wet
//! amount through a [`CurveBinding`].
//!
//! The knob travels `0..1`; the installed curve maps that position to the
//! wet/dry mix the chain applies. The default is the identity curve, so a
//! session that never selects a curve keeps the historical linear response.
//! Shapes come from `curve-core`; this module owns no curve math.

use curve_core::{
    presets, AxisMap, CurveBinding, CurveError, CurveGroup, CurveGroupRef, ResolvedCurveGroup,
};

/// An FX level control drives exactly one curve output.
pub const FX_LEVEL_CHANNELS: usize = 1;

/// The identity FX level curve (`wet = position`).
///
/// This is the default and reproduces the historical linear knob response.
pub fn linear_fx_level_curve() -> ResolvedCurveGroup {
    let group = CurveGroup::scalar(presets::linear())
        .expect("the linear shape preset is always a valid scalar group");
    ResolvedCurveGroup::single(&group).expect("the linear scalar curve always compiles")
}

/// A `0..1` FX level position bound to a `0..1` wet/dry output.
#[derive(Clone, Debug, PartialEq)]
pub struct FxLevelCurve {
    curve: ResolvedCurveGroup,
    binding: CurveBinding,
}

impl FxLevelCurve {
    /// The default identity curve.
    pub fn linear() -> Self {
        Self::new(linear_fx_level_curve()).expect("the linear FX level curve is a scalar curve")
    }

    /// Bind `curve` to the unit `0..1` level range.
    ///
    /// The curve must produce exactly one output.
    pub fn new(curve: ResolvedCurveGroup) -> Result<Self, CurveError> {
        if curve.channels() != FX_LEVEL_CHANNELS {
            return Err(CurveError::ChannelCount {
                expected: FX_LEVEL_CHANNELS,
                count: curve.channels(),
            });
        }
        Ok(Self {
            curve,
            binding: CurveBinding::new(
                CurveGroupRef::preset(presets::LINEAR),
                AxisMap::UNIT,
                AxisMap::UNIT,
            )
            .expect("a valid preset reference and unit axes form a valid binding"),
        })
    }

    /// The installed curve group.
    pub fn curve(&self) -> &ResolvedCurveGroup {
        &self.curve
    }

    /// The normalized-position-to-wet binding.
    pub fn binding(&self) -> &CurveBinding {
        &self.binding
    }

    /// Replace the curve, keeping the current control position.
    pub fn set_curve(&mut self, curve: ResolvedCurveGroup) -> Result<(), CurveError> {
        if curve.channels() != FX_LEVEL_CHANNELS {
            return Err(CurveError::ChannelCount {
                expected: FX_LEVEL_CHANNELS,
                count: curve.channels(),
            });
        }
        self.curve = curve;
        Ok(())
    }

    /// Wet/dry output at control position `position` (`0..1`).
    ///
    /// The position is clamped before evaluation; a non-finite position reads
    /// as dry rather than panicking inside the audio callback.
    pub fn level_at(&self, position: f32) -> f32 {
        let position = if position.is_finite() {
            position.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let x = self.binding.normalized_input(position);
        let mut out = [0.0f32; FX_LEVEL_CHANNELS];
        match self.curve.evaluate_into(x, &mut out) {
            Ok(()) => self.binding.denormalized_output(out[0]).clamp(0.0, 1.0),
            // The channel count is checked on install; fall back to the raw
            // position rather than panicking inside a callback.
            Err(_) => x,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curve_core::{CurvePoint, CurveSpec, SegmentKind};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5
    }

    fn curve(points: &[(f32, f32)]) -> ResolvedCurveGroup {
        let spec = CurveSpec::new(
            points
                .iter()
                .map(|(x, y)| CurvePoint::new(*x, *y))
                .collect(),
            vec![SegmentKind::Linear; points.len() - 1],
        )
        .expect("valid test curve");
        let group = CurveGroup::scalar(spec).expect("scalar test curve");
        ResolvedCurveGroup::single(&group).expect("test curve compiles")
    }

    #[test]
    fn default_linear_curve_preserves_the_historical_response() {
        let level = FxLevelCurve::linear();
        for position in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0] {
            assert!(
                close(level.level_at(position), position),
                "position {position}: {} != {position}",
                level.level_at(position)
            );
        }
    }

    #[test]
    fn custom_exponential_curve_bends_the_response() {
        let spec = presets::exponential(4.0).expect("exponential curve");
        let group = CurveGroup::scalar(spec).expect("scalar curve");
        let level = FxLevelCurve::new(
            ResolvedCurveGroup::single(&group).expect("compiled exponential curve"),
        )
        .expect("one channel");

        assert!(close(level.level_at(0.0), 0.0), "curve starts dry");
        assert!(close(level.level_at(1.0), 1.0), "curve ends fully wet");
        assert!(
            level.level_at(0.5) < 0.25,
            "exponential taper stays low: {}",
            level.level_at(0.5)
        );
    }

    #[test]
    fn custom_logarithmic_curve_lifts_the_response() {
        let spec = presets::logarithmic(4.0).expect("logarithmic curve");
        let group = CurveGroup::scalar(spec).expect("scalar curve");
        let level = FxLevelCurve::new(
            ResolvedCurveGroup::single(&group).expect("compiled logarithmic curve"),
        )
        .expect("one channel");

        assert!(close(level.level_at(0.0), 0.0));
        assert!(close(level.level_at(1.0), 1.0));
        assert!(
            level.level_at(0.5) > 0.75,
            "logarithmic taper lifts early: {}",
            level.level_at(0.5)
        );
    }

    #[test]
    fn custom_curve_shape_changes_without_reinstalling_the_slot() {
        let mut level = FxLevelCurve::linear();
        assert!(close(level.level_at(0.5), 0.5));

        level
            .set_curve(curve(&[(0.0, 0.0), (0.5, 0.25), (1.0, 1.0)]))
            .expect("scalar curve");
        assert!(close(level.level_at(0.5), 0.25));
    }

    #[test]
    fn multi_channel_groups_are_rejected() {
        let group =
            CurveGroup::new(vec![presets::linear(), presets::linear()]).expect("two-channel group");
        let resolved = ResolvedCurveGroup::single(&group).expect("compiled group");
        let error = FxLevelCurve::new(resolved).unwrap_err();
        assert_eq!(
            error,
            CurveError::ChannelCount {
                expected: FX_LEVEL_CHANNELS,
                count: 2,
            }
        );
    }

    #[test]
    fn positions_outside_the_unit_range_are_clamped() {
        let level = FxLevelCurve::linear();
        assert!(close(level.level_at(-0.5), 0.0));
        assert!(close(level.level_at(2.0), 1.0));
        assert!(close(level.level_at(f32::NAN), 0.0));
    }
}
