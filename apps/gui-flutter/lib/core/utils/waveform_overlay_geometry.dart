import 'dart:ui';

/// Maps a track time to an x position inside an overlay picture spanning
/// `[originMs, originMs + spanMs]`.
double msToX({
  required int ms,
  required int originMs,
  required int spanMs,
  required double width,
}) {
  if (spanMs <= 0 || width <= 0) {
    return 0;
  }
  return ((ms - originMs) / spanMs).clamp(0.0, 1.0) * width;
}

/// Full-height loop region in overlay space, or null if invalid.
Rect? loopRegionRect({
  required int inMs,
  required int outMs,
  required int originMs,
  required int spanMs,
  required double width,
  required double height,
}) {
  if (spanMs <= 0 || width <= 0 || height <= 0 || outMs <= inMs) {
    return null;
  }
  final left = msToX(
    ms: inMs,
    originMs: originMs,
    spanMs: spanMs,
    width: width,
  );
  final right = msToX(
    ms: outMs,
    originMs: originMs,
    spanMs: spanMs,
    width: width,
  );
  final w = right - left;
  if (w <= 0) {
    return null;
  }
  return Rect.fromLTWH(left, 0, w, height);
}
