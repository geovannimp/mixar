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

// Lit bands match Tauri emerald/amber/red; idle uses Forui muted (readable on FCard).
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
    switch (mode) {
      case LevelMeterMode.mono:
        final peak = math.max(levels.peakL, levels.peakR);
        final hold = math.max(levels.peakHoldL, levels.peakHoldR);
        return _Ladder(peak: peak, hold: hold);
      case LevelMeterMode.stereo:
        return Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _Ladder(peak: levels.peakL, hold: levels.peakHoldL),
            const SizedBox(width: 1),
            _Ladder(peak: levels.peakR, hold: levels.peakHoldR),
          ],
        );
    }
  }
}

class _Ladder extends StatelessWidget {
  const _Ladder({required this.peak, required this.hold});

  final double peak;
  final double hold;

  @override
  Widget build(BuildContext context) {
    final off = context.theme.colors.muted;
    final holdIdx = holdSegment(hold);
    // Top → bottom visually: high indices first so fromBottom 0 sits at bottom.
    // DecoratedBox with no child sizes to constraints.smallest (width 0) — expand.
    return SizedBox(
      width: 6,
      child: Column(
        children: [
          for (
            var fromTop = kLevelMeterSegments - 1;
            fromTop >= 0;
            fromTop--
          ) ...[
            if (fromTop < kLevelMeterSegments - 1) const SizedBox(height: 1),
            Expanded(
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: _segmentColor(
                    off,
                    fromTop,
                    lit: segmentOn(peak, fromTop) || holdIdx == fromTop,
                  ),
                  borderRadius: BorderRadius.circular(1),
                ),
                child: const SizedBox.expand(),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
