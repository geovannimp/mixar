use super::config::FilterKind;

pub trait BandFilterBank {
    #[allow(dead_code)]
    fn reset(&mut self);
    fn split(&mut self, mono: f32) -> (f32, f32, f32);
}

pub fn new_band_filter_bank(
    kind: FilterKind,
    sample_rate: u32,
    low_hz: f32,
    mid_high_hz: f32,
) -> Box<dyn BandFilterBank> {
    match kind {
        FilterKind::OnePole => Box::new(OnePoleBandFilter::new(sample_rate, low_hz, mid_high_hz)),
        FilterKind::Biquad => Box::new(BiquadBandFilter::new(sample_rate, low_hz, mid_high_hz)),
    }
}

struct OnePoleBandFilter {
    alpha_low: f32,
    alpha_mid_high: f32,
    low_state: f32,
    mid_low_state: f32,
    mid_high_state: f32,
}

impl OnePoleBandFilter {
    fn new(sample_rate: u32, low_hz: f32, mid_high_hz: f32) -> Self {
        let sr = sample_rate as f32;
        Self {
            alpha_low: one_pole_alpha(low_hz, sr),
            alpha_mid_high: one_pole_alpha(mid_high_hz, sr),
            low_state: 0.0,
            mid_low_state: 0.0,
            mid_high_state: 0.0,
        }
    }
}

impl BandFilterBank for OnePoleBandFilter {
    fn reset(&mut self) {
        self.low_state = 0.0;
        self.mid_low_state = 0.0;
        self.mid_high_state = 0.0;
    }

    fn split(&mut self, mono: f32) -> (f32, f32, f32) {
        self.low_state += self.alpha_low * (mono - self.low_state);
        self.mid_low_state += self.alpha_low * (mono - self.mid_low_state);
        self.mid_high_state += self.alpha_mid_high * (mono - self.mid_high_state);
        let low = self.low_state;
        let mid = self.mid_low_state - self.mid_high_state;
        let high = mono - self.mid_high_state;
        (low, mid, high)
    }
}

struct BiquadBandFilter {
    low: Biquad,
    mid_high: Biquad,
    high: Biquad,
}

impl BiquadBandFilter {
    fn new(sample_rate: u32, low_hz: f32, mid_high_hz: f32) -> Self {
        let sr = sample_rate as f32;
        Self {
            low: Biquad::low_pass(sr, low_hz),
            mid_high: Biquad::high_pass(sr, mid_high_hz),
            high: Biquad::high_pass(sr, mid_high_hz),
        }
    }
}

impl BandFilterBank for BiquadBandFilter {
    fn reset(&mut self) {
        self.low.reset();
        self.mid_high.reset();
        self.high.reset();
    }

    fn split(&mut self, mono: f32) -> (f32, f32, f32) {
        let low = self.low.process(mono);
        let high = self.high.process(mono);
        let mid_band = self.mid_high.process(mono);
        let mid = mid_band - high;
        (low, mid, high)
    }
}

fn one_pole_alpha(cutoff_hz: f32, sample_rate: f32) -> f32 {
    let dt = 1.0 / sample_rate;
    let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
    dt / (rc + dt)
}

struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn low_pass(sample_rate: f32, freq: f32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let q = std::f32::consts::FRAC_1_SQRT_2;
        let alpha = sin_w0 / (2.0 * q);
        let b0 = (1.0 - cos_w0) * 0.5;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        Self::from_raw(b0 / a0, b1 / a0, b2 / a0, -2.0 * cos_w0 / a0, (1.0 - alpha) / a0)
    }

    fn high_pass(sample_rate: f32, freq: f32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let q = std::f32::consts::FRAC_1_SQRT_2;
        let alpha = sin_w0 / (2.0 * q);
        let b0 = (1.0 + cos_w0) * 0.5;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        Self::from_raw(b0 / a0, b1 / a0, b2 / a0, -2.0 * cos_w0 / a0, (1.0 - alpha) / a0)
    }

    fn from_raw(b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            z1: 0.0,
            z2: 0.0,
        }
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    fn process(&mut self, input: f32) -> f32 {
        let out = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * out + self.z2;
        self.z2 = self.b2 * input - self.a2 * out;
        out
    }
}
