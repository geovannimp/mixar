//! Rust rasterization for scrolling spectral waveform lanes (§8.5).

use audio_core::SpectralPeak;
use library::BeatGridSnapshot;

const LOW_RGB: [f32; 3] = [255.0, 72.0, 48.0];
const MID_RGB: [f32; 3] = [118.0, 228.0, 88.0];
const HIGH_RGB: [f32; 3] = [72.0, 188.0, 255.0];

/// Beat grid styling defaults when analysis has no meter metadata.
const BEATS_PER_BAR: usize = 4;

/// Render-time band gains (Mixxx-style). MVP defaults to unity; deck EQ maps here.
#[derive(Debug, Clone, Copy)]
pub struct WaveformDisplayGains {
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

impl Default for WaveformDisplayGains {
    fn default() -> Self {
        Self {
            low: 1.0,
            mid: 1.0,
            high: 1.0,
        }
    }
}

impl WaveformDisplayGains {
    pub fn from_eq_db(low_db: f32, mid_db: f32, high_db: f32) -> Self {
        fn db_to_gain(db: f32) -> f32 {
            10f32.powf(db.clamp(-24.0, 24.0) / 20.0)
        }
        Self {
            low: db_to_gain(low_db),
            mid: db_to_gain(mid_db),
            high: db_to_gain(high_db),
        }
    }
}

/// Hi-res peaks covering a sub-range of the track timeline.
#[derive(Debug, Clone)]
pub struct DetailWindow {
    pub peaks: Vec<SpectralPeak>,
    pub start_secs: f64,
    pub end_secs: f64,
}

pub fn render_scrolling_lane(
    width: usize,
    height: usize,
    overview: &[SpectralPeak],
    detail: Option<&DetailWindow>,
    duration_secs: f64,
    position_secs: f64,
    visible_secs: f64,
    gains: WaveformDisplayGains,
    beat_grid: Option<&BeatGridSnapshot>,
    allow_fallback_grid: bool,
) -> Vec<u8> {
    let width = width.max(1);
    let height = height.max(1);
    let mut rgba = vec![0u8; width * height * 4];
    let center_x = width as f32 / 2.0;
    let mid_y = height as f32 / 2.0;
    let max_amp = height as f32 * 0.46;

    fill_background(&mut rgba, width, height);

    if overview.is_empty() || duration_secs <= 0.0 || visible_secs <= 0.0 {
        return rgba;
    }

    let start_time = position_secs - visible_secs / 2.0;
    let end_time = position_secs + visible_secs / 2.0;
    let pixels_per_sec = width as f32 / visible_secs as f32;

    let drew_bpm_grid = beat_grid
        .map(|grid| {
            draw_even_bpm_grid(
                &mut rgba,
                width,
                height,
                grid,
                start_time,
                end_time,
                duration_secs,
                position_secs,
                pixels_per_sec,
                center_x,
            )
        })
        .unwrap_or(false);

    if !drew_bpm_grid && allow_fallback_grid {
        draw_second_grid(
            &mut rgba,
            width,
            height,
            start_time,
            position_secs,
            visible_secs,
            pixels_per_sec,
            center_x,
        );
    }

    for x in 0..width {
        let time = start_time + (x as f64 / width as f64) * visible_secs;
        if time < 0.0 || time > duration_secs {
            continue;
        }

        let peak = peak_at_time(overview, detail, duration_secs, time);
        let low = peak.low * gains.low;
        let mid = peak.mid * gains.mid;
        let high = peak.high * gains.high;
        let amp = low.max(mid).max(high);
        if amp <= 0.001 {
            continue;
        }

        let bar_h = amp * max_amp;
        let (r, g, b) = spectral_rgb(low, mid, high);
        let alpha = 0.65 + amp * 0.35;
        draw_vertical_bar(&mut rgba, width, height, x, mid_y, bar_h, r, g, b, alpha);
    }

    rgba
}

fn fill_background(rgba: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            rgba[idx] = 5;
            rgba[idx + 1] = 5;
            rgba[idx + 2] = 8;
            rgba[idx + 3] = 255;
        }
    }
}

/// Even BPM grid: constant beat period from analyzed BPM, phase from first beat,
/// bar marks every [`BEATS_PER_BAR`] beats (4/4 default).
fn draw_even_bpm_grid(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    grid: &BeatGridSnapshot,
    start_time: f64,
    end_time: f64,
    duration_secs: f64,
    position_secs: f64,
    pixels_per_sec: f32,
    center_x: f32,
) -> bool {
    let Some((beat_period, phase)) = resolve_even_grid(grid) else {
        return false;
    };
    if beat_period <= 0.0 {
        return false;
    }

    let range_start = start_time.max(0.0);
    let range_end = end_time.min(duration_secs.max(0.0));
    if range_end <= range_start {
        return false;
    }

    // First beat index at-or-before visible start.
    let mut beat_index = ((range_start - phase) / beat_period).floor() as i64;
    loop {
        let beat_time = phase + beat_index as f64 * beat_period;
        if beat_time > range_end + beat_period {
            break;
        }
        if beat_time >= range_start - 1e-6 && beat_time <= range_end + 1e-6 {
            let Some(xi) = time_to_x(
                beat_time as f32,
                position_secs,
                pixels_per_sec,
                center_x,
                width,
            ) else {
                beat_index += 1;
                continue;
            };

            let is_bar = beat_index.rem_euclid(BEATS_PER_BAR as i64) == 0;
            if is_bar {
                blend_vertical_line(rgba, width, height, xi, [200, 205, 215, 80]);
                draw_bar_markers(rgba, width, height, xi, [255, 70, 70, 230]);
            } else {
                blend_vertical_line(rgba, width, height, xi, [170, 175, 185, 55]);
                draw_edge_ticks(rgba, width, height, xi, 2, [230, 233, 240, 160]);
            }
        }
        beat_index += 1;
        if beat_index > 1_000_000 {
            break;
        }
    }

    true
}

fn resolve_even_grid(grid: &BeatGridSnapshot) -> Option<(f64, f64)> {
    let bpm = grid.bpm.filter(|b| *b > 20.0 && *b < 400.0).or_else(|| {
        // Infer from the most common early beat interval if BPM column missing.
        if grid.beats.len() < 8 {
            return None;
        }
        let diffs: Vec<f32> = grid
            .beats
            .windows(2)
            .take(32)
            .map(|w| w[1] - w[0])
            .collect();
        let median = {
            let mut d = diffs;
            d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            d[d.len() / 2]
        };
        if median > 0.1 {
            Some(60.0 / f64::from(median))
        } else {
            None
        }
    })?;

    let beat_period = 60.0 / bpm;
    let phase = grid.beats.first().copied().map(f64::from).unwrap_or(0.0);
    Some((beat_period, phase))
}

fn draw_second_grid(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    start_time: f64,
    position_secs: f64,
    visible_secs: f64,
    pixels_per_sec: f32,
    center_x: f32,
) {
    let sec_start = start_time.floor() as i32;
    let sec_end = (start_time + visible_secs).ceil() as i32;
    for sec in sec_start..=sec_end {
        let x = center_x + (sec as f32 - position_secs as f32) * pixels_per_sec;
        if x < 0.0 || x >= width as f32 {
            continue;
        }
        let xi = x.round() as usize;
        if xi < width {
            blend_vertical_line(rgba, width, height, xi, [255, 255, 255, 18]);
        }
    }
}

fn time_to_x(
    time: f32,
    position_secs: f64,
    pixels_per_sec: f32,
    center_x: f32,
    width: usize,
) -> Option<usize> {
    let x = center_x + (f64::from(time) - position_secs) as f32 * pixels_per_sec;
    if x < 0.0 || x >= width as f32 {
        return None;
    }
    let xi = x.round() as usize;
    (xi < width).then_some(xi)
}

fn draw_edge_ticks(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    tick_px: usize,
    color: [u8; 4],
) {
    let tick_px = tick_px.max(1);
    for dy in 0..tick_px {
        set_pixel(rgba, width, height, x, dy, color);
        set_pixel(rgba, width, height, x, height.saturating_sub(1 + dy), color);
    }
}

fn draw_bar_markers(rgba: &mut [u8], width: usize, height: usize, x: usize, color: [u8; 4]) {
    let r = 4isize;
    for cy in [0isize, (height.saturating_sub(1)) as isize] {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() + dy.abs() > r {
                    continue;
                }
                let px = x as isize + dx;
                let py = cy + dy;
                if px < 0 || py < 0 {
                    continue;
                }
                set_pixel(rgba, width, height, px as usize, py as usize, color);
            }
        }
    }
}

fn draw_vertical_bar(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    mid_y: f32,
    bar_h: f32,
    r: u8,
    g: u8,
    b: u8,
    alpha: f32,
) {
    let top = (mid_y - bar_h).max(0.0).floor() as usize;
    let bottom = (mid_y + bar_h).min(height as f32 - 1.0).ceil() as usize;
    for y in top..=bottom.min(height.saturating_sub(1)) {
        blend_pixel(rgba, width, x, y, [r, g, b, (alpha * 255.0) as u8]);
    }
}

fn peak_at_time(
    overview: &[SpectralPeak],
    detail: Option<&DetailWindow>,
    duration_secs: f64,
    time: f64,
) -> SpectralPeak {
    if let Some(window) = detail {
        if time >= window.start_secs && time <= window.end_secs && window.peaks.len() > 1 {
            let span = window.end_secs - window.start_secs;
            if span > 0.0 {
                let frac = (time - window.start_secs) / span;
                let idx = frac * (window.peaks.len() - 1) as f64;
                return interpolate_peak(&window.peaks, idx);
            }
        }
    }

    let frac = (time / duration_secs).clamp(0.0, 1.0);
    let idx = frac * (overview.len().saturating_sub(1)) as f64;
    interpolate_peak(overview, idx)
}

fn interpolate_peak(peaks: &[SpectralPeak], index: f64) -> SpectralPeak {
    if peaks.is_empty() {
        return SpectralPeak {
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        };
    }
    let clamped = index.clamp(0.0, peaks.len() as f64 - 1.0);
    let i0 = clamped.floor() as usize;
    let i1 = (i0 + 1).min(peaks.len() - 1);
    let t = (clamped - i0 as f64) as f32;
    let p0 = peaks[i0];
    let p1 = peaks[i1];
    SpectralPeak {
        low: p0.low * (1.0 - t) + p1.low * t,
        mid: p0.mid * (1.0 - t) + p1.mid * t,
        high: p0.high * (1.0 - t) + p1.high * t,
    }
}

fn spectral_rgb(low: f32, mid: f32, high: f32) -> (u8, u8, u8) {
    let total = low + mid + high + 1e-6;
    let l = low / total;
    let m = mid / total;
    let h = high / total;
    let r = l * LOW_RGB[0] + m * MID_RGB[0] + h * HIGH_RGB[0];
    let g = l * LOW_RGB[1] + m * MID_RGB[1] + h * HIGH_RGB[1];
    let b = l * LOW_RGB[2] + m * MID_RGB[2] + h * HIGH_RGB[2];
    (r.round() as u8, g.round() as u8, b.round() as u8)
}

fn set_pixel(rgba: &mut [u8], width: usize, height: usize, x: usize, y: usize, color: [u8; 4]) {
    if x >= width || y >= height {
        return;
    }
    let idx = (y * width + x) * 4;
    rgba[idx..idx + 4].copy_from_slice(&color);
}

fn blend_pixel(rgba: &mut [u8], width: usize, x: usize, y: usize, color: [u8; 4]) {
    if x >= width {
        return;
    }
    let height = rgba.len() / (width * 4);
    if y >= height {
        return;
    }
    let idx = (y * width + x) * 4;
    let alpha = color[3] as f32 / 255.0;
    for c in 0..3 {
        let dst = rgba[idx + c] as f32;
        let src = color[c] as f32;
        rgba[idx + c] = (dst * (1.0 - alpha) + src * alpha) as u8;
    }
}

fn blend_vertical_line(rgba: &mut [u8], width: usize, height: usize, x: usize, color: [u8; 4]) {
    if x >= width {
        return;
    }
    for y in 0..height {
        blend_pixel(rgba, width, x, y, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn even_grid_uses_constant_period() {
        let grid = BeatGridSnapshot {
            beats: vec![0.2, 0.7, 1.2],
            bars: vec![],
            downbeats: vec![],
            bpm: Some(120.0),
        };
        let (period, phase) = resolve_even_grid(&grid).expect("grid");
        assert!((period - 0.5).abs() < 1e-9);
        assert!((phase - f64::from(0.2_f32)).abs() < 1e-9);
    }

    #[test]
    fn render_produces_expected_buffer_size() {
        let overview = vec![
            SpectralPeak {
                low: 0.2,
                mid: 0.5,
                high: 0.8,
            };
            64
        ];
        let rgba = render_scrolling_lane(
            120,
            40,
            &overview,
            None,
            60.0,
            30.0,
            24.0,
            WaveformDisplayGains::default(),
            None,
            false,
        );
        assert_eq!(rgba.len(), 120 * 40 * 4);
    }
}
