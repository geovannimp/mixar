use super::config::ChannelMode;
use super::SpectralPeak;

pub fn peaks_to_rgb_bytes(peaks: &[SpectralPeak], channel_mode: ChannelMode) -> Vec<u8> {
    let bytes_per_sample = channel_mode.bytes_per_sample();
    let mut out = Vec::with_capacity(peaks.len() * bytes_per_sample);
    for peak in peaks {
        out.push(float_to_u8(peak.low));
        out.push(float_to_u8(peak.mid));
        out.push(float_to_u8(peak.high));
        if channel_mode == ChannelMode::Stereo {
            out.push(float_to_u8(peak.low));
            out.push(float_to_u8(peak.mid));
            out.push(float_to_u8(peak.high));
        }
    }
    out
}

pub fn rgb_bytes_to_peaks(
    bytes: &[u8],
    count: usize,
    channel_mode: ChannelMode,
) -> Option<Vec<SpectralPeak>> {
    let bytes_per_sample = channel_mode.bytes_per_sample();
    let expected = count.checked_mul(bytes_per_sample)?;
    if bytes.len() != expected {
        return None;
    }

    let mut peaks = Vec::with_capacity(count);
    for chunk in bytes.chunks_exact(bytes_per_sample) {
        peaks.push(SpectralPeak {
            low: u8_to_float(chunk[0]),
            mid: u8_to_float(chunk[1]),
            high: u8_to_float(chunk[2]),
        });
    }
    Some(peaks)
}

fn float_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn u8_to_float(value: u8) -> f32 {
    f32::from(value) / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waveform::config::ChannelMode;

    #[test]
    fn round_trip_mono_rgb() {
        let peaks = vec![
            SpectralPeak {
                low: 0.25,
                mid: 0.5,
                high: 1.0,
            },
            SpectralPeak {
                low: 0.0,
                mid: 0.0,
                high: 0.0,
            },
        ];
        let bytes = peaks_to_rgb_bytes(&peaks, ChannelMode::Mono);
        let decoded = rgb_bytes_to_peaks(&bytes, peaks.len(), ChannelMode::Mono).unwrap();
        assert_eq!(decoded.len(), 2);
        assert!((decoded[0].low - 0.25).abs() < 0.01);
        assert!((decoded[0].high - 1.0).abs() < 0.01);
    }
}
