//! Resolved curve groups: compiled tables with optional set morphing.
//!
//! A [`ResolvedCurveGroup`] owns one or more [`CompiledCurveGroup`]s and blends
//! adjacent members for a morph position in `0..=1`. It is the runtime form a
//! control layer installs after resolving ids and compiling specs; evaluating it
//! never allocates and never parses.

use crate::{CompiledCurveGroup, CurveError, CurveGroup, MAX_GROUP_CHANNELS};

/// One or more compiled groups sharing a channel count and a morph position.
///
/// A single member is a plain compiled group. Two or more members form a curve
/// set: the morph position sweeps them in order and adjacent members are
/// blended. Morph `0` is the first member, morph `1` is the last, and an exact
/// member boundary evaluates that member alone.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedCurveGroup {
    members: Vec<CompiledCurveGroup>,
    morph: f32,
}

impl ResolvedCurveGroup {
    /// Compile a single group (the common non-morphing case).
    pub fn single(group: &CurveGroup) -> Result<Self, CurveError> {
        Self::from_compiled(vec![CompiledCurveGroup::compile(group)?])
    }

    /// Compile every group of a curve set.
    pub fn set(groups: &[CurveGroup]) -> Result<Self, CurveError> {
        let members = groups
            .iter()
            .map(CompiledCurveGroup::compile)
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_compiled(members)
    }

    /// Wrap already compiled members.
    pub fn from_compiled(members: Vec<CompiledCurveGroup>) -> Result<Self, CurveError> {
        let Some(first) = members.first() else {
            return Err(CurveError::EmptyResolvedGroup);
        };
        let channels = first.channels();
        for member in &members[1..] {
            if member.channels() != channels {
                return Err(CurveError::ResolvedChannelMismatch {
                    expected: channels,
                    count: member.channels(),
                });
            }
        }
        Ok(Self {
            members,
            morph: 0.0,
        })
    }

    /// Number of curves every member produces.
    pub fn channels(&self) -> usize {
        self.members
            .first()
            .map(CompiledCurveGroup::channels)
            .unwrap_or(0)
    }

    /// The compiled members in set order.
    pub fn members(&self) -> &[CompiledCurveGroup] {
        &self.members
    }

    /// Number of members (a plain group has one).
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Always `false`; present for the `len` lint convention.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Current morph position in `0..=1`.
    pub fn morph(&self) -> f32 {
        self.morph
    }

    /// Set the morph position, clamped to `0..=1`.
    pub fn set_morph(&mut self, morph: f32) {
        self.morph = if morph.is_nan() {
            0.0
        } else {
            morph.clamp(0.0, 1.0)
        };
    }

    /// Builder form of [`Self::set_morph`].
    pub fn with_morph(mut self, morph: f32) -> Self {
        self.set_morph(morph);
        self
    }

    /// Evaluate every output at normalized `x`, blending set neighbors.
    ///
    /// `out` must match [`Self::channels`]. Evaluation is allocation-free.
    pub fn evaluate_into(&self, x: f32, out: &mut [f32]) -> Result<(), CurveError> {
        let channels = self.channels();
        if out.len() != channels {
            return Err(CurveError::OutputLength {
                expected: channels,
                actual: out.len(),
            });
        }

        if self.members.is_empty() {
            return Err(CurveError::EmptyResolvedGroup);
        }
        let last = self.members.len() - 1;
        let scaled = self.morph * last as f32;
        let lower = (scaled.floor() as usize).min(last);
        let upper = (lower + 1).min(last);
        let fraction = (scaled - lower as f32).clamp(0.0, 1.0);

        self.members[lower].evaluate_into(x, out)?;
        if upper == lower || fraction == 0.0 {
            return Ok(());
        }

        let mut next = [0.0f32; MAX_GROUP_CHANNELS];
        let next = &mut next[..channels];
        self.members[upper].evaluate_into(x, next)?;
        for (slot, other) in out.iter_mut().zip(next.iter()) {
            *slot += (*other - *slot) * fraction;
        }
        Ok(())
    }

    /// Evaluate every output, allocating a result vector.
    ///
    /// Control-thread convenience; use [`Self::evaluate_into`] for realtime.
    pub fn evaluate(&self, x: f32) -> Result<Vec<f32>, CurveError> {
        let mut out = vec![0.0; self.channels()];
        self.evaluate_into(x, &mut out)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CurvePoint, CurveSpec, SegmentKind};

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

    fn group(a: [f32; 2], b: [f32; 2]) -> CurveGroup {
        CurveGroup::new(vec![linear(a[0], a[1]), linear(b[0], b[1])]).expect("valid group")
    }

    #[test]
    fn single_member_evaluates_like_its_group() {
        let group = group([1.0, 0.0], [0.0, 1.0]);
        let resolved = ResolvedCurveGroup::single(&group).expect("resolves");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved.channels(), 2);

        let mut out = [0.0; 2];
        resolved.evaluate_into(0.25, &mut out).expect("buffer");
        assert!(close(out[0], 0.75));
        assert!(close(out[1], 0.25));
    }

    #[test]
    fn morph_endpoints_select_the_first_and_last_member() {
        let first = group([1.0, 0.0], [0.0, 1.0]);
        let last = group([1.0, 1.0], [0.0, 0.0]);
        let resolved =
            ResolvedCurveGroup::set(&[first, last]).expect("two-member curve set resolves");

        let mut out = [0.0; 2];
        resolved.evaluate_into(0.5, &mut out).expect("buffer");
        assert!(close(out[0], 0.5));
        assert!(close(out[1], 0.5));

        let last_only = resolved.clone().with_morph(1.0);
        last_only.evaluate_into(0.5, &mut out).expect("buffer");
        assert!(close(out[0], 1.0));
        assert!(close(out[1], 0.0));
    }

    #[test]
    fn two_member_morph_blends_adjacent_tables() {
        let first = group([1.0, 0.0], [0.0, 1.0]);
        let last = group([1.0, 1.0], [0.0, 0.0]);
        let resolved = ResolvedCurveGroup::set(&[first, last])
            .expect("set")
            .with_morph(0.5);

        let mut out = [0.0; 2];
        // At x=0.5 the first member gives (0.5, 0.5) and the last (1.0, 0.0),
        // so a halfway morph gives the average (0.75, 0.25).
        resolved.evaluate_into(0.5, &mut out).expect("buffer");
        assert!(close(out[0], 0.75));
        assert!(close(out[1], 0.25));

        // At x=0 the first gives A=1/B=0, the last A=1/B=0 -> blend stays.
        resolved.evaluate_into(0.0, &mut out).expect("buffer");
        assert!(close(out[0], 1.0));
        assert!(close(out[1], 0.0));
    }

    #[test]
    fn three_member_morph_hits_middle_member_exactly() {
        let flat = group([1.0, 1.0], [1.0, 1.0]);
        let rising = group([0.0, 1.0], [0.0, 1.0]);
        let zero = group([0.0, 0.0], [0.0, 0.0]);
        let resolved = ResolvedCurveGroup::set(&[flat, rising, zero])
            .expect("set")
            .with_morph(0.5);

        let mut out = [0.0; 2];
        resolved.evaluate_into(0.5, &mut out).expect("buffer");
        assert!(close(out[0], 0.5));
        assert!(close(out[1], 0.5));

        // Halfway between members 1 and 2: (0.5, 0.5) mixed with (0.0, 0.0).
        let resolved = resolved.with_morph(0.75);
        resolved.evaluate_into(0.5, &mut out).expect("buffer");
        assert!(close(out[0], 0.25));
        assert!(close(out[1], 0.25));
    }

    #[test]
    fn rejects_empty_and_mixed_channel_sets() {
        assert_eq!(
            ResolvedCurveGroup::from_compiled(Vec::new()),
            Err(CurveError::EmptyResolvedGroup)
        );

        let two = ResolvedCurveGroup::single(&group([1.0, 0.0], [0.0, 1.0])).expect("two");
        let one =
            CompiledCurveGroup::compile(&CurveGroup::scalar(linear(0.0, 1.0)).expect("scalar"))
                .expect("compiles");
        assert_eq!(
            ResolvedCurveGroup::from_compiled(vec![two.members()[0].clone(), one]),
            Err(CurveError::ResolvedChannelMismatch {
                expected: 2,
                count: 1,
            })
        );
    }

    #[test]
    fn morph_is_clamped_and_nan_reads_as_zero() {
        let group = group([1.0, 0.0], [0.0, 1.0]);
        let mut resolved = ResolvedCurveGroup::single(&group).expect("resolves");
        resolved.set_morph(4.0);
        assert_eq!(resolved.morph(), 1.0);
        resolved.set_morph(f32::NAN);
        assert_eq!(resolved.morph(), 0.0);
    }

    #[test]
    fn output_length_is_checked() {
        let group = group([1.0, 0.0], [0.0, 1.0]);
        let resolved = ResolvedCurveGroup::single(&group).expect("resolves");
        assert_eq!(
            resolved.evaluate_into(0.5, &mut [0.0]),
            Err(CurveError::OutputLength {
                expected: 2,
                actual: 1,
            })
        );
    }
}
