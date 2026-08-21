import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/waveform/overlay_geometry.dart';
import 'package:gui_flutter/mixer/waveform/overlay_pictures.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

void main() {
  test('msToX maps mid-track to half width', () {
    expect(
      msToX(ms: 30_000, durationMs: 60_000, width: 200),
      closeTo(100, 1e-6),
    );
  });

  test('msToX clamps and rejects invalid duration', () {
    expect(msToX(ms: -10, durationMs: 1000, width: 100), 0);
    expect(msToX(ms: 2000, durationMs: 1000, width: 100), 100);
    expect(msToX(ms: 500, durationMs: 0, width: 100), 0);
  });

  test('loopRegionRect is null when out <= in or duration invalid', () {
    expect(
      loopRegionRect(
        inMs: 10,
        outMs: 10,
        durationMs: 1000,
        width: 100,
        height: 40,
      ),
      isNull,
    );
    expect(
      loopRegionRect(
        inMs: 0,
        outMs: 100,
        durationMs: 0,
        width: 100,
        height: 40,
      ),
      isNull,
    );
  });

  test('loopRegionRect spans the loop as a full-height rect', () {
    final r = loopRegionRect(
      inMs: 25_000,
      outMs: 50_000,
      durationMs: 100_000,
      width: 200,
      height: 40,
    )!;
    expect(r.left, closeTo(50, 1e-6));
    expect(r.width, closeTo(50, 1e-6));
    expect(r.top, 0);
    expect(r.bottom, 40);
  });

  test('recordActiveLoopPicture is null when inactive or missing', () {
    expect(
      recordActiveLoopPicture(
        loop: null,
        durationMs: 10_000,
        size: const Size(100, 40),
      ),
      isNull,
    );
    expect(
      recordActiveLoopPicture(
        loop: const ActiveLoopInfo(inMs: 0, outMs: 1000, active: false),
        durationMs: 10_000,
        size: const Size(100, 40),
      ),
      isNull,
    );
  });

  test('recordActiveLoopPicture returns a picture when active', () {
    final picture = recordActiveLoopPicture(
      loop: const ActiveLoopInfo(inMs: 1000, outMs: 3000, active: true),
      durationMs: 10_000,
      size: const Size(100, 40),
    );
    expect(picture, isNotNull);
    picture!.dispose();
  });

  test('recordCuePicture and recordLoopPicture record without throw', () {
    final cues = recordCuePicture(
      cues: const [
        DeckHotCue(slot: 0, positionMs: 1000),
        DeckHotCue(slot: 3, positionMs: 5000),
      ],
      durationMs: 10_000,
      size: const Size(200, 40),
    );
    final loops = recordLoopPicture(
      loops: const [
        SavedLoopInfo(slot: 0, inMs: 2000, outMs: 4000),
      ],
      durationMs: 10_000,
      size: const Size(200, 40),
    );
    cues.dispose();
    loops.dispose();
  });
}
