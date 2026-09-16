import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

enum LevelMeterMode { mono, stereo }

class DeckLevels {
  const DeckLevels({
    required this.peakL,
    required this.peakR,
    required this.peakHoldL,
    required this.peakHoldR,
  });

  final double peakL;
  final double peakR;
  final double peakHoldL;
  final double peakHoldR;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DeckLevels &&
          peakL == other.peakL &&
          peakR == other.peakR &&
          peakHoldL == other.peakHoldL &&
          peakHoldR == other.peakHoldR;

  @override
  int get hashCode => Object.hash(peakL, peakR, peakHoldL, peakHoldR);
}

const zeroDeckLevels = DeckLevels(
  peakL: 0,
  peakR: 0,
  peakHoldL: 0,
  peakHoldR: 0,
);

const kLevelMeterSegments = 12;
const kLevelMeterYellowFrom = 8;
const kLevelMeterRedFrom = 10;

bool segmentOn(double level, int indexFromBottom) {
  final threshold = (indexFromBottom + 1) / kLevelMeterSegments;
  return level >= threshold - 1e-6;
}

/// Bottom segment covers [1/SEGMENTS, 2/SEGMENTS). Tiny residual hold must not light.
int? holdSegment(double hold) {
  if (hold < 1 / kLevelMeterSegments - 1e-6) return null;
  return math.min(
    kLevelMeterSegments - 1,
    (hold * kLevelMeterSegments).ceil() - 1,
  );
}

final _green = const Color(
  0xff10b981,
).withValues(alpha: 0.45); // emerald-500/45
final _amber = const Color(0xfffbbf24).withValues(alpha: 0.45); // amber-400/45
final _red = const Color(0xffef4444).withValues(alpha: 0.50); // red-500/50

Color _segmentColor(Color off, int fromBottom, {required bool lit}) {
  if (!lit) return off;
  if (fromBottom >= kLevelMeterRedFrom) return _red;
  if (fromBottom >= kLevelMeterYellowFrom) return _amber;
  return _green;
}

/// Vertical LED ladder matching Tauri `LevelMeter` / `Ladder`.
class LevelMeter extends StatelessWidget {
  const LevelMeter({required this.levels, required this.mode, super.key});

  final DeckLevels levels;
  final LevelMeterMode mode;

  @override
  Widget build(BuildContext context) {
    final off = context.theme.colors.muted;
    switch (mode) {
      case LevelMeterMode.mono:
        final peak = math.max(levels.peakL, levels.peakR);
        final hold = math.max(levels.peakHoldL, levels.peakHoldR);
        return _Ladder(peak: peak, hold: hold, off: off);
      case LevelMeterMode.stereo:
        return Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _Ladder(peak: levels.peakL, hold: levels.peakHoldL, off: off),
            const SizedBox(width: 1),
            _Ladder(peak: levels.peakR, hold: levels.peakHoldR, off: off),
          ],
        );
    }
  }
}

class _Ladder extends StatelessWidget {
  const _Ladder({required this.peak, required this.hold, required this.off});

  final double peak;
  final double hold;
  final Color off;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 6,
      child: CustomPaint(
        painter: _LadderPainter(peak: peak, hold: hold, off: off),
        child: const SizedBox.expand(),
      ),
    );
  }
}

class _LadderPainter extends CustomPainter {
  _LadderPainter({required this.peak, required this.hold, required this.off});

  final double peak;
  final double hold;
  final Color off;

  @override
  void paint(Canvas canvas, Size size) {
    const gap = 1.0;
    final holdIdx = holdSegment(hold);
    final segH =
        (size.height - gap * (kLevelMeterSegments - 1)) / kLevelMeterSegments;
    if (segH <= 0 || size.width <= 0) {
      return;
    }
    final paint = Paint()..isAntiAlias = false;
    final radius = Radius.circular(1);
    for (var fromTop = 0; fromTop < kLevelMeterSegments; fromTop++) {
      final fromBottom = kLevelMeterSegments - 1 - fromTop;
      final top = fromTop * (segH + gap);
      paint.color = _segmentColor(
        off,
        fromBottom,
        lit: segmentOn(peak, fromBottom) || holdIdx == fromBottom,
      );
      canvas.drawRRect(
        RRect.fromRectAndRadius(
          Rect.fromLTWH(0, top, size.width, segH),
          radius,
        ),
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _LadderPainter oldDelegate) =>
      peak != oldDelegate.peak ||
      hold != oldDelegate.hold ||
      off != oldDelegate.off;
}
