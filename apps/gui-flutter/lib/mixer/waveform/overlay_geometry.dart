import 'dart:ui';

/// Maps a track time into a picture whose left edge is [originMs] and
/// width spans [spanMs].
double msToX({
  required num ms,
  required double originMs,
  required double spanMs,
  required double width,
}) {
  if (!(spanMs > 0) || !(width > 0)) {
    return 0;
  }
  return ((ms.toDouble() - originMs) / spanMs).clamp(0.0, 1.0) * width;
}

/// Full-height loop region in overlay / strip space, or null if invalid.
Rect? loopRegionRect({
  required int inMs,
  required int outMs,
  required double originMs,
  required double spanMs,
  required double width,
  required double height,
}) {
  if (!(spanMs > 0) || width <= 0 || height <= 0 || outMs <= inMs) {
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
