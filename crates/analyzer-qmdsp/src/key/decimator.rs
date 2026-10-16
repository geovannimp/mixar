//! qm-dsp `Decimator`: an 8th-order IIR anti-alias filter followed by a
//! power-of-two decimation. GetKeyMode uses factor 8 and downbeat chains ×8
//! then ×2; the factor-4 tables qm-dsp ships are not needed and were deleted.

const B8: [f64; 8] = [
    0.060111378492136,
    -0.257323420830598,
    0.420583503165928,
    -0.222750785197418,
    -0.222750785197418,
    0.420583503165928,
    -0.257323420830598,
    0.060111378492136,
];
const A8: [f64; 8] = [
    1.0,
    -5.667654878577432,
    14.062452278088417,
    -19.737303840697738,
    16.889698874608641,
    -8.796600612325928,
    2.577553446979888,
    -0.326903916815751,
];
const B2: [f64; 8] = [
    0.20898944260075727,
    0.40011234879814367,
    0.819741973072733,
    1.0087419911682323,
    1.0087419911682325,
    0.81974197307273156,
    0.40011234879814295,
    0.20898944260075661,
];
const A2: [f64; 8] = [
    1.0,
    0.0077331184208358217,
    1.9853971155964376,
    0.19296739275341004,
    1.2330748872852182,
    0.18705341389316466,
    0.23659265908013868,
    0.032352924250533946,
];
/// Anti-alias decimation stage qm-dsp's `Decimator` ships: an 8th-order IIR
/// followed by a power-of-two decimation. Only the two factors Mixar uses are
/// supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decimation {
    /// ×2 stage (downbeat's second stage).
    By2,
    /// ×8 stage (key path and downbeat's first stage).
    By8,
}

impl Decimation {
    fn factor(self) -> usize {
        match self {
            Self::By2 => 2,
            Self::By8 => 8,
        }
    }

    fn coefficients(self) -> ([f64; 8], [f64; 8]) {
        match self {
            Self::By2 => (B2, A2),
            Self::By8 => (B8, A8),
        }
    }
}

pub struct Decimator {
    input_len: usize,
    output_len: usize,
    factor: usize,
    b: [f64; 8],
    a: [f64; 8],
    /// Direct-form-II-transpose state (`o1..o7`).
    o: [f64; 7],
}

impl Decimator {
    pub fn new(stage: Decimation, input_len: usize) -> Self {
        let (b, a) = stage.coefficients();
        let factor = stage.factor();
        Self {
            input_len,
            output_len: input_len / factor,
            factor,
            b,
            a,
            o: [0.0; 7],
        }
    }

    /// Filter `input_len` samples and write `input_len / factor` samples.
    pub fn process(&mut self, src: &[f64], dst: &mut [f64]) {
        debug_assert_eq!(src.len(), self.input_len, "decimator input length mismatch");
        debug_assert!(
            dst.len() >= self.output_len,
            "decimator output buffer too small"
        );
        self.anti_alias_decimating(src, &mut dst[..self.output_len]);
    }

    /// qm-dsp `Decimator::antiAliasDecimating`: one DF2T IIR sweep writing
    /// directly into `dst`, no whole-track scratch.
    ///
    /// The sweep is bounded to the last input index whose output is actually
    /// kept, so exactly `output_len` samples are written. `output_len` is
    /// `input_len / factor` (floor), which is not the same as the number of
    /// indices divisible by `factor` in `0..input_len` (ceil) — sweeping the
    /// whole input would write one sample past `dst` whenever
    /// `input_len % factor != 0`. Every value the old `buffer[factor * i]`
    /// lookup produced is still produced, at the same index and bit-for-bit.
    fn anti_alias_decimating(&mut self, src: &[f64], dst: &mut [f64]) {
        if self.output_len == 0 {
            return;
        }
        let last = (self.output_len - 1) * self.factor;
        let mut written = 0usize;
        for i in 0..=last {
            let input = src[i];
            let out = input * self.b[0] + self.o[0];
            self.o[0] = input * self.b[1] - out * self.a[1] + self.o[1];
            self.o[1] = input * self.b[2] - out * self.a[2] + self.o[2];
            self.o[2] = input * self.b[3] - out * self.a[3] + self.o[3];
            self.o[3] = input * self.b[4] - out * self.a[4] + self.o[4];
            self.o[4] = input * self.b[5] - out * self.a[5] + self.o[5];
            self.o[5] = input * self.b[6] - out * self.a[6] + self.o[6];
            self.o[6] = input * self.b[7] - out * self.a[7];
            if i % self.factor == 0 {
                dst[written] = out;
                written += 1;
            }
        }
        debug_assert_eq!(written, self.output_len, "decimation wrote the wrong count");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Direct-form (accumulator) evaluation of the same transfer function.
    ///
    /// Deliberately independent of the DF2T recurrence under test, so a wrong
    /// decimation stride, a wrong coefficient or a wrong initial state all show
    /// up. Tolerances, not `to_bits()`: the two forms round differently by
    /// design, and what matters here is that the *stride* and the *shape*
    /// cannot silently drift.
    fn reference(src: &[f64], b: [f64; 8], a: [f64; 8]) -> Vec<f64> {
        let mut y = vec![0.0; src.len()];
        for n in 0..src.len() {
            let mut acc = 0.0;
            for k in 0..8 {
                if n < k {
                    continue;
                }
                acc += b[k] * src[n - k];
                if k >= 1 {
                    acc -= a[k] * y[n - k];
                }
            }
            y[n] = acc;
        }
        y
    }

    fn assert_stage(stage: Decimation, input_len: usize, b: [f64; 8], a: [f64; 8]) {
        let factor = stage.factor();
        let mut d = Decimator::new(stage, input_len);
        let src: Vec<f64> = (0..input_len)
            .map(|i| (((i * 7 + 1) % 13) as f64) / 13.0)
            .collect();
        let mut dst = vec![0.0; input_len / factor];
        d.process(&src, &mut dst);

        // Length contract and zero initial state.
        assert_eq!(dst.len(), input_len / factor);
        assert_eq!(dst[0].to_bits(), (b[0] * src[0]).to_bits());

        let y = reference(&src, b, a);
        for k in 0..dst.len() {
            let want = y[k * factor];
            assert!(
                (dst[k] - want).abs() <= 1e-9 * want.abs().max(1.0),
                "{stage:?}: dst[{k}] = {}, reference y[{}] = {want}",
                dst[k],
                k * factor
            );
        }
        assert!(dst.iter().all(|v| v.is_finite()));
        assert!(dst.iter().any(|&v| v.abs() > 0.0));
    }

    #[test]
    fn factor_eight_halves_length_eight_times() {
        let mut d = Decimator::new(Decimation::By8, 64);
        let src = vec![1.0; 64];
        let mut dst = vec![0.0; 8];
        d.process(&src, &mut dst);
        assert_eq!(dst.len(), 8);
        assert_eq!(dst[0].to_bits(), (B8[0] * 1.0).to_bits());
    }

    #[test]
    fn stage_by8_matches_an_independent_direct_form() {
        assert_stage(Decimation::By8, 64, B8, A8);
    }

    #[test]
    fn stage_by2_matches_an_independent_direct_form() {
        // The downbeat second stage; a wrong stride is most visible here.
        assert_stage(Decimation::By2, 64, B2, A2);
    }

    #[test]
    fn partial_final_group_still_writes_every_output() {
        // 20 inputs / 8 = 2 outputs, so the last two groups are partial.
        let mut d = Decimator::new(Decimation::By8, 20);
        let src: Vec<f64> = (0..20).map(|i| i as f64 / 20.0).collect();
        let mut dst = vec![0.0; 2];
        d.process(&src, &mut dst);
        let y = reference(&src, B8, A8);
        for k in 0..2 {
            let want = y[k * 8];
            assert!((dst[k] - want).abs() <= 1e-9 * want.abs().max(1.0));
        }
    }
}
