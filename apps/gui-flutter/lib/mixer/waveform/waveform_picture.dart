import 'dart:ui';

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';

class WaveformBarPainter extends CustomPainter {
  WaveformBarPainter({
    required this.overview,
    required this.detail,
    required this.durationMs,
    required this.originMs,
    required this.spanMs,
    this.fallbackToOverview = true,
    this.fillBackground = true,
  });

  final List<SpectralPeak> overview;
  final DetailWindow? detail;
  final int durationMs;
  final double originMs;
  final double spanMs;
  final bool fallbackToOverview;
  final bool fillBackground;

  @override
  void paint(Canvas canvas, Size size) {
    if (fillBackground) {
      canvas.drawRect(Offset.zero & size, Paint()..color = kWaveformBg);
    }
    if (overview.isEmpty || durationMs <= 0 || spanMs <= 0 || size.width <= 0) {
      return;
    }
    final midY = size.height / 2;
    final maxAmp = size.height * 0.46;
    final width = size.width.floor();
    final paint = Paint()..isAntiAlias = false;
    for (var x = 0; x < width; x++) {
      final timeMs = originMs + (x / size.width) * spanMs;
      if (timeMs < 0 || timeMs > durationMs) {
        continue;
      }
      final peak = peakAtTime(
        overview,
        detail,
        durationMs,
        timeMs,
        fallbackToOverview: fallbackToOverview,
      );
      final amp = peak.low > peak.mid
          ? (peak.low > peak.high ? peak.low : peak.high)
          : (peak.mid > peak.high ? peak.mid : peak.high);
      if (amp <= 0.001) {
        continue;
      }
      paint.color = barFill(spectralRgb(peak.low, peak.mid, peak.high), amp);
      final barH = amp * maxAmp;
      canvas.drawRect(
        Rect.fromLTRB(x.toDouble(), midY - barH, x + 1.0, midY + barH),
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(WaveformBarPainter oldDelegate) =>
      !identical(overview, oldDelegate.overview) ||
      !identical(detail, oldDelegate.detail) ||
      durationMs != oldDelegate.durationMs ||
      originMs != oldDelegate.originMs ||
      spanMs != oldDelegate.spanMs ||
      fallbackToOverview != oldDelegate.fallbackToOverview ||
      fillBackground != oldDelegate.fillBackground;
}

Picture recordWaveformPicture({
  required List<SpectralPeak> overview,
  DetailWindow? detail,
  required int durationMs,
  required double originMs,
  required double spanMs,
  required Size size,
  bool fallbackToOverview = true,
  bool fillBackground = true,
}) {
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  WaveformBarPainter(
    overview: overview,
    detail: detail,
    durationMs: durationMs,
    originMs: originMs,
    spanMs: spanMs,
    fallbackToOverview: fallbackToOverview,
    fillBackground: fillBackground,
  ).paint(canvas, size);
  return recorder.endRecording();
}
