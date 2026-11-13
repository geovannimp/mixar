import 'dart:typed_data';

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/mixer/waveform/waveform_scroll_viewport.dart';

const _viewportSize = Size(400, 80);
const _stripSize = Size(1000, 80);
const _originMs = 1000;
const _pxPerMs = 0.1;
const _dpr = 2.0;

const _stripKey = Key('strip');
const _viewportRootKey = Key('viewport-root');

/// Size of the pixel-test canvas the viewport sits inside.
const _captureSize = Size(600, 120);

/// Reports how many listeners a tick source holds.
class _CountingNotifier extends ChangeNotifier {
  new(this.value);

  double value;

  final Set<VoidCallback> _listeners = <VoidCallback>{};

  int get listenerCount => _listeners.length;

  void moveTo(double next) {
    value = next;
    notifyListeners();
  }

  @override
  void addListener(VoidCallback listener) {
    _listeners.add(listener);
    super.addListener(listener);
  }

  @override
  void removeListener(VoidCallback listener) {
    _listeners.remove(listener);
    super.removeListener(listener);
  }
}

class _LayoutCounter {
  int count = 0;
}

/// Wraps the strip in a proxy box that counts layout passes.
class _LayoutProbe extends SingleChildRenderObjectWidget {
  const new({required this.counter, super.child});

  final _LayoutCounter counter;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _LayoutProbeBox(counter);
}

class _LayoutProbeBox extends RenderProxyBox {
  new(this.counter);

  final _LayoutCounter counter;

  @override
  void performLayout() {
    counter.count++;
    super.performLayout();
  }
}

Widget _harness({
  required Listenable playhead,
  required double Function() displayMs,
  int originMs = _originMs,
  double pxPerMs = _pxPerMs,
  Size stripSize = _stripSize,
  double dpr = _dpr,
  Size viewportSize = _viewportSize,
  Widget? strip,
}) {
  return Directionality(
    textDirection: TextDirection.ltr,
    child: Align(
      alignment: Alignment.topLeft,
      child: SizedBox.fromSize(
        size: viewportSize,
        child: WaveformScrollViewport(
          playhead: playhead,
          displayMs: displayMs,
          originMs: originMs,
          pxPerMs: pxPerMs,
          stripSize: stripSize,
          dpr: dpr,
          child: strip ?? const SizedBox(key: _stripKey),
        ),
      ),
    ),
  );
}

/// Top-left of the viewport in global coordinates.
Offset _viewportOrigin(WidgetTester tester) =>
    tester.getTopLeft(find.byType(WaveformScrollViewport));

/// Painted strip rect, in viewport-local coordinates.
///
/// [WidgetTester.getRect] walks `RenderObject.applyPaintTransform`, so the
/// result includes the translate the delegate paints — the child's layout
/// offset stays at zero, so a fixed translate would show up here as the same
/// rect on every position.
Rect _stripRect(WidgetTester tester, {Finder? strip}) {
  final origin = _viewportOrigin(tester);
  final rect = tester.getRect(strip ?? find.byKey(_stripKey));
  return rect.translate(-origin.dx, -origin.dy);
}

Future<void> _tick(
  WidgetTester tester,
  ValueNotifier<double> position,
  double ms,
) async {
  position.value = ms;
  await tester.pump();
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('translation follows the source position and snaps to the dpr', (
    tester,
  ) async {
    final position = ValueNotifier<double>(_originMs.toDouble());
    addTearDown(position.dispose);

    await tester.pumpWidget(
      _harness(playhead: position, displayMs: () => position.value),
    );

    expect(_stripRect(tester).left, 200);

    // That rect is the *painted* position, not a layout offset: the child's
    // layout offset stays zero and only this transform moves it.
    final flow = tester.renderObject<RenderFlow>(find.byType(Flow));
    final painted = Matrix4.identity();
    flow.applyPaintTransform(flow.firstChild!, painted);
    expect(painted.getTranslation().x, 200);

    await _tick(tester, position, 2000);
    expect(_stripRect(tester).left, 100);

    await _tick(tester, position, 1004);
    expect(_stripRect(tester).left, 199.5);

    // A signed position keeps sliding: the strip is not clamped to the track.
    await _tick(tester, position, -100);
    expect(_stripRect(tester).left, 310);

    await _tick(tester, position, 12000);
    expect(_stripRect(tester).left, -900);
  });

  testWidgets('a tick repaints without rebuilding or relaying out', (
    tester,
  ) async {
    final position = ValueNotifier<double>(1000);
    addTearDown(position.dispose);
    final layouts = _LayoutCounter();

    await tester.pumpWidget(
      _harness(
        playhead: position,
        displayMs: () => position.value,
        strip: _LayoutProbe(
          counter: layouts,
          child: const SizedBox(key: _stripKey),
        ),
      ),
    );
    await tester.pump();

    final flow = tester.renderObject<RenderFlow>(find.byType(Flow));
    final viewportElement = tester.element(find.byType(WaveformScrollViewport));
    final flowElement = tester.element(find.byType(Flow));
    final layoutsBefore = layouts.count;
    expect(flow.debugNeedsPaint, isFalse);

    position.value = 2000;

    expect(
      flow.debugNeedsPaint,
      isTrue,
      reason: 'the repaint listenable must mark the viewport for painting',
    );
    expect(flow.debugNeedsLayout, isFalse);
    expect(viewportElement.dirty, isFalse);
    expect(flowElement.dirty, isFalse);

    var rebuilds = 0;
    final previous = debugOnRebuildDirtyWidget;
    debugOnRebuildDirtyWidget = (element, builtOnce) {
      rebuilds++;
      previous?.call(element, builtOnce);
    };
    addTearDown(() => debugOnRebuildDirtyWidget = previous);

    final painted = <double>[];
    for (final ms in <double>[2000, 3000, 4000, 5000]) {
      position.value = ms;
      await tester.pump();
      painted.add(_stripRect(tester).left);
    }

    expect(painted, <double>[100, 0, -100, -200]);
    expect(rebuilds, 0);
    expect(layouts.count, layoutsBefore);
  });

  testWidgets('hit tests follow the painted translation', (tester) async {
    final position = ValueNotifier<double>(2000);
    addTearDown(position.dispose);
    final hits = <Offset>[];

    await tester.pumpWidget(
      _harness(
        playhead: position,
        displayMs: () => position.value,
        strip: Listener(
          key: _stripKey,
          behavior: HitTestBehavior.opaque,
          onPointerDown: (event) => hits.add(event.localPosition),
          child: const SizedBox(),
        ),
      ),
    );
    await tester.pump();

    // Strip left 100: a point 150 px into the viewport is 50 px into the strip.
    await _tapAt(tester, const Offset(150, 40));
    expect(hits, <Offset>[const Offset(50, 40)]);

    // The same 150 px would be outside the strip at x=50, where the strip has
    // not been translated over.
    hits.clear();
    await _tapAt(tester, const Offset(50, 40));
    expect(hits, isEmpty);

    // A newer translation must invalidate the old one: strip left 0 here.
    hits.clear();
    await _tick(tester, position, 3000);
    await _tapAt(tester, const Offset(150, 40));
    expect(hits, <Offset>[const Offset(150, 40)]);
  });

  testWidgets('geometry changes apply on the frame they land on', (
    tester,
  ) async {
    final position = ValueNotifier<double>(2000);
    addTearDown(position.dispose);

    await tester.pumpWidget(
      _harness(playhead: position, displayMs: () => position.value),
    );
    expect(_stripRect(tester).left, 100);

    await tester.pumpWidget(
      _harness(
        playhead: position,
        displayMs: () => position.value,
        originMs: 0,
        pxPerMs: 0.2,
        dpr: 1,
        stripSize: const Size(1200, 60),
      ),
    );
    expect(_stripRect(tester), const Rect.fromLTWH(-200, 0, 1200, 60));

    // Centring uses the live viewport width, not a cached one.
    await tester.pumpWidget(
      _harness(
        playhead: position,
        displayMs: () => position.value,
        viewportSize: const Size(600, 80),
      ),
    );
    expect(_stripRect(tester).left, 200);
  });

  testWidgets(
    'replacing the tick source moves the strip and detaches the old',
    (tester) async {
      final first = _CountingNotifier(2000);
      final second = _CountingNotifier(3000);
      addTearDown(first.dispose);
      addTearDown(second.dispose);

      await tester.pumpWidget(
        _harness(playhead: first, displayMs: () => first.value),
      );
      final element = tester.element(find.byType(WaveformScrollViewport));
      expect(first.listenerCount, 1);
      expect(_stripRect(tester).left, 100);

      await tester.pumpWidget(
        _harness(playhead: second, displayMs: () => second.value),
      );

      expect(
        tester.element(find.byType(WaveformScrollViewport)),
        same(element),
        reason: 'the viewport element must survive a source swap',
      );
      expect(first.listenerCount, 0);
      expect(second.listenerCount, 1);
      expect(_stripRect(tester).left, 0);

      second.moveTo(4000);
      await tester.pump();
      expect(_stripRect(tester).left, -100);

      first.moveTo(0);
      await tester.pump();
      expect(
        _stripRect(tester).left,
        -100,
        reason: 'the detached source must not move the strip',
      );

      await tester.pumpWidget(const SizedBox());

      expect(second.listenerCount, 0);
      expect(() => second.moveTo(0), returnsNormally);
    },
  );

  testWidgets('the viewport paints the strip outside its own bounds', (
    tester,
  ) async {
    final position = ValueNotifier<double>(1000);
    addTearDown(position.dispose);

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: RepaintBoundary(
            key: _viewportRootKey,
            child: SizedBox.fromSize(
              size: _captureSize,
              child: ColoredBox(
                color: const Color(0xFFFFFFFF),
                // Align loosens the constraints so the viewport keeps its size.
                child: Align(
                  alignment: Alignment.topLeft,
                  child: Padding(
                    padding: const EdgeInsets.only(left: 100, top: 20),
                    // Stands in for the lane's ClipRect around the viewport.
                    child: ClipRect(
                      child: SizedBox(
                        width: _viewportSize.width,
                        height: _viewportSize.height,
                        child: ColoredBox(
                          color: const Color(0xFF000000),
                          child: WaveformScrollViewport(
                            playhead: position,
                            displayMs: () => position.value,
                            originMs: _originMs,
                            pxPerMs: _pxPerMs,
                            stripSize: _stripSize,
                            dpr: 1,
                            child: Stack(
                              clipBehavior: Clip.none,
                              children: [
                                // Paint reaching left of the strip's own edge.
                                Positioned.fromRect(
                                  rect: const Rect.fromLTWH(-250, 20, 100, 40),
                                  child: const ColoredBox(
                                    color: Color(0xFFFF0000),
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );

    expect(tester.widget<Flow>(find.byType(Flow)).clipBehavior, Clip.none);

    final boundary = tester.renderObject<RenderRepaintBoundary>(
      find.byKey(_viewportRootKey),
    );
    final capture = await tester.runAsync(() async {
      final image = await boundary.toImage();
      final data = await image.toByteData();
      image.dispose();
      return data!.buffer.asUint8List();
    });

    // Strip left 200 puts the marker at x 50..150, straddling the viewport
    // edge at x=100.
    _expectColour(capture!, 110, 60, (255, 0, 0));
    _expectColour(capture, 90, 60, (255, 255, 255));
    _expectColour(capture, 160, 60, (0, 0, 0));
  });
}

Future<void> _tapAt(WidgetTester tester, Offset localPosition) async {
  final gesture = await tester.startGesture(
    _viewportOrigin(tester) + localPosition,
  );
  await gesture.up();
}

void _expectColour(Uint8List rgba, int x, int y, (int, int, int) expected) {
  final offset = (y * _captureSize.width.toInt() + x) * 4;
  final actual = (rgba[offset], rgba[offset + 1], rgba[offset + 2]);
  expect(
    (actual.$1 - expected.$1).abs() < 8 &&
        (actual.$2 - expected.$2).abs() < 8 &&
        (actual.$3 - expected.$3).abs() < 8,
    isTrue,
    reason: 'pixel ($x, $y) is $actual, expected $expected',
  );
}
