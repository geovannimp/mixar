import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/waveform_overlay_geometry.dart';
import 'package:gui_flutter/core/utils/waveform_overlay_pictures.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart'
    show LoopRegion, SavedLoop;
import 'package:gui_flutter/widgets/mixer/pads/hot_cue_pads.dart';

void main() {
  test('paintBeatGrid paints arrows on both lane edges', () async {
    const size = Size(120, 40);
    final recorder = PictureRecorder();
    paintBeatGrid(
      Canvas(recorder),
      marks: const [],
      phaseArrows: const [60],
      size: size,
    );
    final picture = recorder.endRecording();
    final image = await picture.toImage(120, 40);
    final bytes = await image.toByteData();
    int alphaAt(int x, int y) => bytes!.getUint8((y * 120 + x) * 4 + 3);

    // Top and bottom triangles converge on the arrow column.
    expect(alphaAt(60, 1), greaterThan(0));
    expect(alphaAt(60, 39), greaterThan(0));
    // The bar line runs between them.
    expect(alphaAt(60, 20), greaterThan(0));
    // Neighbouring columns stay clean.
    expect(alphaAt(20, 1), 0);
    expect(alphaAt(20, 39), 0);
    picture.dispose();
    image.dispose();
  });

  test('msToX maps mid-window to half width', () {
    expect(
      msToX(ms: 30_000, originMs: 0, spanMs: 60_000, width: 200),
      closeTo(100, 1e-6),
    );
  });

  test('msToX is origin relative', () {
    expect(
      msToX(ms: 15_000, originMs: 10_000, spanMs: 10_000, width: 200),
      closeTo(100, 1e-6),
    );
  });

  test('msToX clamps and rejects invalid span', () {
    expect(msToX(ms: -10, originMs: 0, spanMs: 1000, width: 100), 0);
    expect(msToX(ms: 2000, originMs: 0, spanMs: 1000, width: 100), 100);
    expect(msToX(ms: 500, originMs: 0, spanMs: 0, width: 100), 0);
  });

  test('loopRegionRect is null when out <= in or span invalid', () {
    expect(
      loopRegionRect(
        inMs: 10,
        outMs: 10,
        originMs: 0,
        spanMs: 1000,
        width: 100,
        height: 40,
      ),
      isNull,
    );
    expect(
      loopRegionRect(
        inMs: 0,
        outMs: 100,
        originMs: 0,
        spanMs: 0,
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
      originMs: 0,
      spanMs: 100_000,
      width: 200,
      height: 40,
    )!;
    expect(r.left, closeTo(50, 1e-6));
    expect(r.width, closeTo(50, 1e-6));
    expect(r.top, 0);
    expect(r.bottom, 40);
  });

  test('loopRegionRect follows a zoomed window', () {
    final r = loopRegionRect(
      inMs: 30_000,
      outMs: 40_000,
      originMs: 20_000,
      spanMs: 40_000,
      width: 200,
      height: 40,
    )!;
    expect(r.left, closeTo(50, 1e-6));
    expect(r.width, closeTo(50, 1e-6));
  });

  test('recordActiveLoopPicture is null when inactive or missing', () {
    expect(
      recordActiveLoopPicture(
        loop: null,
        originMs: 0,
        spanMs: 10_000,
        size: const Size(100, 40),
      ),
      isNull,
    );
    expect(
      recordActiveLoopPicture(
        loop: const LoopRegion(inMs: 0, outMs: 1000, active: false),
        originMs: 0,
        spanMs: 10_000,
        size: const Size(100, 40),
      ),
      isNull,
    );
  });

  test('recordActiveLoopPicture returns a picture when active', () {
    final picture = recordActiveLoopPicture(
      loop: const LoopRegion(inMs: 1000, outMs: 3000, active: true),
      originMs: 0,
      spanMs: 10_000,
      size: const Size(100, 40),
    );
    expect(picture, isNotNull);
    picture!.dispose();
  });

  test('recordPendingLoopInPicture is null without position', () {
    expect(
      recordPendingLoopInPicture(
        pendingInMs: null,
        originMs: 0,
        spanMs: 10_000,
        size: const Size(100, 40),
      ),
      isNull,
    );
  });

  test('recordPendingLoopInPicture returns a picture when pending', () {
    final picture = recordPendingLoopInPicture(
      pendingInMs: 2500,
      originMs: 0,
      spanMs: 10_000,
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
      originMs: 0,
      spanMs: 10_000,
      size: const Size(200, 40),
    );
    final loops = recordLoopPicture(
      loops: const [SavedLoop(slot: 0, inMs: 2000, outMs: 4000)],
      originMs: 0,
      spanMs: 10_000,
      size: const Size(200, 40),
    );
    cues.dispose();
    loops.dispose();
  });
}
