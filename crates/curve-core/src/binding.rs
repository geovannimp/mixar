use crate::{AxisMap, CurveError, CurveGroup, CurveGroupRef};

/// A curve group bound to real input and output ranges.
///
/// The curve always runs in normalized `0..=1` space. `input` converts the real
/// control value into that space before evaluation, and `output` converts each
/// normalized result back into real units. This is what lets one engine serve a
/// signed tempo range, a `0..1` FX level, and a logarithmic filter axis without
/// any target-specific code.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveBinding {
    pub curve: CurveGroupRef,
    pub input: AxisMap,
    pub output: AxisMap,
}

impl CurveBinding {
    /// Validate and construct a binding.
    pub fn new(curve: CurveGroupRef, input: AxisMap, output: AxisMap) -> Result<Self, CurveError> {
        let binding = Self {
            curve,
            input,
            output,
        };
        binding.validate()?;
        Ok(binding)
    }

    /// Validate the reference and both axis maps.
    pub fn validate(&self) -> Result<(), CurveError> {
        self.curve.validate()?;
        self.input.validate()?;
        self.output.validate()
    }

    /// Convert a real input value into the normalized curve domain.
    pub fn normalized_input(&self, value: f32) -> f32 {
        self.input.normalize(value)
    }

    /// Convert a normalized curve output into real units.
    pub fn denormalized_output(&self, value: f32) -> f32 {
        self.output.denormalize(value)
    }

    /// Evaluate `group` at real `value`, writing one real output per channel.
    pub fn evaluate_into(
        &self,
        group: &CurveGroup,
        value: f32,
        out: &mut [f32],
    ) -> Result<(), CurveError> {
        self.validate()?;
        if out.len() != group.channels() {
            return Err(CurveError::OutputLength {
                expected: group.channels(),
                actual: out.len(),
            });
        }
        group.evaluate_into(self.normalized_input(value), out)?;
        for slot in out.iter_mut() {
            *slot = self.output.denormalize(*slot);
        }
        Ok(())
    }

    /// Evaluate `group`, allocating one real output per channel.
    pub fn evaluate(&self, group: &CurveGroup, value: f32) -> Result<Vec<f32>, CurveError> {
        let mut out = vec![0.0; group.channels()];
        self.evaluate_into(group, value, &mut out)?;
        Ok(out)
    }

    /// Evaluate a scalar (one-channel) group.
    pub fn evaluate_scalar(&self, group: &CurveGroup, value: f32) -> Result<f32, CurveError> {
        group.expect_channels(1)?;
        let mut out = [0.0];
        self.evaluate_into(group, value, &mut out)?;
        Ok(out[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{presets, AxisScale, CurvePoint, CurveSpec, SegmentKind};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-3 * b.abs().max(1.0)
    }

    fn linear_curve(y0: f32, y1: f32) -> CurveSpec {
        CurveSpec::new(
            vec![CurvePoint::new(0.0, y0), CurvePoint::new(1.0, y1)],
            vec![SegmentKind::Linear],
        )
        .expect("valid curve")
    }

    #[test]
    fn fx_style_unit_range_works() {
        let group =
            CurveGroup::scalar(presets::exponential(4.0).expect("valid")).expect("valid group");
        let binding = CurveBinding::new(
            CurveGroupRef::preset("exponential"),
            AxisMap::UNIT,
            AxisMap::UNIT,
        )
        .expect("valid binding");
        let output = binding.evaluate_scalar(&group, 0.5).expect("scalar");
        assert!(close(output, group.curves[0].evaluate(0.5)));
    }

    #[test]
    fn signed_tempo_range_roundtrips_without_crossfader_code() {
        let group = CurveGroup::scalar(linear_curve(0.0, 1.0)).expect("valid group");
        let tempo = AxisMap::linear(-8.0, 8.0).expect("valid axis");
        let binding =
            CurveBinding::new(CurveGroupRef::custom("tempo"), tempo, tempo).expect("valid binding");
        assert_eq!(binding.normalized_input(0.0), 0.5);
        assert_eq!(binding.denormalized_output(0.5), 0.0);
        for value in [-8.0, -4.0, 0.0, 4.0, 8.0] {
            let output = binding.evaluate_scalar(&group, value).expect("scalar");
            assert!(close(output, value), "tempo {value} -> {output}");
        }
    }

    #[test]
    fn logarithmic_output_maps_real_units() {
        let group = CurveGroup::scalar(linear_curve(0.0, 1.0)).expect("valid group");
        let binding = CurveBinding::new(
            CurveGroupRef::custom("filter"),
            AxisMap::UNIT,
            AxisMap::logarithmic(20.0, 20_000.0).expect("valid axis"),
        )
        .expect("valid binding");
        let output = binding.evaluate_scalar(&group, 0.5).expect("scalar");
        assert!(close(output, 632.455_5), "geometric midpoint, got {output}");
    }

    #[test]
    fn two_channel_group_maps_every_output() {
        let group = CurveGroup::new(vec![linear_curve(1.0, 0.0), linear_curve(0.0, 1.0)])
            .expect("valid group");
        let binding = CurveBinding::new(
            CurveGroupRef::custom("crossfader"),
            AxisMap::UNIT,
            AxisMap::UNIT,
        )
        .expect("valid binding");
        let output = binding.evaluate(&group, 0.25).expect("two channels");
        assert!(close(output[0], 0.75));
        assert!(close(output[1], 0.25));

        let mut out = [0.0; 2];
        binding
            .evaluate_into(&group, 0.75, &mut out)
            .expect("buffer");
        assert!(close(out[0], 0.25));
        assert!(close(out[1], 0.75));
    }

    #[test]
    fn rejects_bad_bindings_and_buffers() {
        assert!(matches!(
            CurveBinding::new(
                CurveGroupRef::custom("bad-id"),
                AxisMap::UNIT,
                AxisMap::UNIT,
            ),
            Err(CurveError::InvalidId { .. })
        ));
        let invalid_axis = AxisMap {
            min: 1.0,
            max: 0.0,
            scale: AxisScale::Linear,
        };
        assert!(matches!(
            CurveBinding::new(CurveGroupRef::custom("curve"), AxisMap::UNIT, invalid_axis,),
            Err(CurveError::AxisBounds { .. })
        ));

        let binding =
            CurveBinding::new(CurveGroupRef::custom("curve"), AxisMap::UNIT, AxisMap::UNIT)
                .expect("valid binding");
        let group = CurveGroup::scalar(linear_curve(0.0, 1.0)).expect("valid group");
        assert!(matches!(
            binding.evaluate_into(&group, 0.5, &mut [0.0; 2]),
            Err(CurveError::OutputLength {
                expected: 1,
                actual: 2
            })
        ));
        let two = CurveGroup::new(vec![linear_curve(0.0, 1.0), linear_curve(1.0, 0.0)])
            .expect("valid group");
        assert!(matches!(
            binding.evaluate_scalar(&two, 0.5),
            Err(CurveError::ChannelCount {
                expected: 1,
                count: 2
            })
        ));
    }
}
