import 'dart:ui';

/// Maps a track time to an x position in full-track overlay / strip space.
double msToX({
  required int ms,
  required int durationMs,
  required double width,
}) {
  if (durationMs <= 0 || width <= 0) {
    return 0;
  }
  return (ms / durationMs).clamp(0.0, 1.0) * width;
}

/// Full-height loop region in overlay / strip space, or null if invalid.
Rect? loopRegionRect({
  required int inMs,
  required int outMs,
  required int durationMs,
  required double width,
  required double height,
}) {
  if (durationMs <= 0 || width <= 0 || height <= 0 || outMs <= inMs) {
    return null;
  }
  final left = msToX(ms: inMs, durationMs: durationMs, width: width);
  final right = msToX(ms: outMs, durationMs: durationMs, width: width);
  final w = right - left;
  if (w <= 0) {
    return null;
  }
  return Rect.fromLTWH(left, 0, w, height);
}
