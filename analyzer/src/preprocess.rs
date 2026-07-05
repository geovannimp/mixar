use analyzer_core::AnalysisConfig;

pub struct PreparedPcm {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

pub fn prepare(samples: &[f32], sample_rate: u32, _config: &AnalysisConfig) -> PreparedPcm {
    let mut out = samples.to_vec();
    peak_normalize(&mut out);
    PreparedPcm {
        samples: out,
        sample_rate,
    }
}

fn peak_normalize(samples: &mut [f32]) {
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if peak > 1.0 {
        let scale = 1.0 / peak;
        for s in samples {
            *s *= scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_normalize_scales_down() {
        let mut samples = vec![0.0, 2.0, -2.0];
        peak_normalize(&mut samples);
        assert!((samples[1] - 1.0).abs() < f32::EPSILON);
    }
}
