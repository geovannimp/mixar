import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_strip.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_window.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

import 'support/fake_library_transport.dart';

const _trackId = 't1';
const _durationMs = 180000;
const _deckId = 0;
const (String, int, int) _key = (_trackId, _durationMs, _deckId);

/// One recorded `getWaveformWindow` request.
typedef _WindowRequest = ({int startMs, int endMs, int buckets});

List<int> _rgb(int buckets) => [
  for (var i = 0; i < buckets; i++) ...[255, 128, 64],
];

class _WaveformTransport extends FakeLibraryTransport {
  new() : hangWindow = false;

  /// When true, window fetches never resolve, so the strip on screen stays the
  /// previous one.
  bool hangWindow;

  /// Every window fetch, in request order.
  final List<_WindowRequest> windowRequests = [];

  /// Returned bounds; default to the request's own (the host clamps, so it may
  /// answer with a narrower interval than asked).
  ({int startMs, int endMs})? responseBounds;

  /// Returned bucket count; defaults to the requested one.
  int? responseBuckets;

  /// Answer with `endMs <= startMs`, which the strip must ignore.
  bool invertBounds = false;

  /// Answer with no peaks at all.
  bool emptyWindow = false;

  /// While set, window fetches wait for it before answering, so a test can
  /// hold a refresh in flight.
  Completer<void>? hold;

  @override
  Future<WaveformPeaks?> getWaveformOverview({required String trackId}) async =>
      WaveformPeaks(
        rgb: Uint8List.fromList(_rgb(64)),
        count: 64,
        startMs: 0,
        endMs: _durationMs,
      );

  @override
  Future<WaveformPeaks> getWaveformWindow({
    required String trackId,
    required int startMs,
    required int endMs,
    required int buckets,
  }) async {
    windowRequests.add((startMs: startMs, endMs: endMs, buckets: buckets));
    final gate = hold;
    if (gate != null) {
      await gate.future;
    }
    if (hangWindow) {
      return Completer<WaveformPeaks>().future;
    }
    final bounds = invertBounds
        ? (startMs: endMs, endMs: startMs)
        : responseBounds ?? (startMs: startMs, endMs: endMs);
    final count = responseBuckets ?? buckets;
    if (emptyWindow) {
      return WaveformPeaks(
        rgb: Uint8List(0),
        count: 0,
        startMs: bounds.startMs,
        endMs: bounds.endMs,
      );
    }
    return WaveformPeaks(
      rgb: Uint8List.fromList(_rgb(count)),
      count: count,
      startMs: bounds.startMs,
      endMs: bounds.endMs,
    );
  }
}

class _FixedWindow extends WaveformWindowNotifier {
  new(super.deckId);

  static WaveformWindow window = (originMs: 0, spanMs: 36000, visibleMs: 24000);

  @override
  WaveformWindow build() => window;
}

Future<ProviderContainer> _pumpStrip(
  WidgetTester tester, {
  required _WaveformTransport transport,
}) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        libraryTransportProvider.overrideWith((ref) async => transport),
        waveformWindowProvider.overrideWith2(_FixedWindow.new),
      ],
      child: Consumer(
        builder: (context, ref, _) {
          ref.watch(waveformStripProvider(_key));
          return const SizedBox();
        },
      ),
    ),
  );
  return ProviderScope.containerOf(tester.element(find.byType(SizedBox)));
}

/// Polls [until] on the real clock until it returns non-null, then pumps.
///
/// Encoding a peak texture awaits real engine futures, which only advance
/// outside the widget test's fake clock. Poll inside one real-time window so a
/// loaded parallel test run does not exhaust a per-iteration budget.
Future<T?> _poll<T>(WidgetTester tester, T? Function() until) async {
  T? found;
  await tester.runAsync(() async {
    for (var attempt = 0; attempt < 400; attempt++) {
      final value = until();
      if (value != null) {
        found = value;
        return;
      }
      await Future<void>.delayed(const Duration(milliseconds: 2));
    }
  });
  await tester.pump();
  return found;
}

/// Waits for a published strip satisfying [matches] (any non-null by default).
Future<WaveformStrip?> _awaitWindowStrip(
  WidgetTester tester,
  ProviderContainer container, {
  bool Function(WaveformStrip strip)? matches,
}) => _poll(tester, () {
  final strip = container.read(waveformStripProvider(_key));
  if (strip == null || (matches != null && !matches(strip))) {
    return null;
  }
  return strip;
});

/// Waits until the transport has seen [count] window requests.
Future<void> _awaitRequests(
  WidgetTester tester,
  _WaveformTransport transport,
  int count,
) async {
  await _poll(
    tester,
    () => transport.windowRequests.length >= count ? count : null,
  );
}

/// Lets an in-flight response settle inside the real-time window.
Future<void> _drain(WidgetTester tester) async {
  await tester.runAsync(() async {
    for (var i = 0; i < 10; i++) {
      await Future<void>.delayed(const Duration(milliseconds: 2));
    }
  });
  await tester.pump();
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() {
    _FixedWindow.window = (originMs: 0, spanMs: 36000, visibleMs: 24000);
  });

  testWidgets('window strip publishes at the requested density', (
    tester,
  ) async {
    final transport = _WaveformTransport();
    final container = await _pumpStrip(tester, transport: transport);
    final strip = await _awaitWindowStrip(tester, container);

    expect(strip, isNotNull, reason: 'the window never published');
    expect(transport.windowRequests, hasLength(1));
    expect(transport.windowRequests.single, (
      startMs: 0,
      endMs: 36000,
      buckets: 2048,
    ));
    expect(strip!.originMs, 0);
    expect(strip.spanMs, 36000);
    expect(strip.widthPx, strip.texture.count);
    expect(strip.texture.count, 2048);
    expect(strip.texture.disposed, isFalse);

    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 1));
  });

  testWidgets('an analysis refresh keeps the strip on screen', (tester) async {
    final container = await _pumpStrip(tester, transport: _WaveformTransport());
    final first = await _awaitWindowStrip(tester, container);
    expect(first, isNotNull);

    container.read(libraryAnalysisEpochProvider.notifier).bump();
    final kept = container.read(waveformStripProvider(_key));

    expect(
      identical(kept, first),
      isTrue,
      reason: 'the previous window stays painted while the refetch runs',
    );
    expect(first!.texture.disposed, isFalse);

    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 1));
  });

  testWidgets('an expanded window adds buckets without thinning detail', (
    tester,
  ) async {
    final transport = _WaveformTransport();
    final container = await _pumpStrip(tester, transport: transport);
    final first = await _awaitWindowStrip(tester, container);
    expect(first, isNotNull);
    expect(first!.texture.count, 2048);

    // Hold the refresh so the painted strip can be inspected mid-flight.
    transport.hold = Completer<void>();
    _FixedWindow.window = (originMs: 0, spanMs: 54000, visibleMs: 24000);
    container.invalidate(waveformWindowProvider(_deckId));
    container.read(waveformWindowProvider(_deckId));
    await _awaitRequests(tester, transport, 2);

    expect(
      identical(container.read(waveformStripProvider(_key)), first),
      isTrue,
      reason: 'the first strip stays alive while response two is pending',
    );
    expect(first.texture.disposed, isFalse);

    transport.hold!.complete();
    final second = await _awaitWindowStrip(
      tester,
      container,
      matches: (strip) => !identical(strip, first),
    );

    expect(transport.windowRequests.map((r) => r.buckets), [2048, 3072]);
    expect(second, isNotNull);
    expect(second!.texture.count, 3072, reason: 'no density step on expand');
    expect(second.widthPx, second.texture.count);
    expect(second.originMs, 0);
    expect(second.spanMs, 54000);

    // Same nominal time per bucket in both crops.
    final perBucket = transport.windowRequests
        .map((r) => (r.endMs - r.startMs) / r.buckets)
        .toSet();
    expect(perBucket, hasLength(1));

    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 1));
  });

  testWidgets('bucket density follows the requested span', (tester) async {
    final transport = _WaveformTransport();
    final container = await _pumpStrip(tester, transport: transport);
    final first = await _awaitWindowStrip(tester, container);
    expect(first, isNotNull);

    Future<WaveformStrip?> refetch(
      ({int originMs, int spanMs, int visibleMs}) window, {
      required int originMs,
      required int count,
    }) async {
      _FixedWindow.window = window;
      container.invalidate(waveformWindowProvider(_deckId));
      container.read(waveformWindowProvider(_deckId));
      return _awaitWindowStrip(
        tester,
        container,
        matches: (strip) =>
            strip.originMs == originMs && strip.texture.count == count,
      );
    }

    // Full nominal three-visible-span window.
    final full = await refetch(
      (originMs: 0, spanMs: 72000, visibleMs: 24000),
      originMs: 0,
      count: 4096,
    );
    expect(full, isNotNull, reason: '0-72000 should request 4096 buckets');
    expect(full!.spanMs, 72000);
    expect(full.widthPx, full.texture.count);

    // Tail-clipped: the host cannot analyze past the track.
    final tail = await refetch(
      (originMs: 144000, spanMs: 36000, visibleMs: 24000),
      originMs: 144000,
      count: 2048,
    );
    expect(tail, isNotNull, reason: '144000-180000 should request 2048');
    expect(transport.windowRequests.last, (
      startMs: 144000,
      endMs: 180000,
      buckets: 2048,
    ));

    // Zoomed in: a shorter span at the same texture budget is denser.
    final zoom = await refetch(
      (originMs: 0, spanMs: 18000, visibleMs: 6000),
      originMs: 0,
      count: 4096,
    );
    expect(
      zoom,
      isNotNull,
      reason: '18000 ms at 6000 ms visible should max out',
    );
    expect(transport.windowRequests.last, (
      startMs: 0,
      endMs: 18000,
      buckets: 4096,
    ));

    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 1));
  });

  testWidgets('density is part of what a strip was recorded for', (
    tester,
  ) async {
    final transport = _WaveformTransport();
    final container = await _pumpStrip(tester, transport: transport);
    final first = await _awaitWindowStrip(tester, container);
    expect(first, isNotNull);
    expect(first!.texture.count, 2048);

    // Same bounds, twice the density: this must refetch, not reuse the strip.
    _FixedWindow.window = (originMs: 0, spanMs: 36000, visibleMs: 12000);
    container.invalidate(waveformWindowProvider(_deckId));
    container.read(waveformWindowProvider(_deckId));

    final denser = await _awaitWindowStrip(
      tester,
      container,
      matches: (strip) => !identical(strip, first),
    );
    expect(transport.windowRequests.map((r) => r.buckets), [2048, 4096]);
    expect(transport.windowRequests.last.startMs, 0);
    expect(transport.windowRequests.last.endMs, 36000);
    expect(denser, isNotNull);
    expect(denser!.texture.count, 4096);

    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 1));
  });

  testWidgets('publishes the bounds the host analyzed and keeps them on junk', (
    tester,
  ) async {
    final transport = _WaveformTransport()
      ..responseBounds = (startMs: 500, endMs: 35000);
    final container = await _pumpStrip(tester, transport: transport);
    final first = await _awaitWindowStrip(tester, container);

    expect(first, isNotNull);
    expect(first!.originMs, 500, reason: 'origin comes from the response');
    expect(first.spanMs, 34500);
    expect(first.widthPx, first.texture.count);
    expect(first.texture.count, 2048);

    // An inverted response has nothing to paint.
    transport.invertBounds = true;
    container.read(libraryAnalysisEpochProvider.notifier).bump();
    container.read(waveformStripProvider(_key));
    await _awaitRequests(tester, transport, 2);
    await _drain(tester);

    expect(
      identical(container.read(waveformStripProvider(_key)), first),
      isTrue,
      reason: 'an inverted window must retain the painted strip',
    );

    // So does an empty one.
    transport
      ..invertBounds = false
      ..emptyWindow = true;
    container.read(libraryAnalysisEpochProvider.notifier).bump();
    container.read(waveformStripProvider(_key));
    await _awaitRequests(tester, transport, 3);
    await _drain(tester);

    expect(
      identical(container.read(waveformStripProvider(_key)), first),
      isTrue,
      reason: 'an empty window must retain the painted strip',
    );
    expect(first.texture.disposed, isFalse);

    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 1));
  });

  testWidgets('a new window keeps the previous one painted until it lands', (
    tester,
  ) async {
    final transport = _WaveformTransport();
    final container = await _pumpStrip(tester, transport: transport);
    final first = await _awaitWindowStrip(tester, container);
    expect(first, isNotNull);

    // The next window never resolves, so nothing may replace the painted strip.
    transport.hangWindow = true;
    _FixedWindow.window = (originMs: 5000, spanMs: 18000, visibleMs: 6000);
    container.invalidate(waveformWindowProvider(_deckId));
    container.read(waveformWindowProvider(_deckId));

    final kept = container.read(waveformStripProvider(_key));

    expect(
      identical(kept, first),
      isTrue,
      reason: 'no coarse or blank frame may replace the zoomed window',
    );
    expect(first!.texture.disposed, isFalse);

    // Drain the deferred release so the held texture is freed at teardown.
    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 1));
  });
}
