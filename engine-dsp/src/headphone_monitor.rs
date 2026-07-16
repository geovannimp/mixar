//! Headphone / preview bus monitor: blend PFL with gated master tap.

use audio_core::Sample;

/// Blends pre-fader listen (PFL) with an optional master tap for the cue bus.
#[derive(Debug, Default, Clone, Copy)]
pub struct HeadphoneMonitor;

impl HeadphoneMonitor {
    /// Render interleaved stereo (or any sample buffer) into `out`.
    ///
    /// `cue_mix`: 0.0 = PFL only, 1.0 = master tap only (when `master_cue`).
    /// When `master_cue` is false, the master contribution is silence.
    pub fn render(
        pfl: &[Sample],
        master_tap: &[Sample],
        cue_mix: f32,
        master_cue: bool,
        out: &mut [Sample],
    ) {
        let mix = cue_mix.clamp(0.0, 1.0);
        let n = out.len().min(pfl.len()).min(master_tap.len());
        for i in 0..n {
            let master = if master_cue { master_tap[i] } else { 0.0 };
            out[i] = (1.0 - mix) * pfl[i] + mix * master;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pfl_only_when_mix_zero() {
        let pfl = [1.0_f32, -1.0];
        let master = [0.5_f32, 0.5];
        let mut out = [0.0; 2];
        HeadphoneMonitor::render(&pfl, &master, 0.0, true, &mut out);
        assert!((out[0] - 1.0).abs() < 1e-6);
        assert!((out[1] + 1.0).abs() < 1e-6);
    }

    #[test]
    fn master_only_when_mix_one_and_master_cue_on() {
        let pfl = [1.0_f32, 1.0];
        let master = [0.25_f32, -0.25];
        let mut out = [0.0; 2];
        HeadphoneMonitor::render(&pfl, &master, 1.0, true, &mut out);
        assert!((out[0] - 0.25).abs() < 1e-6);
        assert!((out[1] + 0.25).abs() < 1e-6);
    }

    #[test]
    fn master_gated_off_when_master_cue_false() {
        let pfl = [1.0_f32, 1.0];
        let master = [0.9_f32, 0.9];
        let mut out = [0.0; 2];
        HeadphoneMonitor::render(&pfl, &master, 1.0, false, &mut out);
        assert!(out[0].abs() < 1e-6);
        assert!(out[1].abs() < 1e-6);
    }

    #[test]
    fn mid_blend_with_master_cue() {
        let pfl = [1.0_f32, 0.0];
        let master = [0.0_f32, 0.0];
        let mut out = [0.0; 2];
        HeadphoneMonitor::render(&pfl, &master, 0.5, true, &mut out);
        assert!((out[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn master_cue_off_mix_attenuates_pfl_only() {
        let pfl = [1.0_f32, 1.0];
        let master = [1.0_f32, 1.0];
        let mut out = [0.0; 2];
        HeadphoneMonitor::render(&pfl, &master, 0.5, false, &mut out);
        assert!((out[0] - 0.5).abs() < 1e-6);
        assert!((out[1] - 0.5).abs() < 1e-6);
    }
}
