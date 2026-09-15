const kWaveformVisibleMs = 24000;
const kWaveformSeekSnapMs = 180.0;
const kWaveformDriftCorrectMs = 60.0;
const kWaveformStripMsPerPx = 13.0;
const kWaveformStripMinPx = 2048;
const kWaveformStripMaxPx = 16384;
const kWaveformStripHeight = 128.0;

/// Viewport ring: 8 chunks, 4 visible, slide by 2.
const kWaveformRingChunks = 8;
const kWaveformRingVisibleChunks = 4;
const kWaveformRingSlideChunks = 2;

int ringChunkPx(double windowWidth) {
  if (!(windowWidth > 0) || !windowWidth.isFinite) {
    return 1;
  }
  return (windowWidth / kWaveformRingVisibleChunks).floor().clamp(1, 1 << 20);
}

/// Centered ring time range clamped to the track.
({double originMs, double chunkMs, int chunkCount}) ringRange({
  required double positionMs,
  required double visibleMs,
  required int durationMs,
}) {
  if (durationMs <= 0 || !(visibleMs > 0)) {
    return (originMs: 0, chunkMs: 0, chunkCount: 0);
  }
  final chunkMs = visibleMs / kWaveformRingVisibleChunks;
  if (!(chunkMs > 0)) {
    return (originMs: 0, chunkMs: 0, chunkCount: 0);
  }
  final maxChunks = (durationMs / chunkMs).floor().clamp(
    1,
    kWaveformRingChunks,
  );
  final spanMs = chunkMs * maxChunks;
  var origin = positionMs - (chunkMs * kWaveformRingChunks) / 2;
  if (origin < 0) {
    origin = 0;
  }
  if (origin + spanMs > durationMs) {
    origin = (durationMs - spanMs).clamp(0, durationMs.toDouble());
  }
  return (originMs: origin, chunkMs: chunkMs, chunkCount: maxChunks);
}

bool ringNeedsSlide({
  required double positionMs,
  required double ringOriginMs,
  required double chunkMs,
  required int chunkCount,
  required int durationMs,
}) {
  if (chunkMs <= 0 || chunkCount <= 0 || durationMs <= 0) {
    return false;
  }
  final half = chunkMs * chunkCount / 2;
  final center = ringOriginMs + half;
  final delta = positionMs - center;
  final threshold = chunkMs * kWaveformRingSlideChunks;
  if (delta.abs() < threshold) {
    return false;
  }
  // No slide past ends when the ring already covers the track edge.
  if (delta > 0) {
    final end = ringOriginMs + chunkMs * chunkCount;
    if (end >= durationMs - 1e-6) {
      return false;
    }
  } else if (ringOriginMs <= 1e-6) {
    return false;
  }
  return true;
}

/// New origin after a ±2-chunk slide toward [positionMs], clamped.
double ringSlideOriginMs({
  required double ringOriginMs,
  required double chunkMs,
  required int chunkCount,
  required double positionMs,
  required int durationMs,
}) {
  if (chunkMs <= 0 || chunkCount <= 0 || durationMs <= 0) {
    return ringOriginMs;
  }
  final half = chunkMs * chunkCount / 2;
  final center = ringOriginMs + half;
  final step = chunkMs * kWaveformRingSlideChunks;
  final delta = positionMs - center;
  final next = delta >= 0 ? ringOriginMs + step : ringOriginMs - step;
  final spanMs = chunkMs * chunkCount;
  return next.clamp(0.0, (durationMs - spanMs).clamp(0, durationMs).toDouble());
}

int visibleSourceMs(double speed) {
  return (kWaveformVisibleMs * waveformSpeedScale(speed)).round();
}

/// Playback-ratio factor for waveform zoom (faster → show more source time).
double waveformSpeedScale(double speed) {
  final clamped = speed.isFinite && speed > 0 ? speed : 1.0;
  return clamped.clamp(0.5, 2.0);
}

/// Strip density scaled so tempo changes zoom the scrolling viewport.
double stripDisplayPxPerMs({required double pxPerMs, required double speed}) =>
    pxPerMs / waveformSpeedScale(speed);

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

int cropVisibleMs({
  required int durationMs,
  required double viewportWidth,
  double speed = 1.0,
}) {
  final px = stripDisplayPxPerMs(
    pxPerMs: stripPxPerMs(durationMs),
    speed: speed,
  );
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
