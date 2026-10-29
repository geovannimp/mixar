import 'dart:ui';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_strip.dart';
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

Picture _emptyPicture() {
  final recorder = PictureRecorder();
  Canvas(recorder);
  return recorder.endRecording();
}

WaveformStrip _strip() => WaveformStrip(
  durationMs: _durationMs,
  widthPx: _stripWidthPx,
  heightPx: _stripHeightPx,
  l0: _emptyPicture(),
);

class _SeededEngineUi extends EngineUi {
  new({this.playing = false, this.jogTouching = false});

  final bool playing;
  final bool jogTouching;

  @override
  EngineUiSnapshot build() {
    var snap = applyEngineEvent(
      EngineUiSnapshot.empty,
      deckUpdatedEvent(
        deck: 0,
        update: deckUpdated(
          deck: 0,
          trackPath: '/t.flac',
          trackId: 't1',
          durationMs: _durationMs,
          speed: 0.5,
          tempoRange: 0.08,
        ),
      ),
    );
    if (playing) {
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(deck: 0, durationMs: _durationMs, playing: true),
        ),
      );
    }
    if (jogTouching) {
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(
            deck: 0,
            durationMs: _durationMs,
            jogTouching: true,
          ),
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
  final pxPerMs = stripDisplayPxPerMs(
    pxPerMs: _stripWidthPx / _durationMs,
    speed: speed,
  );
  return snapPx(
    stripTranslateX(
      positionMs: positionMs,
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
      (w) => w is Positioned && w.width == _stripWidthPx.toDouble(),
    ),
  );
  expect(positioned, findsOneWidget);
  return tester.widget<Positioned>(positioned);
}

Future<ProviderContainer> _pumpLane(
  WidgetTester tester, {
  required bool playing,
  required WaveformStrip strip,
  bool jogTouching = false,
  int positionMs = 0,
  FakeEngineTransport? engine,
  int Function()? nowMs,
}) async {
  debugOverrideDesktopWindow = false;
  addTearDown(() => debugOverrideDesktopWindow = null);

  _FixedStrip.strip = strip;
  addTearDown(() {
    strip.dispose();
    _FixedStrip.strip = null;
  });

  final theme = MixarThemeData.dark();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        if (engine != null)
          engineTransportProvider.overrideWith((ref) async => engine),
        engineUiProvider.overrideWith(
          () => _SeededEngineUi(playing: playing, jogTouching: jogTouching),
        ),
        deckPlayheadsProvider.overrideWith(
          () => _FixedPlayheads({0: positionMs}),
        ),
        waveformStripProvider.overrideWith2(_FixedStrip.new),
        beatGridFetchProvider.overrideWith((ref, id) async => null),
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
      .apply(deckPositionEvent(deck: 0, positionMs: ms));
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
    final container = await _pumpLane(tester, playing: false, strip: _strip());

    expect(_stripPositioned(tester).left, closeTo(_expectedLeft(0), 0.5));

    _applyPosition(container, 25000);
    await tester.pump();

    expect(_stripPositioned(tester).left, closeTo(_expectedLeft(25000), 0.5));
  });

  testWidgets('vinyl jogTouching position ticks move strip translate', (
    tester,
  ) async {
    final container = await _pumpLane(
      tester,
      playing: true,
      jogTouching: true,
      positionMs: 10000,
      strip: _strip(),
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
      strip: _strip(),
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
      strip: _strip(),
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
    final container = await _pumpLane(tester, playing: false, strip: _strip());

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
      strip: _strip(),
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
        strip: _strip(),
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
    },
  );
}
