import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/waveform/beat_grid.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';

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

  test('spectralRgb matches Tauri band mix', () {
    expect(spectralRgb(1, 0, 0), const Color.fromARGB(255, 255, 72, 48));
    expect(spectralRgb(0, 1, 0), const Color.fromARGB(255, 118, 228, 88));
    expect(spectralRgb(0, 0, 1), const Color.fromARGB(255, 72, 188, 255));
  });

  test('filteredBars draws low then mid then high from the same center', () {
    const peak = SpectralPeak(low: 1, mid: 0.5, high: 0.25);
    final bars = filteredBars(peak, 100);
    expect(bars, hasLength(3));
    expect(bars[0].height, 100);
    expect(bars[0].color, kLowColor);
    expect(bars[1].height, 50);
    expect(bars[1].color, kMidColor);
    expect(bars[2].height, 25);
    expect(bars[2].color, kHighColor);
  });

  test(
    'waveformBars is one mixed bar in RGB and three stacked bars in Filtered',
    () {
      const peak = SpectralPeak(low: 1, mid: 0.5, high: 0.25);
      expect(
        waveformBars(peak, 100, WaveformDisplayMode.filtered),
        hasLength(3),
      );
      final rgb = waveformBars(peak, 100, WaveformDisplayMode.rgb);
      expect(rgb, hasLength(1));
      expect(rgb.single.height, 100);
    },
  );

  test('barFill is opaque and leans toward the background as amp drops', () {
    final full = barFill(const Color.fromARGB(255, 255, 72, 48), 1);
    expect(full.a, 1);
    expect(full.r, closeTo(1, 1e-3));
    final dim = barFill(const Color.fromARGB(255, 255, 72, 48), 0);
    expect(dim.a, 1);
    expect(dim.r, lessThan(full.r));
  });

  test('peakAtTime prefers L1 inside the detail window', () {
    const overview = [
      SpectralPeak(low: 1, mid: 0, high: 0),
      SpectralPeak(low: 1, mid: 0, high: 0),
    ];
    const detail = DetailWindow(
      peaks: [
        SpectralPeak(low: 0, mid: 1, high: 0),
        SpectralPeak(low: 0, mid: 1, high: 0),
      ],
      startMs: 1000,
      endMs: 2000,
    );
    expect(peakAtTime(overview, detail, 4000, 1500).mid, closeTo(1, 1e-6));
    expect(peakAtTime(overview, detail, 4000, 0).low, closeTo(1, 1e-6));
    expect(
      peakAtTime(overview, detail, 4000, 0, fallbackToOverview: false).low,
      0,
    );
  });

  test('ringChunkPx is floor(window/4) with a floor of 1', () {
    expect(ringChunkPx(1920), 480);
    expect(ringChunkPx(800), 200);
    expect(ringChunkPx(3), 1);
    expect(ringChunkPx(0), 1);
  });

  test('ringRange centers eight chunks and clamps at track ends', () {
    final mid = ringRange(
      positionMs: 90_000,
      visibleMs: 24_000,
      durationMs: 180_000,
    );
    expect(mid.chunkMs, 6_000);
    expect(mid.chunkCount, 8);
    expect(mid.originMs, closeTo(66_000, 1e-6));

    final head = ringRange(
      positionMs: 0,
      visibleMs: 24_000,
      durationMs: 180_000,
    );
    expect(head.originMs, 0);
    expect(head.chunkCount, 8);

    final short = ringRange(
      positionMs: 5_000,
      visibleMs: 24_000,
      durationMs: 20_000,
    );
    expect(short.chunkCount, lessThanOrEqualTo(8));
    expect(short.originMs, 0);
    expect(
      short.originMs + short.chunkMs * short.chunkCount,
      lessThanOrEqualTo(20_000 + 1e-6),
    );
  });

  test('ringNeedsSlide trips after two chunks of drift', () {
    const origin = 66_000.0;
    const chunk = 6_000.0;
    const count = 8;
    // Center at origin + 4*chunk = 90_000
    expect(
      ringNeedsSlide(
        positionMs: 90_000,
        ringOriginMs: origin,
        chunkMs: chunk,
        chunkCount: count,
        durationMs: 180_000,
      ),
      isFalse,
    );
    expect(
      ringNeedsSlide(
        positionMs: 90_000 + 2 * chunk - 1,
        ringOriginMs: origin,
        chunkMs: chunk,
        chunkCount: count,
        durationMs: 180_000,
      ),
      isFalse,
    );
    expect(
      ringNeedsSlide(
        positionMs: 90_000 + 2 * chunk,
        ringOriginMs: origin,
        chunkMs: chunk,
        chunkCount: count,
        durationMs: 180_000,
      ),
      isTrue,
    );
    expect(
      ringNeedsSlide(
        positionMs: 179_000,
        ringOriginMs: 180_000 - 8 * chunk,
        chunkMs: chunk,
        chunkCount: count,
        durationMs: 180_000,
      ),
      isFalse,
    );
  });

  test('ringSlideOriginMs shifts by two chunks and clamps', () {
    expect(
      ringSlideOriginMs(
        ringOriginMs: 66_000,
        chunkMs: 6_000,
        chunkCount: 8,
        positionMs: 102_000,
        durationMs: 180_000,
      ),
      closeTo(78_000, 1e-6),
    );
    expect(
      ringSlideOriginMs(
        ringOriginMs: 0,
        chunkMs: 6_000,
        chunkCount: 8,
        positionMs: 0,
        durationMs: 180_000,
      ),
      0,
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

  test('centerScrubMs subtracts pointer delta across the span', () {
    expect(
      centerScrubMs(anchorPosMs: 10_000, deltaX: 50, width: 100, spanMs: 1000),
      9500,
    );
  });

  test('grid edit helpers clamp bpm and shift phase', () {
    expect(nudgeFirstBeatSecs(1.0, 10), closeTo(1.01, 1e-9));
    expect(stepGridBpm(128.0, 0.01), 128.01);
    expect(stepGridBpm(20.0, -1.0), 20.01);
    expect(stepGridBpm(399.0, 1.0), 399.99);
    expect(parseGridBpm('128.5'), 128.5);
    expect(parseGridBpm(''), isNull);
    expect(parseGridBpm('10'), isNull);
    expect(stepGridBpm(128.0, kGridBpmCoarseStep), 129.0);
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
