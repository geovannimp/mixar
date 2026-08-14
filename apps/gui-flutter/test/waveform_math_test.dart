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
  });

  test('visibleSourceMs scales with speed and clamps', () {
    expect(visibleSourceMs(1), kWaveformVisibleMs);
    expect(visibleSourceMs(2), kWaveformVisibleMs * 2);
    expect(visibleSourceMs(8), kWaveformVisibleMs * 2);
    expect(visibleSourceMs(0), kWaveformVisibleMs);
  });

  test('overviewWindowRect is the visible span as 0..1', () {
    final rect = overviewWindowRect(
      positionMs: 12_000,
      durationMs: 48_000,
      visibleMs: 24_000,
    );
    expect(rect.left, closeTo(0, 1e-6));
    expect(rect.right, closeTo(0.5, 1e-6));
  });

  test('l1Range clamps to the track so t=0 maps to the first L1 peak', () {
    final range = l1Range(
      positionMs: 0,
      visibleMs: 24_000,
      durationMs: 180_000,
    );
    expect(range.startMs, 0);
    expect(range.endMs, 36_000);

    const overview = [
      SpectralPeak(low: 1, mid: 0, high: 0),
      SpectralPeak(low: 1, mid: 0, high: 0),
    ];
    final detail = DetailWindow(
      peaks: const [
        SpectralPeak(low: 0, mid: 0, high: 1),
        SpectralPeak(low: 0, mid: 1, high: 0),
      ],
      startMs: range.startMs,
      endMs: range.endMs,
    );
    expect(peakAtTime(overview, detail, 180_000, 0).high, closeTo(1, 1e-6));

    final tail = l1Range(
      positionMs: 170_000,
      visibleMs: 24_000,
      durationMs: 180_000,
    );
    expect(tail.startMs, 134_000);
    expect(tail.endMs, 180_000);
  });

  test('centerScrubMs subtracts pointer delta across the span', () {
    expect(
      centerScrubMs(anchorPosMs: 10_000, deltaX: 50, width: 100, spanMs: 1000),
      9500,
    );
  });

  test('beatGridXs marks bars every 4 beats from bpm + phase', () {
    final xs = beatGridXs(
      bpm: 120,
      firstBeatSecs: 0,
      startMs: 0,
      endMs: 2000,
      positionMs: 1000,
      width: 200,
      visibleMs: 2000,
    );
    expect(xs.where((m) => m.isBar), isNotEmpty);
    expect(xs.where((m) => !m.isBar), isNotEmpty);
  });
}
