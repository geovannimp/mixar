//! qm-dsp `Decimator`: an 8th-order IIR anti-alias filter followed by a
//! power-of-two decimation. GetKeyMode uses factor 8.

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
const B4: [f64; 8] = [
    0.10133306904918619,
    -0.2447523353702363,
    0.33622528590120965,
    -0.13936581560633518,
    -0.13936581560633382,
    0.3362252859012087,
    -0.2447523353702358,
    0.10133306904918594,
];
const A4: [f64; 8] = [
    1.0,
    -3.9035590278139427,
    7.5299379980621133,
    -8.6890803793177511,
    6.4578667096099176,
    -3.0242979431223631,
    0.83043385136748382,
    -0.094420800837809335,
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
const B1: [f64; 8] = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
const A1: [f64; 8] = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

pub struct Decimator {
    input_len: usize,
    output_len: usize,
    factor: usize,
    b: [f64; 8],
    a: [f64; 8],
    /// Direct-form-II-transpose state (`o1..o7`).
    o: [f64; 7],
    buffer: Vec<f64>,
}

impl Decimator {
    pub fn new(input_len: usize, factor: usize) -> Self {
        let (b, a) = match factor {
            8 => (B8, A8),
            4 => (B4, A4),
            2 => (B2, A2),
            _ => (B1, A1),
        };
        Self {
            input_len,
            output_len: input_len / factor,
            factor,
            b,
            a,
            o: [0.0; 7],
            buffer: vec![0.0; input_len],
        }
    }

    /// Filter `input_len` samples and write `input_len / factor` samples.
    pub fn process(&mut self, src: &[f64], dst: &mut [f64]) {
        if self.factor == 1 {
            dst[..self.output_len].copy_from_slice(&src[..self.output_len]);
            return;
        }
        self.anti_alias(src);
        for (i, d) in dst.iter_mut().enumerate().take(self.output_len) {
            *d = self.buffer[self.factor * i];
        }
    }

    fn anti_alias(&mut self, src: &[f64]) {
        for (i, &input) in src.iter().enumerate().take(self.input_len) {
            let out = input * self.b[0] + self.o[0];
            self.o[0] = input * self.b[1] - out * self.a[1] + self.o[1];
            self.o[1] = input * self.b[2] - out * self.a[2] + self.o[2];
            self.o[2] = input * self.b[3] - out * self.a[3] + self.o[3];
            self.o[3] = input * self.b[4] - out * self.a[4] + self.o[4];
            self.o[4] = input * self.b[5] - out * self.a[5] + self.o[5];
            self.o[5] = input * self.b[6] - out * self.a[6] + self.o[6];
            self.o[6] = input * self.b[7] - out * self.a[7];
            self.buffer[i] = out;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_eight_halves_length_eight_times() {
        let mut d = Decimator::new(64, 8);
        let src = vec![1.0; 64];
        let mut dst = vec![0.0; 8];
        d.process(&src, &mut dst);
        assert!(dst.iter().all(|v| v.is_finite()));
        assert!(dst.iter().any(|&v| v.abs() > 0.0));
    }

    #[test]
    fn factor_one_is_a_copy() {
        let mut d = Decimator::new(8, 1);
        let src: Vec<f64> = (0..8).map(|i| i as f64).collect();
        let mut dst = vec![0.0; 8];
        d.process(&src, &mut dst);
        assert_eq!(dst, src);
    }
}
