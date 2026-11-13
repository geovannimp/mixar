import 'package:flutter/gestures.dart' show PointerScrollEvent;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';
import 'package:gui_flutter/core/utils/waveform_section_painter.dart';
import 'package:gui_flutter/core/utils/waveform_shader.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_strip.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_zoom.dart';
import 'package:gui_flutter/src/rust/api/library.dart'
    show BeatGridData, BeatGridSegmentData;
import 'package:gui_flutter/widgets/mixer/waveform/scrolling_lane.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/bus_events.dart';
import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';

const _durationMs = 100000;
const _viewportWidth = 400.0;
const _viewportHeight = 80.0;
const _stripWidthPx = 1000;
const _stripHeightPx = 64;

Future<WaveformStrip> _strip() async => WaveformStrip(
  originMs: 0,
  spanMs: _durationMs,
  widthPx: _stripWidthPx,
  heightPx: _stripHeightPx,
  texture: await encodeWaveformTexture(const [
    SpectralPeak(low: 0.4, mid: 0.2, high: 0.1),
  ]),
);

class _SeededEngineUi extends EngineUi {
  new({this.playing = false, this.jogTouching = false, this.cuePointMs});

  final bool playing;
  final bool jogTouching;
  final int? cuePointMs;

  @override
  EngineUiSnapshot build() {
    var snap = applyEngineEvent(
      EngineUiSnapshot.empty,
      deckUpdatedEvent(
        update: deckUpdated(
          trackPath: '/t.flac',
          trackId: 't1',
          durationMs: _durationMs,
          tempoRange: 0.08,
          cuePointMs: cuePointMs,
        ),
      ),
    );
    if (playing) {
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          update: deckUpdated(durationMs: _durationMs, playing: true),
        ),
      );
    }
    if (jogTouching) {
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          update: deckUpdated(durationMs: _durationMs, jogTouching: true),
        ),
      );
    }
    return snap;
  }
}

class _FixedPlayheads extends DeckPlayheads {
  new(this.initial);
  final Map<int, int> initial;
  @override
  Map<int, int> build() => Map<int, int>.from(initial);
}

class _FixedStrip extends WaveformStripNotifier {
  new(super.arg);
  static WaveformStrip? strip;
  @override
  WaveformStrip? build() => strip;
}

double _expectedLeft(double positionMs, {double speed = 1.0}) {
  final pxPerMs = _viewportWidth / visibleSourceMs(speed);
  return snapPx(
    stripTranslateX(
      positionMs: positionMs,
      originMs: 0,
      viewportWidth: _viewportWidth,
      pxPerMs: pxPerMs,
    ),
    1,
  );
}

Positioned _stripPositioned(WidgetTester tester) {
  final positioned = find.descendant(
    of: find.byType(ScrollingLane),
    matching: find.byWidgetPredicate(
      (w) => w is Positioned && (w.width ?? 0) > 1,
    ),
  );
  expect(positioned, findsOneWidget);
  return tester.widget<Positioned>(positioned);
}

Future<ProviderContainer> _pumpLane(
  WidgetTester tester, {
  required bool playing,
  required Future<WaveformStrip> Function() strip,
  bool jogTouching = false,
  int? cuePointMs,
  int positionMs = 0,
  FakeEngineTransport? engine,
  BeatGridData? beatGrid,
  int Function()? nowMs,
}) async {
  debugOverrideDesktopWindow = false;
  addTearDown(() => debugOverrideDesktopWindow = null);

  // Encoding the peak texture awaits real engine futures, which only advance
  // outside the widget test's fake clock.
  final built = await tester.runAsync(strip);
  _FixedStrip.strip = built;
  addTearDown(() {
    built?.dispose();
    _FixedStrip.strip = null;
  });

  final theme = MixarThemeData.dark();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        if (engine != null)
          engineTransportProvider.overrideWith((ref) async => engine),
        engineUiProvider.overrideWith(
          () => _SeededEngineUi(
            playing: playing,
            jogTouching: jogTouching,
            cuePointMs: cuePointMs,
          ),
        ),
        deckPlayheadsProvider.overrideWith(
          () => _FixedPlayheads({0: positionMs}),
        ),
        waveformStripProvider.overrideWith2(_FixedStrip.new),
        beatGridFetchProvider.overrideWith((ref, id) async => beatGrid),
      ],
      child: MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: SizedBox(
            width: _viewportWidth,
            height: _viewportHeight,
            child: ScrollingLane(deckId: 0, label: 'A', nowMs: nowMs),
          ),
        ),
      ),
    ),
  );
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 16));
  return ProviderScope.containerOf(tester.element(find.byType(ScrollingLane)));
}

void _applyPosition(ProviderContainer container, int ms) {
  container
      .read(engineUiProvider.notifier)
      .apply(deckPositionEvent(positionMs: ms));
}

typedef _LaneRebuildProbe = ({int Function() count, void Function() stop});

_LaneRebuildProbe _startLaneRebuildProbe() {
  var rebuilds = 0;
  final previous = debugOnRebuildDirtyWidget;
  debugOnRebuildDirtyWidget = (element, builtOnce) {
    if (element.widget is ScrollingLane) {
      rebuilds++;
    }
    previous?.call(element, builtOnce);
  };
  return (
    count: () => rebuilds,
    stop: () => debugOnRebuildDirtyWidget = previous,
  );
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('paused position ticks move strip translate', (tester) async {
    final container = await _pumpLane(tester, playing: false, strip: _strip);

    expect(_stripPositioned(tester).left, closeTo(_expectedLeft(0), 0.5));

    _applyPosition(container, 25000);
    await tester.pump();

    expect(_stripPositioned(tester).left, closeTo(_expectedLeft(25000), 0.5));
  });

  testWidgets('wheel over the lane zooms the shared span in', (tester) async {
    final container = await _pumpLane(tester, playing: false, strip: _strip);
    expect(container.read(waveformZoomProvider), kWaveformVisibleMs);
    final widthBefore = _stripPositioned(tester).width ?? 0;

    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: tester.getCenter(find.byType(ScrollingLane)),
        scrollDelta: const Offset(0, -120),
      ),
    );
    await tester.pump();

    expect(container.read(waveformZoomProvider), lessThan(kWaveformVisibleMs));
    expect(
      _stripPositioned(tester).width ?? 0,
      greaterThan(widthBefore),
      reason: 'zooming in shows fewer milliseconds, so the strip paints wider',
    );
  });

  testWidgets('double tap resets the zoom to the default span', (tester) async {
    final container = await _pumpLane(tester, playing: false, strip: _strip);
    container.read(waveformZoomProvider.notifier).zoomBy(-400);
    await tester.pump();
    expect(container.read(waveformZoomProvider), lessThan(kWaveformVisibleMs));

    final detector = tester.widget<GestureDetector>(
      find
          .descendant(
            of: find.byType(ScrollingLane),
            matching: find.byType(GestureDetector),
          )
          .first,
    );
    detector.onDoubleTap!();
    await tester.pump();

    expect(container.read(waveformZoomProvider), kWaveformVisibleMs);
  });

  testWidgets('vinyl jogTouching position ticks move strip translate', (
    tester,
  ) async {
    final container = await _pumpLane(
      tester,
      playing: true,
      jogTouching: true,
      positionMs: 10000,
      strip: _strip,
    );

    expect(_stripPositioned(tester).left, closeTo(_expectedLeft(10000), 0.5));

    _applyPosition(container, 14000);
    await tester.pump();

    expect(_stripPositioned(tester).left, closeTo(_expectedLeft(14000), 0.5));
  });

  testWidgets('playing large engine jump snaps strip translate', (
    tester,
  ) async {
    final container = await _pumpLane(
      tester,
      playing: true,
      positionMs: 1000,
      strip: _strip,
    );

    _applyPosition(container, 1000 + kWaveformSeekSnapMs.round());
    await tester.pump();

    expect(
      _stripPositioned(tester).left,
      closeTo(_expectedLeft(1000 + kWaveformSeekSnapMs), 0.5),
    );
  });

  testWidgets('advancing position ticks do not rebuild ScrollingLane', (
    tester,
  ) async {
    final container = await _pumpLane(
      tester,
      playing: true,
      positionMs: 5000,
      strip: _strip,
    );

    // Settle any post-pump listen/sync work before measuring.
    await tester.pump();
    final probe = _startLaneRebuildProbe();
    addTearDown(probe.stop);
    for (var i = 1; i <= 8; i++) {
      _applyPosition(container, 5000 + i * 33);
    }
    await tester.pump();

    expect(
      probe.count(),
      0,
      reason:
          'ScrollingLane must not watch deckPositionMsProvider while advancing',
    );
  });

  testWidgets('paused position ticks do not rebuild ScrollingLane', (
    tester,
  ) async {
    final container = await _pumpLane(tester, playing: false, strip: _strip);

    await tester.pump();
    final probe = _startLaneRebuildProbe();
    addTearDown(probe.stop);
    for (var i = 1; i <= 8; i++) {
      _applyPosition(container, i * 500);
    }
    await tester.pump();

    expect(
      probe.count(),
      0,
      reason: 'paused Position ticks should sync via listen, not rebuild',
    );
    expect(_stripPositioned(tester).left, closeTo(_expectedLeft(4000), 0.5));
  });

  testWidgets('vinyl position ticks do not rebuild ScrollingLane', (
    tester,
  ) async {
    final container = await _pumpLane(
      tester,
      playing: true,
      jogTouching: true,
      positionMs: 10000,
      strip: _strip,
    );

    await tester.pump();
    final probe = _startLaneRebuildProbe();
    addTearDown(probe.stop);
    for (var i = 1; i <= 8; i++) {
      _applyPosition(container, 10000 + i * 250);
    }
    await tester.pump();

    expect(
      probe.count(),
      0,
      reason: 'vinyl Position ticks should sync via listen, not rebuild',
    );
    expect(_stripPositioned(tester).left, closeTo(_expectedLeft(12000), 0.5));
  });

  testWidgets(
    'scrub move seeks are throttled to 32ms; pointer up always seeks',
    (tester) async {
      // Injected clock: the throttle window is stepped explicitly, so the
      // assertion does not depend on real elapsed time between pumps.
      var nowMs = 0;
      final engine = FakeEngineTransport();
      await _pumpLane(
        tester,
        playing: false,
        positionMs: 50000,
        strip: _strip,
        engine: engine,
        nowMs: () => nowMs,
      );

      final center = tester.getCenter(find.byType(ScrollingLane));
      final gesture = await tester.startGesture(center);
      await tester.pump();

      // Several moves inside one throttle window → one seek.
      await gesture.moveBy(const Offset(-10, 0));
      await tester.pump();
      await gesture.moveBy(const Offset(-10, 0));
      await tester.pump();
      await gesture.moveBy(const Offset(-10, 0));
      await tester.pump();
      expect(engine.seeks.length, 1);

      // Step past the 32 ms window of the injected clock.
      nowMs += 40;
      await gesture.moveBy(const Offset(-10, 0));
      await tester.pump();
      // Drain fire-and-forget _seek → seekDeck microtasks before asserting.
      await tester.pump();
      expect(engine.seeks.length, 2);

      await gesture.up();
      await tester.pump();
      expect(engine.seeks.length, 3);
      expect(engine.seeks.every((s) => s.$1 == 0), isTrue);

      // The double-tap zoom reset recognizer arms a timer on each tap.
      await tester.pump(const Duration(milliseconds: 400));
    },
  );

  testWidgets('dragging before the start keeps moving the strip', (
    tester,
  ) async {
    final engine = FakeEngineTransport();
    await _pumpLane(tester, playing: false, strip: _strip, engine: engine);

    final restingLeft = _stripPositioned(tester).left!;
    final center = tester.getCenter(find.byType(ScrollingLane));
    final gesture = await tester.startGesture(center);
    await tester.pump();

    // Moving the waveform right walks the playhead backwards, into the
    // silence before the track: the strip must follow instead of freezing at
    // t=0 because the controller cannot go negative.
    await gesture.moveBy(const Offset(40, 0));
    await tester.pump();

    final movedLeft = _stripPositioned(tester).left!;
    expect(movedLeft, greaterThan(restingLeft));
    expect(movedLeft, closeTo(restingLeft + 40, 0.5));
    expect(engine.seeks, isNotEmpty);
    expect(engine.seeks.last.$2, lessThan(0));

    await gesture.up();
    await tester.pump(const Duration(milliseconds: 400));
  });
  testWidgets('tempo bars paint below the cue and loop overlay', (
    tester,
  ) async {
    await _pumpLane(
      tester,
      playing: false,
      strip: _strip,
      beatGrid: const BeatGridData(
        bpm: 120,
        firstBeatMs: 0,
        beatsPerBar: 4,
        beatUnit: 4,
        firstDownbeatMs: 0,
        segmentCount: 1,
        segments: [
          BeatGridSegmentData(
            startTimeMs: 0,
            bpm: 120,
            beatsPerBar: 4,
            beatUnit: 4,
            startBeatIndex: 0,
            downbeatOffsetBeats: 0,
          ),
        ],
      ),
    );
    await tester.pump();

    final grid = find.byWidgetPredicate(
      (w) =>
          w is CustomPaint &&
          w.painter.runtimeType.toString() == '_BeatGridPicturePainter',
    );
    expect(grid, findsOneWidget);

    // Grid and overlays share one strip stack; the overlay painter must be
    // later so cue flags cover the phase arrows.
    final stack = tester.widget<Stack>(
      find.byWidgetPredicate((w) => w is Stack && w.clipBehavior == Clip.none),
    );
    final gridIndex = stack.children.indexWhere((c) => c is Positioned);
    final overlayIndex = stack.children.indexWhere(
      (c) =>
          c is CustomPaint &&
          c.painter.runtimeType.toString() == '_StripOverlayPainter',
    );
    expect(gridIndex, isNonNegative);
    expect(overlayIndex, greaterThan(gridIndex));
  });

  testWidgets('the main cue is flagged on the lane', (tester) async {
    await _pumpLane(tester, playing: false, strip: _strip, cuePointMs: 25000);

    final mainCue = [
      for (final section in _sections(tester))
        if (section.type == WaveformSectionType.mainCue) section,
    ];
    expect(mainCue, hasLength(1));
    expect(mainCue.single.startMs, 25000);
    expect(
      mainCue.single.label,
      isNull,
      reason: 'the main cue carries an icon, not a hot-cue slot number',
    );
  });
}

/// Cue/loop sections the lane hands to its painter.
List<WaveformSection> _sections(WidgetTester tester) {
  for (final paint in tester.widgetList<CustomPaint>(
    find.byType(CustomPaint),
  )) {
    final painter = paint.painter;
    if (painter is WaveformSectionPainter) {
      return painter.sections;
    }
  }
  fail('the lane has no WaveformSectionPainter');
}
