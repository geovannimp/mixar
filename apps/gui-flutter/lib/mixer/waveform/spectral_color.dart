import 'dart:ui';

const kLowRgb = [255.0, 72.0, 48.0];
const kMidRgb = [118.0, 228.0, 88.0];
const kHighRgb = [72.0, 188.0, 255.0];

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
