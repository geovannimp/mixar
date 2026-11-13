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
import 'package:gui_flutter/widgets/mixer/waveform/loop_band_delete.dart';
import 'package:gui_flutter/widgets/mixer/waveform/scrolling_lane.dart';
import 'package:gui_flutter/widgets/mixer/waveform/waveform_frame_clock.dart';
import 'package:gui_flutter/widgets/mixer/waveform/waveform_scroll_viewport.dart';
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
  new({
    this.playing = false,
    this.jogTouching = false,
    this.cuePointMs,
    this.playbackRatio = 1.0,
  });

  final bool playing;
  final bool jogTouching;
  final int? cuePointMs;

  /// Actual playback ratio, tempo-sync override included.
  final double playbackRatio;

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
          playbackRatio: playbackRatio,
          cuePointMs: cuePointMs,
        ),
      ),
    );
    if (playing) {
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          update: deckUpdated(
            durationMs: _durationMs,
            playing: true,
            playbackRatio: playbackRatio,
          ),
        ),
      );
    }
    if (jogTouching) {
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          update: deckUpdated(
            durationMs: _durationMs,
            jogTouching: true,
            playbackRatio: playbackRatio,
          ),
        ),
      );
    }
    return snap;
  }
}

/// Two loaded decks, each published with its own actual playback ratio.
class _TwoDeckEngineUi extends EngineUi {
  new({required this.ratios, this.playing = true});

  final Map<int, double> ratios;
  final bool playing;

  @override
  EngineUiSnapshot build() {
    var snap = EngineUiSnapshot.empty;
    for (final entry in ratios.entries) {
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          deck: entry.key,
          update: deckUpdated(
            deck: entry.key,
            trackPath: '/t${entry.key}.flac',
            trackId: 't${entry.key}',
            durationMs: _durationMs,
            tempoRange: 0.08,
            playing: playing,
            playbackRatio: entry.value,
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

double _expectedLeft(
  double positionMs, {
  double speed = 1.0,
  int visibleMs = kWaveformVisibleMs,
}) {
  final pxPerMs = _viewportWidth / visibleSourceMs(speed, visibleMs);
  // No device-pixel snapping: the strip moves continuously so a slow scroll
  // advances by an equal amount every frame.
  return stripTranslateX(
    positionMs: positionMs,
    originMs: 0,
    viewportWidth: _viewportWidth,
    pxPerMs: pxPerMs,
  );
}

/// Painted strip rect, in lane-local coordinates.
///
/// [WidgetTester.getRect] walks `RenderObject.applyPaintTransform`, so this is
/// the strip's painted position: the lane moves it only by painting, and its
/// layout offset stays at zero.
Rect _stripRect(WidgetTester tester, {int deckId = 0}) {
  final lane = find.byWidgetPredicate(
    (widget) => widget is ScrollingLane && widget.deckId == deckId,
  );
  expect(lane, findsOneWidget);
  final viewport = find.descendant(
    of: lane,
    matching: find.byType(WaveformScrollViewport),
  );
  final region = find.descendant(
    of: viewport,
    matching: find.byType(LoopBandDeleteRegion),
  );
  expect(region, findsOneWidget);
  final origin = tester.getTopLeft(viewport);
  return tester.getRect(region).translate(-origin.dx, -origin.dy);
}

/// Source position a painted strip rect represents, in milliseconds.
///
/// Inverse of [stripTranslateX] for a strip whose origin is 0: the playhead
/// sits at the viewport centre, so the strip's left edge encodes the position
/// at the selected zoom and actual playback ratio.
double _sourceMsAt(
  Rect rect, {
  required double speed,
  int visibleMs = kWaveformVisibleMs,
}) {
  final pxPerMs = _viewportWidth / visibleSourceMs(speed, visibleMs);
  return (_viewportWidth / 2 - rect.left) / pxPerMs;
}

Future<ProviderContainer> _pumpLane(
  WidgetTester tester, {
  required bool playing,
  required Future<WaveformStrip> Function() strip,
  bool jogTouching = false,
  int? cuePointMs,
  int positionMs = 0,
  double playbackRatio = 1.0,
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
            playbackRatio: playbackRatio,
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

void _applyPosition(ProviderContainer container, int ms, {int deck = 0}) {
  container
      .read(engineUiProvider.notifier)
      .apply(deckPositionEvent(deck: deck, positionMs: ms));
}

/// Applies one `DeckUpdated` for [deck], carrying the current value of every
/// field the caller leaves null: a deck update replaces all of them outright.
void _applyDeckUpdate(
  ProviderContainer container, {
  int deck = 0,
  bool? playing,
  bool? jogTouching,
  double? playbackRatio,
  int? durationMs,
}) {
  final state = container.read(engineUiProvider);
  container
      .read(engineUiProvider.notifier)
      .apply(
        deckUpdatedEvent(
          deck: deck,
          update: deckUpdated(
            deck: deck,
            trackPath: '/t.flac',
            trackId: 't1',
            durationMs: durationMs ?? _durationMs,
            tempoRange: 0.08,
            playing: playing ?? state.isPlaying(deck),
            jogTouching: jogTouching ?? state.jogTouchingFor(deck),
            playbackRatio: playbackRatio ?? state.playbackRatioFor(deck),
          ),
        ),
      );
}

/// Zooms all the way in so a millisecond-scale discontinuity spans pixels.
int _zoomIn(ProviderContainer container) {
  container.read(waveformZoomProvider.notifier).zoomBy(-10000);
  return container.read(waveformZoomProvider);
}

/// Makes [positionMs] authoritative, then starts the lane's clock.
Future<double> _startClock(
  WidgetTester tester,
  ProviderContainer container, {
  required double speed,
  required int visibleMs,
  int positionMs = 10000,
}) async {
  _applyPosition(container, positionMs);
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 16));
  return _sourceMsAt(_stripRect(tester), speed: speed, visibleMs: visibleMs);
}

/// Two lanes in one scope, each deck playing at its own actual ratio.
Future<ProviderContainer> _pumpTwoLanes(
  WidgetTester tester, {
  required Map<int, double> ratios,
  int positionMs = 0,
  bool playing = true,
}) async {
  debugOverrideDesktopWindow = false;
  addTearDown(() => debugOverrideDesktopWindow = null);

  final built = await tester.runAsync(_strip);
  _FixedStrip.strip = built;
  addTearDown(() {
    built?.dispose();
    _FixedStrip.strip = null;
  });

  final theme = MixarThemeData.dark();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        engineUiProvider.overrideWith(
          () => _TwoDeckEngineUi(ratios: ratios, playing: playing),
        ),
        deckPlayheadsProvider.overrideWith(
          () => _FixedPlayheads({
            for (final deck in ratios.keys) deck: positionMs,
          }),
        ),
        waveformStripProvider.overrideWith2(_FixedStrip.new),
        beatGridFetchProvider.overrideWith((ref, id) async => null),
      ],
      child: MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Column(
            children: [
              for (final deck in ratios.keys)
                SizedBox(
                  width: _viewportWidth,
                  height: _viewportHeight,
                  child: ScrollingLane(
                    deckId: deck,
                    label: deck == 0 ? 'A' : 'B',
                  ),
                ),
            ],
          ),
        ),
      ),
    ),
  );
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 16));
  return ProviderScope.containerOf(
    tester.element(find.byType(ScrollingLane).first),
  );
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

    expect(_stripRect(tester).left, closeTo(_expectedLeft(0), 0.5));

    _applyPosition(container, 25000);
    await tester.pump();

    expect(_stripRect(tester).left, closeTo(_expectedLeft(25000), 0.5));
  });

  testWidgets('wheel over the lane zooms the shared span in', (tester) async {
    final container = await _pumpLane(tester, playing: false, strip: _strip);
    expect(container.read(waveformZoomProvider), kWaveformVisibleMs);
    final widthBefore = _stripRect(tester).width;

    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: tester.getCenter(find.byType(ScrollingLane)),
        scrollDelta: const Offset(0, -120),
      ),
    );
    await tester.pump();

    expect(container.read(waveformZoomProvider), lessThan(kWaveformVisibleMs));
    expect(
      _stripRect(tester).width,
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

    expect(_stripRect(tester).left, closeTo(_expectedLeft(10000), 0.5));

    _applyPosition(container, 14000);
    await tester.pump();

    expect(_stripRect(tester).left, closeTo(_expectedLeft(14000), 0.5));
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
      _stripRect(tester).left,
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
    expect(_stripRect(tester).left, closeTo(_expectedLeft(4000), 0.5));
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
    expect(_stripRect(tester).left, closeTo(_expectedLeft(12000), 0.5));
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

    final restingLeft = _stripRect(tester).left;
    final center = tester.getCenter(find.byType(ScrollingLane));
    final gesture = await tester.startGesture(center);
    await tester.pump();

    // Moving the waveform right walks the playhead backwards, into the
    // silence before the track: the strip must follow instead of freezing at
    // t=0 because the controller cannot go negative.
    await gesture.moveBy(const Offset(40, 0));
    await tester.pump();

    final movedLeft = _stripRect(tester).left;
    expect(movedLeft, greaterThan(restingLeft));
    expect(movedLeft, closeTo(restingLeft + 40, 0.5));
    expect(engine.seeks, isNotEmpty);
    expect(engine.seeks.last.$2, lessThan(0));

    await gesture.up();
    await tester.pump(const Duration(milliseconds: 400));
  });
  // Drift is a slow accumulation: the engine clock and the lane's
  // interpolation clock run at slightly different rates, so the residual creeps
  // a couple of ms per poll. A seek moves the audio in one step instead.

  testWidgets('the lane advances by an equal amount every frame', (
    tester,
  ) async {
    // Zoomed out, a 16 ms frame is well under a pixel: exactly the regime where
    // rounding the translate to whole device pixels turned a steady scroll into
    // a 1,1,2,1,1,2 judder (and put two lanes on different phases).
    final container = await _pumpLane(tester, playing: true, strip: _strip);
    await tester.pump();
    await _startClock(
      tester,
      container,
      speed: 1,
      visibleMs: kWaveformVisibleMs,
    );
    final msPerPx = visibleSourceMs(1) / _viewportWidth;

    var previous = _stripRect(tester).left;
    for (var frame = 0; frame < 12; frame++) {
      await tester.pump(const Duration(milliseconds: 16));
      final left = _stripRect(tester).left;
      expect(
        (previous - left) * msPerPx,
        closeTo(16, 0.01),
        reason:
            'every frame must advance the strip by the same amount '
            '(frame $frame)',
      );
      previous = left;
    }
  });

  testWidgets('a correction never runs a slow deck backwards', (tester) async {
    // A persistent *negative* residual on a slow deck is the case an escalating
    // correction overshoots: closing 80 ms inside 120 ms would move the strip
    // backwards faster than a 0.5x deck moves it forwards. The plan's 25% pull
    // is rate-bounded instead, so every frame still advances.
    const ratio = 0.5;
    final container = await _pumpLane(
      tester,
      playing: true,
      playbackRatio: ratio,
      strip: _strip,
    );
    await tester.pump();
    final display = await _startClock(
      tester,
      container,
      speed: ratio,
      visibleMs: kWaveformVisibleMs,
    );
    final msPerPx = visibleSourceMs(ratio) / _viewportWidth;

    // The engine stays a fixed 120 ms *behind* the advancing display, and each
    // poll carries a new sample: deckPositionMsProvider stores an int and drops
    // unchanged values, so a constant engine value would only reach the lane
    // once and never exercise repeated corrections.
    const pollMs = 32;
    var engineMs = display - 120;
    await tester.pump();

    var previous = _stripRect(tester).left;
    for (var poll = 0; poll < 20; poll++) {
      engineMs += pollMs * ratio;
      _applyPosition(container, engineMs.round());
      await tester.pump();
      expect(
        _stripRect(tester).left,
        closeTo(previous, 1.0),
        reason: 'poll $poll must not move the strip on its own',
      );
      await tester.pump(const Duration(milliseconds: pollMs));

      final left = _stripRect(tester).left;
      final advanceMs = (previous - left) * msPerPx;
      previous = left;
      // One 32 ms frame advances 32 * 0.5 = 16 ms; the pull may remove at most
      // 0.25 * 0.5 * 32 = 4 ms of that, and never more than all of it.
      expect(
        advanceMs,
        inInclusiveRange(16 - 4 - 0.2, 16 + 0.2),
        reason: 'poll $poll must advance, never run backwards',
      );
    }
  });

  for (final ratio in const [1.0, 1.1857]) {
    testWidgets('gradual drift glides without a step (ratio $ratio)', (
      tester,
    ) async {
      final container = await _pumpLane(
        tester,
        playing: true,
        playbackRatio: ratio,
        strip: _strip,
      );
      final visibleMs = _zoomIn(container);
      expect(visibleMs, kWaveformMinVisibleMs);
      await tester.pump();

      final display = await _startClock(
        tester,
        container,
        speed: ratio,
        visibleMs: visibleMs,
      );
      final msPerPx = visibleSourceMs(ratio, visibleMs) / _viewportWidth;

      // The engine clock runs ~0.6% fast: 2 ms of extra error per poll.
      const pollMs = 32;
      const driftPerPollMs = 2.0;
      var engineMs = display;
      var previous = _stripRect(tester).left;
      for (var poll = 1; poll <= 40; poll++) {
        engineMs += pollMs * ratio + driftPerPollMs;
        _applyPosition(container, engineMs.round());
        final before = _stripRect(tester).left;
        await tester.pump();
        expect(
          _stripRect(tester).left,
          closeTo(before, 1.0),
          reason: 'a poll must never move the strip on its own (poll $poll)',
        );
        // The poll lands before this frame advances, so this is the error the
        // lane actually has to work with.
        expect(
          (engineMs -
                  _sourceMsAt(
                    _stripRect(tester),
                    speed: ratio,
                    visibleMs: visibleMs,
                  ))
              .abs(),
          lessThan(kWaveformDriftCorrectMs + 25),
          reason: 'the drift must stay inside the policy band (poll $poll)',
        );
        await tester.pump(const Duration(milliseconds: pollMs));
        final left = _stripRect(tester).left;
        expect(
          (previous - left) * msPerPx,
          // Clock advance plus a bounded catch-up: never a jump.
          inInclusiveRange(pollMs * ratio - 12, pollMs * ratio + 25),
          reason: 'the glide follows the clock instead of jumping (poll $poll)',
        );
        previous = left;
      }

      // Uncapped, 40 polls of 2 ms would leave ~110 ms of error; the glide has
      // to have kept it inside the band the whole way, without any poll moving
      // the strip.
      expect(
        (engineMs +
                pollMs * ratio -
                _sourceMsAt(
                  _stripRect(tester),
                  speed: ratio,
                  visibleMs: visibleMs,
                ))
            .abs(),
        lessThan(kWaveformDriftCorrectMs + 25),
        reason: 'the corrections must track the engine position',
      );
    });
  }

  for (final ratio in const [1.0, 1.1857]) {
    testWidgets('a late poll is smoothed, never snapped (ratio $ratio)', (
      tester,
    ) async {
      // A delayed report moves the estimate by a poll's worth of audio while
      // playback is perfectly continuous: an age swing of 0 -> 50 ms shifts
      // engine - display by ~59 ms at 1.1857. That is not a seek, so the strip
      // must not move on the poll itself.
      final container = await _pumpLane(
        tester,
        playing: true,
        playbackRatio: ratio,
        strip: _strip,
      );
      final visibleMs = _zoomIn(container);
      await tester.pump();

      final display = await _startClock(
        tester,
        container,
        speed: ratio,
        visibleMs: visibleMs,
      );
      // Drift a little, so the next swing lands outside the deadband.
      _applyPosition(container, (display + 40).round());
      await tester.pump();

      final ageSwingMs = 50 * ratio;
      final before = _stripRect(tester);
      _applyPosition(container, (display + 40 + ageSwingMs).round());
      await tester.pump();

      expect(
        _stripRect(tester).left,
        closeTo(before.left, 1.0),
        reason: 'a late report must not jump the strip',
      );

      var previous = _stripRect(tester).left;
      for (var frame = 0; frame < 25; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
        final left = _stripRect(tester).left;
        expect(
          left,
          lessThan(previous),
          reason: 'the swing must resolve forward, frame by frame',
        );
        previous = left;
      }
    });
  }

  for (final ratio in const [1.0, 1.1857]) {
    testWidgets('one 80 ms step is still smoothed (ratio $ratio)', (
      tester,
    ) async {
      final container = await _pumpLane(
        tester,
        playing: true,
        playbackRatio: ratio,
        strip: _strip,
      );
      final visibleMs = _zoomIn(container);
      await tester.pump();

      final display = await _startClock(
        tester,
        container,
        speed: ratio,
        visibleMs: visibleMs,
      );
      final msPerPx = visibleSourceMs(ratio, visibleMs) / _viewportWidth;

      final before = _stripRect(tester);
      _applyPosition(container, (display + 80).round());
      await tester.pump();

      expect(
        _stripRect(tester).left,
        closeTo(before.left, 1.0),
        reason: 'a single step must not move the strip instantly',
      );

      var previous = _stripRect(tester).left;
      for (var frame = 0; frame < 25; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
        final left = _stripRect(tester).left;
        expect(left, lessThan(previous), reason: 'no reversal');
        previous = left;
      }

      // 25% of the error is the documented target for a transient step.
      expect(
        _sourceMsAt(_stripRect(tester), speed: ratio, visibleMs: visibleMs),
        closeTo(display + 20 + 400 * ratio, 2 * msPerPx),
      );
    });
  }

  for (final ratio in const [1.0, 1.1857]) {
    testWidgets('a persistent offset settles without crawling (ratio $ratio)', (
      tester,
    ) async {
      // A seek the audio already made: the estimate stays 90 ms ahead of the
      // picture. The policy is a rate-bounded 25% pull, retargeted on each
      // poll, so it converges over about a second — and never jumps.
      const seekMs = 90.0;
      const pollMs = 32;
      final container = await _pumpLane(
        tester,
        playing: true,
        playbackRatio: ratio,
        strip: _strip,
      );
      final visibleMs = _zoomIn(container);
      await tester.pump();

      final display = await _startClock(
        tester,
        container,
        speed: ratio,
        visibleMs: visibleMs,
      );

      // The audio already made a 90 ms seek and keeps playing from there; the
      // picture has to close that gap without jumping and without crawling.
      var engineMs = display + seekMs;
      for (var poll = 0; poll < 40; poll++) {
        _applyPosition(container, engineMs.round());
        final before = _stripRect(tester).left;
        await tester.pump();
        expect(
          _stripRect(tester).left,
          closeTo(before, 1.0),
          reason: 'a persistent offset must still not jump (poll $poll)',
        );
        await tester.pump(const Duration(milliseconds: pollMs));
        engineMs += pollMs * ratio;
      }

      final residualMs =
          (engineMs -
                  _sourceMsAt(
                    _stripRect(tester),
                    speed: ratio,
                    visibleMs: visibleMs,
                  ))
              .abs();
      expect(
        residualMs,
        // The deadband is the floor: a correction pulls until the residual is
        // back inside kWaveformDriftCorrectMs and then leaves it alone, so the
        // settled error is the deadband itself, not zero.
        lessThan(kWaveformDriftCorrectMs + 5),
        reason: 'the rate-bounded pull must settle at the deadband, no jump',
      );
    });
  }

  for (final ratio in const [1.0, 1.1857]) {
    testWidgets('retargeting a correction does not jump (ratio $ratio)', (
      tester,
    ) async {
      final container = await _pumpLane(
        tester,
        playing: true,
        playbackRatio: ratio,
        strip: _strip,
      );
      final visibleMs = _zoomIn(container);
      await tester.pump();

      final display = await _startClock(
        tester,
        container,
        speed: ratio,
        visibleMs: visibleMs,
      );
      _applyPosition(container, display.round());
      await tester.pump();

      // Build up drift in polls small enough to stay under the seek step, so
      // the lane glides instead of snapping.
      var engine = display;
      for (var poll = 0; poll < 4; poll++) {
        engine += 40;
        _applyPosition(container, engine.round());
        await tester.pump();
      }
      await tester.pump(const Duration(milliseconds: 32));

      final mid = _stripRect(tester);
      final midDisplay = _sourceMsAt(mid, speed: ratio, visibleMs: visibleMs);
      // Retarget while the correction is still in flight. The engine has crept
      // further ahead (the frame advanced), so this step stays inside the drift
      // band and only changes the correction's target, not its position.
      _applyPosition(container, (midDisplay + 130).round());
      await tester.pump();

      expect(
        _stripRect(tester).left,
        closeTo(mid.left, 1.0),
        reason: 'a retarget must not move the strip',
      );
      var previous = _stripRect(tester).left;
      for (var frame = 0; frame < 25; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
        final left = _stripRect(tester).left;
        expect(
          left,
          lessThan(previous),
          reason: 'a retarget must not reverse the strip',
        );
        previous = left;
      }
    });
  }

  testWidgets('two decks hold phase for a minute without a jump', (
    tester,
  ) async {
    const ratios = {0: 1.0, 1: 1.1857};
    final container = await _pumpTwoLanes(tester, ratios: ratios);
    final zoomMs = _zoomIn(container);
    expect(zoomMs, kWaveformMinVisibleMs);
    await tester.pump();

    for (final deck in ratios.keys) {
      _applyPosition(container, 10000, deck: deck);
    }
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 16));

    final msPerPx = {
      for (final ratio in ratios.values)
        ratio: visibleSourceMs(ratio, zoomMs) / _viewportWidth,
    };
    final baseline = {
      for (final entry in ratios.entries)
        entry.key: _sourceMsAt(
          _stripRect(tester, deckId: entry.key),
          speed: entry.value,
          visibleMs: zoomMs,
        ),
    };

    final probe = _startLaneRebuildProbe();
    addTearDown(probe.stop);

    const ages = [0.0, 8.0, 20.0, 33.0];
    final nextEventAt = {for (final deck in ratios.keys) deck: 0.0};
    final interval = {for (final deck in ratios.keys) deck: 33.0};
    final ageIndex = {for (final deck in ratios.keys) deck: 0};

    var wallMs = 0.0;
    for (var frame = 0; frame < 3750; frame++) {
      final before = {
        for (final deck in ratios.keys)
          deck: _stripRect(tester, deckId: deck).left,
      };
      final due = [
        for (final deck in ratios.keys)
          if (nextEventAt[deck]! <= wallMs + 16) deck,
      ];
      for (final deck in due) {
        final ratio = ratios[deck]!;
        // The poll reports the audible position when it arrives, minus the
        // latency it took to reach the UI.
        final age = ages[ageIndex[deck]! % ages.length];
        final reported = 10000 + (wallMs + 16 - age) * ratio;
        _applyPosition(container, reported.round(), deck: deck);
        nextEventAt[deck] = nextEventAt[deck]! + interval[deck]!;
        interval[deck] = interval[deck] == 33.0 ? 34.0 : 33.0;
        ageIndex[deck] = ageIndex[deck]! + 1;
      }
      if (due.isNotEmpty) {
        await tester.pump();
        for (final deck in ratios.keys) {
          expect(
            _stripRect(tester, deckId: deck).left,
            closeTo(before[deck]!, 1.0),
            reason: 'deck $deck moved on a position event at frame $frame',
          );
        }
      }
      await tester.pump(const Duration(milliseconds: 16));
      wallMs += 16;
      for (final entry in ratios.entries) {
        final deck = entry.key;
        final rect = _stripRect(tester, deckId: deck);
        final gained = (before[deck]! - rect.left) * msPerPx[entry.value]!;
        expect(
          gained,
          greaterThan(0),
          reason: 'deck $deck pulled backwards at frame $frame',
        );
        expect(
          gained,
          lessThan(16 * entry.value + 10),
          reason: 'deck $deck jumped ahead at frame $frame',
        );
        final shown = _sourceMsAt(rect, speed: entry.value, visibleMs: zoomMs);
        expect(
          (shown - (baseline[deck]! + wallMs * entry.value)).abs(),
          lessThan(kWaveformSeekSnapMs),
          reason: 'deck $deck drifted at frame $frame',
        );
      }
    }

    expect(
      probe.count(),
      0,
      reason: 'position ticks must not rebuild either lane',
    );
  });

  testWidgets('a decreasing rate retimes a pending correction', (tester) async {
    final container = await _pumpLane(
      tester,
      playing: true,
      playbackRatio: 1.0,
      strip: _strip,
    );
    final visibleMs = _zoomIn(container);
    await tester.pump();
    final display = await _startClock(
      tester,
      container,
      speed: 1.0,
      visibleMs: visibleMs,
    );

    // Schedule a backwards pull: the engine is 120 ms behind the display, so
    // the correction runs against the direction of playback at speed 1.0.
    _applyPosition(container, (display - 120).round());
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 32));

    // Drop the rate while that pull is still gliding. Its velocity was sized
    // for 1.0: at 25% it is 0.25 source-ms per ms, which is larger than 0.1
    // and would make net motion negative.
    _applyDeckUpdate(container, playbackRatio: 0.1);
    await tester.pump();

    final msPerPx = visibleSourceMs(0.1, visibleMs) / _viewportWidth;
    var previousLeft = _stripRect(tester).left;
    for (var frame = 0; frame < 40; frame++) {
      await tester.pump(const Duration(milliseconds: 16));
      final left = _stripRect(tester).left;
      expect(
        left,
        lessThanOrEqualTo(previousLeft + 1e-6),
        reason:
            'the retimed correction ran the slow deck backwards '
            '(frame $frame)',
      );
      final advanceMs = (previousLeft - left) * msPerPx;
      expect(
        advanceMs / 16,
        lessThanOrEqualTo(0.1 * 1.25 + 0.02),
        reason: 'the pull exceeded 25% of the new speed (frame $frame)',
      );
      previousLeft = left;
    }
  });

  testWidgets('a stopped lane detaches from the shared clock', (tester) async {
    final container = await _pumpLane(
      tester,
      playing: true,
      playbackRatio: 1.0,
      strip: _strip,
    );
    await tester.pump(const Duration(milliseconds: 16));
    expect(
      waveformFrameClock.isRunning,
      isTrue,
      reason: 'a playing lane must drive the shared clock',
    );

    _applyDeckUpdate(container, playing: false);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 16));

    expect(
      waveformFrameClock.isRunning,
      isFalse,
      reason: 'a stopped lane must not keep the shared ticker alive',
    );
  });

  testWidgets('a starting deck is not pulled by a position event before it '
      'rebuilds', (tester) async {
    // Deck 1 keeps the shared clock ticking, so a correction scheduled for the
    // stopped deck 0 would actually animate.
    const ratios = {0: 1.0, 1: 1.0};
    final container = await _pumpTwoLanes(tester, ratios: ratios);
    await tester.pump();

    _applyPosition(container, 10000, deck: 0);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 16));

    // Stop deck 0 and let the rebuild settle, so its state says "not playing".
    _applyDeckUpdate(container, deck: 0, playing: false);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 16));
    final stopped = _stripRect(tester, deckId: 0).left;

    // The playing flag flips first; the position event arrives before any
    // rebuild can update the lane's own playing state.
    _applyDeckUpdate(container, deck: 0, playing: true);
    _applyPosition(container, 9920, deck: 0);
    var previous = stopped;
    for (var frame = 0; frame < 20; frame++) {
      await tester.pump(const Duration(milliseconds: 16));
      final left = _stripRect(tester, deckId: 0).left;
      // Playing again legitimately moves the strip forward; a correction
      // scheduled against the frozen anchor would move it backwards instead.
      expect(
        left,
        lessThanOrEqualTo(previous + 1e-6),
        reason: 'the resumed deck moved backwards at frame $frame',
      );
      previous = left;
    }
  });

  testWidgets('pause and vinyl freeze a pending correction', (tester) async {
    final container = await _pumpLane(
      tester,
      playing: true,
      playbackRatio: 1.1857,
      strip: _strip,
    );
    final visibleMs = _zoomIn(container);
    await tester.pump();
    final display = await _startClock(
      tester,
      container,
      speed: 1.1857,
      visibleMs: visibleMs,
    );
    final msPerPx = visibleSourceMs(1.1857, visibleMs) / _viewportWidth;

    _applyPosition(container, (display + 80).round());
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 32));

    // Pause: the stopped position becomes authoritative, then freezes.
    _applyDeckUpdate(container, playing: false, playbackRatio: 1.1857);
    await tester.pump();
    final paused = _stripRect(tester);
    expect(
      _sourceMsAt(paused, speed: 1.1857, visibleMs: visibleMs),
      closeTo(display + 80, 2 * msPerPx),
      reason: 'a paused position event must display exactly',
    );
    await tester.pump(const Duration(milliseconds: 200));
    expect(
      _stripRect(tester).left,
      closeTo(paused.left, 1.0),
      reason: 'a paused lane must not keep correcting',
    );

    // Vinyl touch freezes too, and a newer touched position shows exactly.
    _applyDeckUpdate(
      container,
      playing: false,
      jogTouching: true,
      playbackRatio: 1.1857,
    );
    _applyPosition(container, 20000);
    await tester.pump();
    final touched = _stripRect(tester);
    expect(
      _sourceMsAt(touched, speed: 1.1857, visibleMs: visibleMs),
      closeTo(20000, 2 * msPerPx),
    );
    await tester.pump(const Duration(milliseconds: 200));
    expect(_stripRect(tester).left, closeTo(touched.left, 1.0));

    // Releasing the wheel resumes the advance.
    _applyDeckUpdate(
      container,
      playing: true,
      jogTouching: false,
      playbackRatio: 1.1857,
    );
    await tester.pump();
    final released = _stripRect(tester);
    await tester.pump(const Duration(milliseconds: 16));
    expect(_stripRect(tester).left, lessThan(released.left));
  });

  testWidgets('a ratio change keeps the source position continuous', (
    tester,
  ) async {
    final container = await _pumpLane(tester, playing: true, strip: _strip);
    final visibleMs = _zoomIn(container);
    await tester.pump();

    final display = await _startClock(
      tester,
      container,
      speed: 1,
      visibleMs: visibleMs,
    );
    _applyPosition(container, (display + 80).round());
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 32));

    final beforeSwitch = _sourceMsAt(
      _stripRect(tester),
      speed: 1,
      visibleMs: visibleMs,
    );
    _applyDeckUpdate(container, playing: true, playbackRatio: 1.1857);
    // No time passes: only the tempo and the visible source span change.
    await tester.pump();

    final newMsPerPx = visibleSourceMs(1.1857, visibleMs) / _viewportWidth;
    expect(
      _sourceMsAt(_stripRect(tester), speed: 1.1857, visibleMs: visibleMs),
      closeTo(beforeSwitch, 2 * newMsPerPx),
    );

    final switched = _stripRect(tester).left;
    await tester.pump(const Duration(milliseconds: 16));
    expect(
      _stripRect(tester).left,
      lessThan(switched),
      reason: 'playback continues at the new ratio',
    );
  });

  testWidgets('pointer down freezes a pending correction', (tester) async {
    final engine = FakeEngineTransport();
    final container = await _pumpLane(
      tester,
      playing: true,
      engine: engine,
      strip: _strip,
    );
    final visibleMs = _zoomIn(container);
    await tester.pump();

    final display = await _startClock(
      tester,
      container,
      speed: 1,
      visibleMs: visibleMs,
    );
    _applyPosition(container, (display + 80).round());
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 32));

    final held = _stripRect(tester);
    final gesture = await tester.startGesture(
      tester.getCenter(find.byType(ScrollingLane)),
    );
    await tester.pump();
    expect(_stripRect(tester).left, closeTo(held.left, 1.0));

    await tester.pump(const Duration(milliseconds: 200));
    expect(
      _stripRect(tester).left,
      closeTo(held.left, 1.0),
      reason: 'a stationary held pointer must not keep drifting',
    );

    await gesture.up();
    await tester.pump();
    // The double-tap zoom reset recognizer arms a timer on each tap.
    await tester.pump(const Duration(milliseconds: 400));
  });

  testWidgets('clearing the duration clears the display', (tester) async {
    final container = await _pumpLane(
      tester,
      playing: false,
      positionMs: 50000,
      strip: _strip,
    );
    await tester.pump();

    // A paused end-of-track position is authoritative, not clamped short.
    _applyPosition(container, _durationMs);
    await tester.pump();
    expect(
      _stripRect(tester).left,
      closeTo(_expectedLeft(_durationMs.toDouble()), 1.0),
    );

    _applyDeckUpdate(container, durationMs: 0);
    await tester.pump();
    expect(find.byType(WaveformScrollViewport), findsNothing);
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
