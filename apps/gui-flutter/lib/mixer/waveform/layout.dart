import 'dart:ui';

const kWaveformVisibleMs = 24000;
const kWaveformBufferRatio = 1.0;
const kWaveformRefreshMargin = 0.35;
const kWaveformSeekSnapMs = 180.0;
const kWaveformDriftCorrectMs = 60.0;
const kWaveformStripMsPerPx = 13.0;
const kWaveformStripMinPx = 2048;
const kWaveformStripMaxPx = 16384;
const kWaveformStripHeight = 128.0;
const kWaveformStripTilePx = 2048;

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

double snapPx(double x, double dpr) {
  final scale = dpr > 0 ? dpr : 1.0;
  return (x * scale).round() / scale;
}

int stripWidthPx(int durationMs) {
  if (durationMs <= 0) {
    return kWaveformStripMinPx;
  }
  return (durationMs / kWaveformStripMsPerPx).ceil().clamp(
    kWaveformStripMinPx,
    kWaveformStripMaxPx,
  );
}

double stripPxPerMs(int durationMs) {
  final width = stripWidthPx(durationMs);
  if (durationMs <= 0) {
    return 0;
  }
  return width / durationMs;
}

double stripTranslateX({
  required double positionMs,
  required double viewportWidth,
  required double pxPerMs,
}) => viewportWidth / 2 - positionMs * pxPerMs;

int cropVisibleMs({required int durationMs, required double viewportWidth}) {
  final px = stripPxPerMs(durationMs);
  if (px <= 0) {
    return kWaveformVisibleMs;
  }
  return (viewportWidth / px).round().clamp(1, durationMs);
}

/// Engine estimate at this frame: last poll plus time elapsed at [speed].
double engineEstimateMs({
  required double anchorMs,
  required double ageMs,
  required double speed,
}) => anchorMs + ageMs * speed;

/// Wall-clock duration for an [AnimationController] whose 0..1 value is
/// track progress, so `forward()` reaches the end in `durationMs / speed`.
Duration playheadWallDuration({
  required int durationMs,
  required double speed,
}) {
  if (durationMs <= 0) {
    return const Duration(milliseconds: 1);
  }
  final s = speed.isFinite && speed > 0 ? speed : 1.0;
  return Duration(milliseconds: (durationMs / s).round().clamp(1, 86400000));
}

/// Keep interpolating; only pull 25% when drift exceeds [kWaveformDriftCorrectMs].
/// A late ~33ms position poll must not yank the playhead backward.
double correctPlayheadDrift({
  required double displayMs,
  required double estimateMs,
}) {
  final error = estimateMs - displayMs;
  if (error.abs() > kWaveformDriftCorrectMs) {
    return displayMs + error * 0.25;
  }
  return displayMs;
}

bool playheadShouldSnap({
  required double displayMs,
  required double engineMs,
  required bool playing,
}) => !playing || (displayMs - engineMs).abs() >= kWaveformSeekSnapMs;

/// Vinyl touch zeros `jog_rate` but leaves `playing` true, so interpolation
/// must treat touch like pause or the lane keeps scrolling with no audio.
bool playheadAdvancing({required bool playing, required bool jogTouching}) =>
    playing && !jogTouching;

int l1StartMs({required int positionMs, required int visibleMs}) =>
    positionMs - (visibleMs * 3 / 2).round();

int l1EndMs({required int positionMs, required int visibleMs}) =>
    positionMs + (visibleMs * 3 / 2).round();

({int startMs, int endMs}) l1Range({
  required int positionMs,
  required int visibleMs,
  required int durationMs,
}) {
  if (durationMs <= 0) {
    return (startMs: 0, endMs: 0);
  }
  final start = l1StartMs(
    positionMs: positionMs,
    visibleMs: visibleMs,
  ).clamp(0, durationMs).toInt();
  final end = l1EndMs(
    positionMs: positionMs,
    visibleMs: visibleMs,
  ).clamp(start, durationMs).toInt();
  return (startMs: start, endMs: end);
}

/// Keep ~1 bucket per viewport pixel so L1 swaps don't change peak density.
int l1BucketCount({
  required int startMs,
  required int endMs,
  required int visibleMs,
  required double width,
}) {
  if (visibleMs <= 0 || width <= 0 || endMs <= startMs) {
    return 16;
  }
  return ((endMs - startMs) / visibleMs * width).round().clamp(16, 16384);
}

bool l1CoversVisible({
  required double positionMs,
  required int visibleMs,
  required int startMs,
  required int endMs,
  int durationMs = 1 << 30,
}) {
  if (visibleMs <= 0 || endMs <= startMs) {
    return false;
  }
  final half = visibleMs / 2;
  final viewStart = (positionMs - half).clamp(0, durationMs.toDouble());
  final viewEnd = (positionMs + half).clamp(0, durationMs.toDouble());
  return startMs <= viewStart && endMs >= viewEnd;
}

bool l1NeedsRefresh({
  required double positionMs,
  required int? detailStartMs,
  required int? detailEndMs,
  required int visibleMs,
  required int durationMs,
}) {
  if (detailStartMs == null || detailEndMs == null) {
    return true;
  }
  if (detailEndMs <= detailStartMs || visibleMs <= 0) {
    return true;
  }
  final margin = visibleMs * kWaveformRefreshMargin;
  final nearStart = positionMs < detailStartMs + margin;
  final nearEnd = positionMs > detailEndMs - margin;
  if (!nearStart && !nearEnd) {
    return false;
  }
  final canSlideStart = detailStartMs > 0;
  final canSlideEnd = detailEndMs < durationMs;
  if (nearStart && !canSlideStart && !(nearEnd && canSlideEnd)) {
    return false;
  }
  if (nearEnd && !canSlideEnd && !(nearStart && canSlideStart)) {
    return false;
  }
  return true;
}
