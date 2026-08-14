import 'dart:ui';

const kWaveformVisibleMs = 24000;
const kWaveformBufferRatio = 1.0;
const kWaveformRefreshMargin = 0.35;

int visibleSourceMs(double speed) {
  final clamped = speed.isFinite && speed > 0 ? speed : 1.0;
  return (kWaveformVisibleMs * clamped.clamp(0.5, 2.0)).round();
}

Rect overviewWindowRect({
  required int positionMs,
  required int durationMs,
  required int visibleMs,
}) {
  if (durationMs <= 0) {
    return Rect.zero;
  }
  final half = visibleMs / 2;
  final left = ((positionMs - half) / durationMs).clamp(0.0, 1.0);
  final right = ((positionMs + half) / durationMs).clamp(0.0, 1.0);
  return Rect.fromLTRB(left, 0, right, 1);
}

double centerScrubMs({
  required double anchorPosMs,
  required double deltaX,
  required double width,
  required double spanMs,
}) {
  return anchorPosMs - (deltaX / width.clamp(1, double.infinity)) * spanMs;
}

double playheadDx({
  required double positionMs,
  required double originMs,
  required double width,
  required double pxPerMs,
}) {
  return width / 2 - (positionMs - originMs) * pxPerMs;
}

int l1StartMs({required int positionMs, required int visibleMs}) =>
    positionMs - (visibleMs * 3 / 2).round();

int l1EndMs({required int positionMs, required int visibleMs}) =>
    positionMs + (visibleMs * 3 / 2).round();
