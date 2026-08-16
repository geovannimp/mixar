import 'dart:ui';

import 'package:gui_flutter/mixer/waveform/peaks.dart';

const kLowRgb = [255.0, 72.0, 48.0];
const kMidRgb = [118.0, 228.0, 88.0];
const kHighRgb = [72.0, 188.0, 255.0];

const kLowColor = Color.fromARGB(255, 255, 72, 48);
const kMidColor = Color.fromARGB(255, 118, 228, 88);
const kHighColor = Color.fromARGB(255, 72, 188, 255);

const kWaveformBg = Color.fromARGB(255, 5, 5, 8);

enum WaveformDisplayMode { rgb, filtered }

class FilteredBar {
  const FilteredBar({required this.height, required this.color});

  final double height;
  final Color color;
}

double peakAmp(SpectralPeak peak) {
  final a = peak.low > peak.mid ? peak.low : peak.mid;
  return a > peak.high ? a : peak.high;
}

/// Mixxx Filtered: three centered bars, low (widest) then mid then high on top.
List<FilteredBar> filteredBars(SpectralPeak peak, double maxAmp) => [
  FilteredBar(height: peak.low * maxAmp, color: kLowColor),
  FilteredBar(height: peak.mid * maxAmp, color: kMidColor),
  FilteredBar(height: peak.high * maxAmp, color: kHighColor),
];

List<FilteredBar> waveformBars(
  SpectralPeak peak,
  double maxAmp,
  WaveformDisplayMode mode,
) {
  switch (mode) {
    case WaveformDisplayMode.filtered:
      return filteredBars(peak, maxAmp);
    case WaveformDisplayMode.rgb:
      final amp = peakAmp(peak);
      if (amp <= 0.001) {
        return const [];
      }
      return [
        FilteredBar(
          height: amp * maxAmp,
          color: barFill(spectralRgb(peak.low, peak.mid, peak.high), amp),
        ),
      ];
  }
}

Color spectralRgb(double low, double mid, double high) {
  final total = low + mid + high + 1e-6;
  final l = low / total;
  final m = mid / total;
  final h = high / total;
  return Color.fromARGB(
    255,
    (l * kLowRgb[0] + m * kMidRgb[0] + h * kHighRgb[0]).round(),
    (l * kLowRgb[1] + m * kMidRgb[1] + h * kHighRgb[1]).round(),
    (l * kLowRgb[2] + m * kMidRgb[2] + h * kHighRgb[2]).round(),
  );
}

double barAlpha(double amp) => 0.65 + amp * 0.35;

/// Premultiply [barAlpha] onto the waveform background so 1px bars stay opaque.
Color barFill(Color rgb, double amp) {
  final a = barAlpha(amp).clamp(0.0, 1.0);
  final ia = 1 - a;
  return Color.fromARGB(
    255,
    (rgb.r * 255.0 * a + 5.0 * ia).round().clamp(0, 255),
    (rgb.g * 255.0 * a + 5.0 * ia).round().clamp(0, 255),
    (rgb.b * 255.0 * a + 8.0 * ia).round().clamp(0, 255),
  );
}
