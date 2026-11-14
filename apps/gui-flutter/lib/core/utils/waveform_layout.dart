// Layout helpers only — no widget or controller is constructed here.
import 'dart:math' as math;

/// Default source time on screen before tempo scaling.
const kWaveformVisibleMs = 24000;

/// User zoom range for the source time on screen.
const kWaveformMinVisibleMs = 2000;
const kWaveformMaxVisibleMs = 120000;

/// Exponent per scroll pixel: one mouse notch (~53 px) zooms ~1.17x.
const kWaveformZoomSensitivity = 0.003;

/// How far inside a strip's edges the playhead may reach before the next
/// window is fetched, as a fraction of the visible span.
///
/// The lane draws the playhead at its centre, so the viewport reaches *half* a
/// visible span past the playhead on each side. A strip therefore only covers
/// the screen while the playhead stays at least `0.5` visible spans inside its
/// edges: refreshing later than that leaves the strip's last pixels short of
/// the lane edge and paints a blank band there until the position moves far
/// enough to queue a fetch. The extra `0.25` is lead time for the async fetch,
/// so the replacement normally lands before the old strip runs out.
const kWaveformRefreshMargin = 0.75;
const kWaveformSeekSnapMs = 180.0;
const kWaveformDriftCorrectMs = 60.0;

const kWaveformStripHeight = 128.0;

/// Playback-ratio factor for waveform zoom (faster → show more source time).
double waveformSpeedScale(double speed) {
  final clamped = speed.isFinite && speed > 0 ? speed : 1.0;
  return clamped.clamp(0.5, 2.0);
}

/// Source time on screen for [visibleMs] at [speed], so tempo changes zoom the
/// viewport instead of changing the musical span.
int visibleSourceMs(double speed, [int visibleMs = kWaveformVisibleMs]) =>
    (visibleMs * waveformSpeedScale(speed)).round();

/// Applies one scroll event to the visible span: scroll up zooms in.
int zoomedVisibleMs({required int visibleMs, required double scrollDeltaY}) {
  if (scrollDeltaY == 0 || !scrollDeltaY.isFinite) {
    return visibleMs;
  }
  final next = (visibleMs * math.exp(scrollDeltaY * kWaveformZoomSensitivity))
      .round();
  return next.clamp(kWaveformMinVisibleMs, kWaveformMaxVisibleMs);
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

double stripTranslateX({
  required double positionMs,
  required int originMs,
  required double viewportWidth,
  required double pxPerMs,
}) => playheadDx(
  positionMs: positionMs,
  originMs: originMs.toDouble(),
  width: viewportWidth,
  pxPerMs: pxPerMs,
);

/// Engine estimate at this frame: last poll plus time elapsed at [speed].
double engineEstimateMs({
  required double anchorMs,
  required double ageMs,
  required double speed,
}) => anchorMs + ageMs * speed;

/// Wall-clock duration for an `AnimationController` whose 0..1 value is
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

/// Wall-clock duration for gliding a source-millisecond drift correction.
///
/// The correction stays linear and never faster than a quarter of normal
/// source-time velocity, so a slow deck gets a proportionally longer glide
/// instead of a visible jump. A zero correction still reports the 100 ms
/// floor; callers skip scheduling it.
Duration playheadDriftDuration({
  required double deltaMs,
  required double speed,
}) {
  const ceiling = Duration(milliseconds: 86400000);
  final s = speed.isFinite && speed > 0 ? speed : 1.0;
  if (!deltaMs.isFinite) {
    return ceiling;
  }
  final raw = deltaMs.abs() / (0.25 * s);
  // The quotient can be infinite (a denormal speed) or larger than an int can
  // hold; `ceil()` throws in the first case and the Duration conversion
  // overflows in the second.
  if (!raw.isFinite || raw >= ceiling.inMilliseconds) {
    return ceiling;
  }
  return Duration(milliseconds: math.max(100, raw.ceil()));
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
  ).clamp(0, durationMs);
  final end = l1EndMs(
    positionMs: positionMs,
    visibleMs: visibleMs,
  ).clamp(start, durationMs);
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
  // A window that no longer covers the viewport paints a blank band on the
  // lane, so it must never survive a build. Checking it directly (instead of
  // trusting the margin) keeps the invariant even for windows clamped by the
  // track edges or shorter than the nominal three visible spans.
  if (!l1CoversVisible(
    positionMs: positionMs,
    visibleMs: visibleMs,
    startMs: detailStartMs,
    endMs: detailEndMs,
    durationMs: durationMs,
  )) {
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
