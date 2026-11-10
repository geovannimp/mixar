import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/utils/waveform_beat_grid.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';

void main() {
  test('decodeRgbPeaks maps uint8 triples to 0..1 peaks', () {
    final peaks = decodeRgbPeaks([64, 128, 255, 0, 0, 0]);
    expect(peaks, hasLength(2));
    expect(peaks[0].low, closeTo(64 / 255, 1e-6));
    expect(peaks[0].mid, closeTo(128 / 255, 1e-6));
    expect(peaks[0].high, closeTo(1.0, 1e-6));
    expect(peaks[1].low, 0);
  });

  test('decodeRgbPeaks rejects truncated payloads', () {
    expect(decodeRgbPeaks([1, 2]), isEmpty);
  });

  test('l1BucketCount stays one bucket per viewport pixel', () {
    expect(
      l1BucketCount(startMs: 0, endMs: 36_000, visibleMs: 24_000, width: 1920),
      2880,
    );
    expect(
      l1BucketCount(startMs: 0, endMs: 72_000, visibleMs: 24_000, width: 1920),
      5760,
    );
  });

  test('l1CoversVisible is true when the window contains the viewport', () {
    expect(
      l1CoversVisible(
        positionMs: 0,
        visibleMs: 24_000,
        startMs: 0,
        endMs: 36_000,
        durationMs: 180_000,
      ),
      isTrue,
    );
    expect(
      l1CoversVisible(
        positionMs: 12_000,
        visibleMs: 24_000,
        startMs: 0,
        endMs: 36_000,
      ),
      isTrue,
    );
    expect(
      l1CoversVisible(
        positionMs: 30_000,
        visibleMs: 24_000,
        startMs: 0,
        endMs: 36_000,
      ),
      isFalse,
    );
  });

  test('visibleSourceMs scales with speed and clamps', () {
    expect(visibleSourceMs(1), kWaveformVisibleMs);
    expect(visibleSourceMs(2), kWaveformVisibleMs * 2);
    expect(visibleSourceMs(8), kWaveformVisibleMs * 2);
    expect(visibleSourceMs(0), kWaveformVisibleMs);
    expect(visibleSourceMs(1, kWaveformMinVisibleMs), kWaveformMinVisibleMs);
  });

  test('zoomedVisibleMs scrolls in on wheel up and clamps to the range', () {
    final zoomedIn = zoomedVisibleMs(
      visibleMs: kWaveformVisibleMs,
      scrollDeltaY: -120,
    );
    expect(zoomedIn, lessThan(kWaveformVisibleMs));
    final zoomedOut = zoomedVisibleMs(
      visibleMs: kWaveformVisibleMs,
      scrollDeltaY: 120,
    );
    expect(zoomedOut, greaterThan(kWaveformVisibleMs));
    expect(
      zoomedVisibleMs(visibleMs: kWaveformMinVisibleMs, scrollDeltaY: -5000),
      kWaveformMinVisibleMs,
    );
    expect(
      zoomedVisibleMs(visibleMs: kWaveformMaxVisibleMs, scrollDeltaY: 5000),
      kWaveformMaxVisibleMs,
    );
    expect(zoomedVisibleMs(visibleMs: 5000, scrollDeltaY: 0), 5000);
  });

  test('l1Range clamps to the track so t=0 maps to the first L1 peak', () {
    final range = l1Range(
      positionMs: 0,
      visibleMs: 24_000,
      durationMs: 180_000,
    );
    expect(range.startMs, 0);
    expect(range.endMs, 36_000);

    final tail = l1Range(
      positionMs: 170_000,
      visibleMs: 24_000,
      durationMs: 180_000,
    );
    expect(tail.startMs, 134_000);
    expect(tail.endMs, 180_000);
  });

  test('l1NeedsRefresh is false at t=0 once the clamped window is loaded', () {
    expect(
      l1NeedsRefresh(
        positionMs: 0,
        detailStartMs: 0,
        detailEndMs: 36_000,
        visibleMs: 24_000,
        durationMs: 180_000,
      ),
      isFalse,
    );
    expect(
      l1NeedsRefresh(
        positionMs: 28_000,
        detailStartMs: 0,
        detailEndMs: 36_000,
        visibleMs: 24_000,
        durationMs: 180_000,
      ),
      isTrue,
    );
    expect(
      l1NeedsRefresh(
        positionMs: 179_000,
        detailStartMs: 144_000,
        detailEndMs: 180_000,
        visibleMs: 24_000,
        durationMs: 180_000,
      ),
      isFalse,
    );
  });

  test('centerScrubMs subtracts pointer delta across the span', () {
    expect(
      centerScrubMs(anchorPosMs: 10_000, deltaX: 50, width: 100, spanMs: 1000),
      9500,
    );
  });

  test('grid edit helpers clamp bpm and shift phase', () {
    expect(nudgeFirstBeatSecs(1, 10), closeTo(1.01, 1e-9));
    expect(stepGridBpm(128, 0.01), 128.01);
    expect(stepGridBpm(20, -1), 20.01);
    expect(stepGridBpm(399, 1), 399.99);
    expect(parseGridBpm('128.5'), 128.5);
    expect(parseGridBpm(''), isNull);
    expect(parseGridBpm('10'), isNull);
    expect(stepGridBpm(128, kGridBpmCoarseStep), 129.0);
  });

  test(
    'beatGridXs is origin-relative so marks stay put while the lane scrolls',
    () {
      final xs = beatGridXs(
        bpm: 120,
        firstBeatSecs: 0,
        originMs: 0,
        spanMs: 2000,
        width: 200,
      );
      expect(xs.first.x, closeTo(0, 1e-6));
      expect(xs[1].x, closeTo(50, 1e-6));
      expect(xs.where((m) => m.isBar), isNotEmpty);
      expect(xs.where((m) => !m.isBar), isNotEmpty);
    },
  );

  test('late engine poll under 60ms does not yank the playhead', () {
    expect(correctPlayheadDrift(displayMs: 1040, estimateMs: 1000), 1040);
  });

  test('large playhead drift is pulled 25 percent', () {
    expect(correctPlayheadDrift(displayMs: 1000, estimateMs: 1080), 1020);
  });

  test('playhead snaps when paused or on a seek', () {
    expect(
      playheadShouldSnap(displayMs: 1000, engineMs: 1010, playing: false),
      isTrue,
    );
    expect(
      playheadShouldSnap(displayMs: 1000, engineMs: 1010, playing: true),
      isFalse,
    );
    expect(
      playheadShouldSnap(displayMs: 1000, engineMs: 1200, playing: true),
      isTrue,
    );
  });

  test('vinyl jog touch does not advance the interpolated playhead', () {
    expect(playheadAdvancing(playing: true, jogTouching: false), isTrue);
    expect(playheadAdvancing(playing: true, jogTouching: true), isFalse);
    expect(playheadAdvancing(playing: false, jogTouching: true), isFalse);
  });

  test('snapPx rounds to device pixels', () {
    expect(snapPx(1.4, 1), 1);
    expect(snapPx(1.4, 2), 1.5);
  });

  test('stripTranslateX keeps the playhead at the viewport center', () {
    const width = 1920.0;
    const pxPerMs = 0.08;
    const pos = 10_000.0;
    final dx = stripTranslateX(
      positionMs: pos,
      originMs: 0,
      viewportWidth: width,
      pxPerMs: pxPerMs,
    );
    expect(dx + pos * pxPerMs, closeTo(width / 2, 1e-6));
  });

  test('stripTranslateX is origin relative', () {
    const width = 1920.0;
    const pxPerMs = 0.08;
    final dx = stripTranslateX(
      positionMs: 10_000,
      originMs: 9_000,
      viewportWidth: width,
      pxPerMs: pxPerMs,
    );
    expect(dx, closeTo(width / 2 - 1_000 * pxPerMs, 1e-6));
  });

  test(
    'rebased origin at t=0 puts the visible slice one viewport to the left',
    () {
      const width = 1920.0;
      const visible = 24000.0;
      final dx = playheadDx(
        positionMs: 0,
        originMs: -visible * 1.5,
        width: width,
        pxPerMs: width / visible,
      );
      expect(dx, closeTo(-width, 1e-6));
    },
  );

  test('playheadWallDuration is track length scaled by speed', () {
    expect(
      playheadWallDuration(durationMs: 60_000, speed: 1),
      const Duration(milliseconds: 60_000),
    );
    expect(
      playheadWallDuration(durationMs: 60_000, speed: 2),
      const Duration(milliseconds: 30_000),
    );
    expect(
      playheadWallDuration(durationMs: 60_000, speed: 0.5),
      const Duration(milliseconds: 120_000),
    );
    expect(
      playheadWallDuration(durationMs: 0, speed: 1),
      const Duration(milliseconds: 1),
    );
  });
}
