//! Tempo fader `0..1` ↔ playback ratio using ±`tempo_range` (pitch fraction).
//!
//! The fader position runs through a [`CurveBinding`]: the input is the
//! normalized `0..1` position and the output is a signed pitch range
//! (`-tempo_range..=tempo_range`). The default curve is the historical linear
//! mapping (`position 0` = faster, `0.5` = unity, `1` = slower); custom curves
//! shape each half but never move the center detent.

use curve_core::{
    presets, AxisMap, CurveBinding, CurveError, CurveGroup, CurveGroupRef, CurvePoint, CurveSpec,
    ResolvedCurveGroup, SegmentKind,
};

/// A tempo fader drives exactly one curve output.
pub const TEMPO_CURVE_CHANNELS: usize = 1;

/// Default half-span: ±6% (Mixxx / DDJ-400 first step).
pub const DEFAULT_TEMPO_RANGE: f32 = 0.06;

/// Pioneer-style cycle steps (fraction of rate).
pub const TEMPO_RANGE_STEPS: &[f32] = &[0.06, 0.10, 0.16, 0.25];

/// Smallest span a [`CurveBinding`] output axis can represent.
const MIN_OUTPUT_SPAN: f32 = 1e-6;

/// Next step after `current` in [`TEMPO_RANGE_STEPS`] (wraps). Unknown → first step.
pub fn next_tempo_range(current: f32) -> f32 {
    const EPS: f32 = 1e-4;
    if let Some(i) = TEMPO_RANGE_STEPS
        .iter()
        .position(|s| (*s - current).abs() < EPS)
    {
        return TEMPO_RANGE_STEPS[(i + 1) % TEMPO_RANGE_STEPS.len()];
    }
    TEMPO_RANGE_STEPS[0]
}

/// Position `0` = +range (faster), `0.5` = 1.0, `1` = −range (slower).
///
/// This is the default linear tempo curve, kept as a free function for callers
/// that only need the historical mapping.
pub fn norm_to_playback_ratio(norm: f32, tempo_range: f32) -> f32 {
    let range = f64::from(tempo_range.max(0.0));
    let n = f64::from(norm.clamp(0.0, 1.0));
    (1.0 + (0.5 - n) * 2.0 * range).max(0.01) as f32
}

/// Saturates outside ±tempo_range.
pub fn playback_ratio_to_norm(ratio: f32, tempo_range: f32) -> f32 {
    let range = f64::from(tempo_range.max(1e-6));
    let n = 0.5 - (f64::from(ratio) - 1.0) / (2.0 * range);
    n.clamp(0.0, 1.0) as f32
}

/// The default tempo curve: `y = 1 - x`, the historical fader mapping.
pub fn linear_tempo_curve() -> ResolvedCurveGroup {
    let spec = CurveSpec::new(
        vec![CurvePoint::new(0.0, 1.0), CurvePoint::new(1.0, 0.0)],
        vec![SegmentKind::Linear],
    )
    .expect("the linear tempo curve is always valid");
    let group = CurveGroup::scalar(spec).expect("the linear tempo curve is a scalar group");
    ResolvedCurveGroup::single(&group).expect("the linear tempo curve always compiles")
}

/// A tempo fader curve bound to a signed pitch range.
///
/// The binding always maps the normalized `0..1` position to
/// `-range..=range`. A custom curve's center is recentered onto zero and each
/// half is rescaled so the endpoints still reach exactly `±range`: the center
/// detent is therefore independent of the selected curve.
#[derive(Clone, Debug, PartialEq)]
pub struct TempoCurve {
    curve: ResolvedCurveGroup,
    binding: CurveBinding,
    range: f32,
    /// Curve outputs at `x = 0`, `0.5`, and `1`, used to pin the center detent
    /// and both endpoints while keeping the selected shape.
    endpoints: [f32; 3],
}

impl TempoCurve {
    /// The default linear curve at `range`.
    pub fn linear(range: f32) -> Self {
        Self::new(linear_tempo_curve(), range).expect("the linear tempo curve is a scalar curve")
    }

    /// Bind `curve` to signed `±range` pitch.
    ///
    /// The curve must produce exactly one output.
    pub fn new(curve: ResolvedCurveGroup, range: f32) -> Result<Self, CurveError> {
        if curve.channels() != TEMPO_CURVE_CHANNELS {
            return Err(CurveError::ChannelCount {
                expected: TEMPO_CURVE_CHANNELS,
                count: curve.channels(),
            });
        }
        let range = sanitized_range(range);
        let endpoints = curve_endpoints(&curve);
        Ok(Self {
            curve,
            binding: tempo_binding(range),
            range,
            endpoints,
        })
    }

    /// The installed curve group.
    pub fn curve(&self) -> &ResolvedCurveGroup {
        &self.curve
    }

    /// The normalized-to-signed-pitch binding.
    pub fn binding(&self) -> &CurveBinding {
        &self.binding
    }

    /// Tempo fader half-span as a pitch fraction.
    pub fn range(&self) -> f32 {
        self.range
    }

    /// Replace the curve, keeping the current range and fader position.
    pub fn set_curve(&mut self, curve: ResolvedCurveGroup) -> Result<(), CurveError> {
        if curve.channels() != TEMPO_CURVE_CHANNELS {
            return Err(CurveError::ChannelCount {
                expected: TEMPO_CURVE_CHANNELS,
                count: curve.channels(),
            });
        }
        self.endpoints = curve_endpoints(&curve);
        self.curve = curve;
        Ok(())
    }

    /// Change the signed pitch half-span.
    pub fn set_range(&mut self, range: f32) {
        self.range = sanitized_range(range);
        self.binding = tempo_binding(self.range);
    }

    /// Signed pitch offset at fader position `position`.
    pub fn pitch_at(&self, position: f32) -> f32 {
        let x = self.binding.normalized_input(position);
        let mut out = [0.0f32; TEMPO_CURVE_CHANNELS];
        let y = match self.curve.evaluate_into(x, &mut out) {
            Ok(()) => out[0],
            // The channel count is checked on install; fall back to the raw
            // position rather than panicking inside a callback.
            Err(_) => x,
        };
        self.binding.denormalized_output(self.recenter(x, y))
    }

    /// Playback ratio at fader position `position` (`1.0` at the detent).
    pub fn playback_ratio_at(&self, position: f32) -> f32 {
        (1.0 + self.pitch_at(position)).max(0.01)
    }

    /// Fader position that produces playback `ratio`.
    ///
    /// Used by tempo sync: the inverse is solved numerically so custom curves
    /// (including ones with a shifted center) keep the same UI position.
    pub fn position_for_playback_ratio(&self, ratio: f32) -> f32 {
        self.position_for_pitch(ratio - 1.0)
    }

    fn position_for_pitch(&self, target: f32) -> f32 {
        if !target.is_finite() {
            return 0.5;
        }
        const COARSE: usize = 128;
        let mut best_x = 0.0;
        let mut best_error = f32::INFINITY;
        for i in 0..=COARSE {
            let x = i as f32 / COARSE as f32;
            let error = (self.pitch_at(x) - target).abs();
            if error < best_error {
                best_error = error;
                best_x = x;
            }
        }
        if best_error <= 1e-6 {
            return best_x;
        }

        // Refine inside the bracket around the best coarse sample. Tempo curves
        // are monotonic in practice; if a half is flat, keep the coarse point.
        let step = 1.0 / COARSE as f32;
        let mut lo = (best_x - step).max(0.0);
        let mut hi = (best_x + step).min(1.0);
        let pitch_lo = self.pitch_at(lo);
        let pitch_hi = self.pitch_at(hi);
        if (pitch_lo - pitch_hi).abs() <= 1e-7 {
            return best_x;
        }
        let decreasing = pitch_lo > pitch_hi;
        for _ in 0..24 {
            let mid = (lo + hi) * 0.5;
            let pitch = self.pitch_at(mid);
            if (pitch - target).abs() <= 1e-7 {
                return mid;
            }
            let move_right = if decreasing {
                pitch > target
            } else {
                pitch < target
            };
            if move_right {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo + hi) * 0.5
    }

    /// Pin `x = 0.5` to zero pitch and both ends to the full signed range.
    ///
    /// Scaling each half by its own curve span keeps the selected shape while
    /// making the center detent and `±range` invariants unconditional.
    fn recenter(&self, x: f32, y: f32) -> f32 {
        let [start, center, end] = self.endpoints;
        let value = if x <= 0.5 {
            let span = center - start;
            if span.abs() <= f32::EPSILON {
                0.5
            } else {
                1.0 - 0.5 * (y - start) / span
            }
        } else {
            let span = end - center;
            if span.abs() <= f32::EPSILON {
                0.5
            } else {
                0.5 - 0.5 * (y - center) / span
            }
        };
        value.clamp(0.0, 1.0)
    }
}

fn sanitized_range(range: f32) -> f32 {
    if range.is_finite() {
        range.max(0.0)
    } else {
        DEFAULT_TEMPO_RANGE
    }
}

fn tempo_binding(range: f32) -> CurveBinding {
    let span = range.max(MIN_OUTPUT_SPAN);
    CurveBinding::new(
        CurveGroupRef::preset(presets::LINEAR),
        AxisMap::UNIT,
        AxisMap::linear(-span, span).expect("a finite positive tempo span is a valid axis"),
    )
    .expect("a valid preset reference and unit input form a valid binding")
}

fn curve_endpoints(curve: &ResolvedCurveGroup) -> [f32; 3] {
    [0.0, 0.5, 1.0].map(|x| {
        let mut out = [0.0f32; TEMPO_CURVE_CHANNELS];
        match curve.evaluate_into(x, &mut out) {
            Ok(()) => out[0],
            Err(_) => x,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn percent_range_maps_ends() {
        let range = 0.06;
        assert!((norm_to_playback_ratio(0.5, range) - 1.0).abs() < 1e-5);
        assert!((norm_to_playback_ratio(0.0, range) - 1.06).abs() < 1e-5);
        assert!((norm_to_playback_ratio(1.0, range) - 0.94).abs() < 1e-5);
    }

    #[test]
    fn next_steps_cycle() {
        assert!((next_tempo_range(0.06) - 0.10).abs() < 1e-5);
        assert!((next_tempo_range(0.10) - 0.16).abs() < 1e-5);
        assert!((next_tempo_range(0.16) - 0.25).abs() < 1e-5);
        assert!((next_tempo_range(0.25) - 0.06).abs() < 1e-5);
        assert!((next_tempo_range(0.08) - 0.06).abs() < 1e-5);
    }

    #[test]
    fn set_range_keeps_speed_changes_ratio() {
        let speed = 0.0;
        let r6 = norm_to_playback_ratio(speed, 0.06);
        let r10 = norm_to_playback_ratio(speed, 0.10);
        assert!((r6 - 1.06).abs() < 1e-5);
        assert!((r10 - 1.10).abs() < 1e-5);
    }

    #[test]
    fn default_linear_curve_preserves_the_historical_mapping() {
        let tempo = TempoCurve::linear(DEFAULT_TEMPO_RANGE);
        for position in [0.0, 0.1, 0.25, 0.4, 0.5, 0.6, 0.75, 0.9, 1.0] {
            let expected = norm_to_playback_ratio(position, DEFAULT_TEMPO_RANGE);
            let actual = tempo.playback_ratio_at(position);
            assert!(
                close(actual, expected),
                "position {position}: {actual} != {expected}"
            );
        }
        assert!(close(tempo.pitch_at(0.5), 0.0), "center is unity");
        assert!(
            close(tempo.pitch_at(0.0), DEFAULT_TEMPO_RANGE),
            "top = +range"
        );
        assert!(
            close(tempo.pitch_at(1.0), -DEFAULT_TEMPO_RANGE),
            "bottom = -range"
        );
    }

    #[test]
    fn custom_curve_keeps_the_center_detent_and_endpoints() {
        // Deliberately asymmetric times and a center below 0.5: the tempo
        // mapping must still put unity at center and reach both signed ends.
        let tempo = TempoCurve::new(curve(&[(0.0, 1.0), (0.5, 0.8), (1.0, 0.0)]), 0.16)
            .expect("one-channel curve");
        assert!(close(tempo.pitch_at(0.5), 0.0), "center stays zero");
        assert!(close(tempo.pitch_at(0.0), 0.16), "start = +range");
        assert!(close(tempo.pitch_at(1.0), -0.16), "end = -range");
        assert!(
            tempo.pitch_at(0.25) > tempo.pitch_at(0.75),
            "the curve remains signed and ordered"
        );
    }

    #[test]
    fn custom_curve_shapes_the_ratio_inside_the_range() {
        let tempo = TempoCurve::new(
            curve(&[(0.0, 1.0), (0.25, 0.85), (0.5, 0.5), (1.0, 0.0)]),
            0.06,
        )
        .expect("one-channel curve");
        assert!(close(tempo.playback_ratio_at(0.25), 1.042));
        assert!(close(tempo.playback_ratio_at(0.5), 1.0));
    }

    #[test]
    fn re_range_updates_the_signed_output() {
        let mut tempo = TempoCurve::linear(DEFAULT_TEMPO_RANGE);
        tempo.set_range(0.25);
        assert!(close(tempo.pitch_at(0.0), 0.25));
        assert!(close(tempo.pitch_at(1.0), -0.25));
        assert!(close(tempo.pitch_at(0.5), 0.0));
    }

    #[test]
    fn position_inverse_roundtrips_for_the_default_curve() {
        let tempo = TempoCurve::linear(DEFAULT_TEMPO_RANGE);
        for ratio in [0.94, 0.97, 1.0, 1.03, 1.06] {
            let position = tempo.position_for_playback_ratio(ratio);
            let actual = tempo.playback_ratio_at(position);
            assert!(
                close(actual, ratio),
                "ratio {ratio} -> position {position}: {actual}"
            );
        }
    }

    #[test]
    fn two_channel_curve_is_rejected() {
        let group = CurveGroup::new(vec![presets::linear(), presets::linear()])
            .expect("two scalar test curves");
        let two_channel = ResolvedCurveGroup::single(&group).expect("compiles");
        assert!(matches!(
            TempoCurve::new(two_channel, DEFAULT_TEMPO_RANGE),
            Err(CurveError::ChannelCount {
                expected: TEMPO_CURVE_CHANNELS,
                ..
            })
        ));
    }
}
