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
  });

  test('strip display pxPerMs zooms out when tempo increases', () {
    const base = 1 / 13;
    expect(stripDisplayPxPerMs(pxPerMs: base, speed: 1), closeTo(base, 1e-12));
    expect(
      stripDisplayPxPerMs(pxPerMs: base, speed: 2),
      closeTo(base / 2, 1e-12),
    );
    expect(
      stripDisplayPxPerMs(pxPerMs: base, speed: 0.5),
      closeTo(base / 0.5, 1e-12),
    );
    expect(
      stripDisplayPxPerMs(pxPerMs: base, speed: 8),
      closeTo(base / 2, 1e-12),
    );
  });

  test('cropVisibleMs widens with faster tempo', () {
    final atOne = cropVisibleMs(durationMs: 180_000, viewportWidth: 800);
    final atTwo = cropVisibleMs(
      durationMs: 180_000,
      viewportWidth: 800,
      speed: 2,
    );
    expect(atTwo, closeTo(atOne * 2, 2));
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

  test('phaseMarkXs anchors on the first downbeat, one arrow per bar', () {
    // 120 bpm, 4/4: one bar every 2 s, so arrows run 2 s apart from the first
    // downbeat (1 s here), never before it.
    final xs = phaseMarkXs(
      bpm: 120,
      downbeatsSecs: [1, 3, 5, 7, 9, 11, 13, 15, 17, 19],
      firstBeatSecs: 1,
      originMs: 0,
      spanMs: 40_000,
      width: 400,
    );
    expect(xs.take(4), [
      closeTo(10, 1e-6), // 1 s
      closeTo(30, 1e-6), // 3 s
      closeTo(50, 1e-6), // 5 s
      closeTo(70, 1e-6), // 7 s
    ]);
    // Extrapolated past the stored downbeats (they stop at 19 s).
    expect(xs, hasLength(20));
    expect(xs.last, closeTo(390, 1e-6)); // 39 s
  });

  test('phaseMarkXs keeps arrowing past the analyzed span', () {
    // Stored grids stop at the analysis window (30 s here) on a 200 s track:
    // arrows still run the whole buffer at the detected 4/4 bar period.
    final xs = phaseMarkXs(
      bpm: 96,
      downbeatsSecs: [for (var i = 0; i < 12; i++) 0.6888549 + 2.5 * i],
      firstBeatSecs: 0.06385487,
      originMs: 0,
      spanMs: 200_000,
      width: 200,
    );
    // 1 bar at 96 bpm = 2.5 s = 2.5 px here.
    expect(xs, hasLength(80));
    expect(xs.first, closeTo(0.69, 0.01));
    expect(xs.last, closeTo(198.19, 0.01));
    for (var i = 1; i < xs.length; i++) {
      expect(xs[i] - xs[i - 1], closeTo(2.5, 1e-6));
    }
  });

  test('phaseMarkXs follows the meter in the downbeats, not 4/4', () {
    // 3/4 at 60 bpm: downbeats every 3 s, so arrows land every 3 s.
    final xs = phaseMarkXs(
      bpm: 60,
      downbeatsSecs: [0, 3, 6, 9, 12, 15, 18, 21, 24],
      firstBeatSecs: 0,
      originMs: 0,
      spanMs: 30_000,
      width: 300,
    );
    expect(xs.take(3), [
      closeTo(0, 1e-6),
      closeTo(30, 1e-6),
      closeTo(60, 1e-6),
    ]);
    // 0..30 s inclusive, one bar (3 s) each.
    expect(xs, hasLength(11));
    expect(beatsPerBarFrom(downbeats: [0, 3, 6], bpm: 60), 3);
  });

  test('phaseMarkXs falls back to an even 4/4 grid without downbeats', () {
    // 120 bpm, no downbeats: first bar at the first beat, one arrow per bar.
    final xs = phaseMarkXs(
      bpm: 120,
      downbeatsSecs: const [],
      firstBeatSecs: 0.5,
      originMs: 0,
      spanMs: 20_000,
      width: 200,
    );
    expect(xs.take(3), [
      closeTo(5, 1e-6),
      closeTo(25, 1e-6),
      closeTo(45, 1e-6),
    ]);
    expect(xs, hasLength(10));
  });

  test('barPeriodMs falls back to downbeat spacing without a BPM', () {
    // No BPM: the meter cannot be derived, so the stored spacing is the bar.
    expect(barPeriodMs(downbeats: [0, 3, 6]), closeTo(3000, 1e-6));
    expect(barPeriodMs(downbeats: const []), isNull);
    expect(barPeriodMs(downbeats: [0, 2.5, 5], bpm: 96), closeTo(2500, 1e-6));
    expect(beatsPerBarFrom(downbeats: [0, 3, 6], bpm: 60), 3);
    expect(beatsPerBarFrom(downbeats: [0, 2, 4], bpm: 120), 4);
    expect(beatsPerBarFrom(downbeats: [0], bpm: 120), 4);
  });

  test('phaseMarkXs yields nothing without a grid', () {
    expect(
      phaseMarkXs(
        bpm: null,
        downbeatsSecs: const [],
        firstBeatSecs: 0,
        originMs: 0,
        spanMs: 20_000,
        width: 200,
      ),
      isEmpty,
    );
  });

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

  test('strip width depends on duration, not viewport', () {
    expect(stripWidthPx(180_000), (180_000 / 13).ceil());
    expect(stripWidthPx(60_000), (60_000 / 13).ceil());
    expect(stripWidthPx(180_000), inInclusiveRange(2048, 16384));
  });

  test('strip crop shows more time in a wider viewport', () {
    final narrow = cropVisibleMs(durationMs: 180_000, viewportWidth: 800);
    final wide = cropVisibleMs(durationMs: 180_000, viewportWidth: 1600);
    expect(wide, closeTo(narrow * 2, 2));
  });

  test('stripTranslateX keeps the playhead at the viewport center', () {
    const width = 1920.0;
    const pxPerMs = 0.08;
    const pos = 10_000.0;
    final dx = stripTranslateX(
      positionMs: pos,
      viewportWidth: width,
      pxPerMs: pxPerMs,
    );
    expect(dx + pos * pxPerMs, closeTo(width / 2, 1e-6));
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
