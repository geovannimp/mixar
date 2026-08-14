import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';

class WaveformBarPainter extends CustomPainter {
  // ponytail: CustomPaint display-list + RepaintBoundary, not a recorded
  // ui.Picture. Upgrade: record Picture on data/size/L1 and scroll with
  // Transform so playhead ticks don't rebuild the lane State.
  WaveformBarPainter({
    required this.overview,
    required this.detail,
    required this.durationMs,
    required this.originMs,
    required this.spanMs,
  });

  final List<SpectralPeak> overview;
  final DetailWindow? detail;
  final int durationMs;
  final double originMs;
  final double spanMs;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(
      Offset.zero & size,
      Paint()..color = const Color.fromARGB(255, 5, 5, 8),
    );
    if (overview.isEmpty || durationMs <= 0 || spanMs <= 0 || size.width <= 0) {
      return;
    }
    final midY = size.height / 2;
    final maxAmp = size.height * 0.46;
    final width = size.width.floor();
    final paint = Paint();
    for (var x = 0; x < width; x++) {
      final timeMs = originMs + (x / size.width) * spanMs;
      if (timeMs < 0 || timeMs > durationMs) {
        continue;
      }
      final peak = peakAtTime(overview, detail, durationMs, timeMs);
      final amp = peak.low > peak.mid
          ? (peak.low > peak.high ? peak.low : peak.high)
          : (peak.mid > peak.high ? peak.mid : peak.high);
      if (amp <= 0.001) {
        continue;
      }
      final color = spectralRgb(peak.low, peak.mid, peak.high);
      paint.color = color.withValues(alpha: barAlpha(amp));
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
      spanMs != oldDelegate.spanMs;
}
