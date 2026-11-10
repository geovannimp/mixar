import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/waveform_shader.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_strip.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_window.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

import 'support/fake_library_transport.dart';

const _trackId = 't1';
const _durationMs = 13000;
const _deckId = 0;
const (String, int, int) _key = (_trackId, _durationMs, _deckId);

List<int> _rgb(int buckets) => [
  for (var i = 0; i < buckets; i++) ...[255, 128, 64],
];

class _WaveformTransport extends FakeLibraryTransport {
  new() : hangWindow = false;

  /// When true, window fetches never resolve, so the strip on screen stays the
  /// previous one.
  bool hangWindow;

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
  }) {
    if (hangWindow) {
      return Completer<WaveformPeaks>().future;
    }
    return Future.value(
      WaveformPeaks(
        rgb: Uint8List.fromList(_rgb(buckets)),
        count: buckets,
        startMs: startMs,
        endMs: endMs,
      ),
    );
  }
}

class _FixedWindow extends WaveformWindowNotifier {
  new(super.deckId);

  static WaveformWindow window = (
    originMs: 0,
    spanMs: _durationMs,
    visibleMs: _durationMs,
  );

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

Future<WaveformStrip?> _awaitWindowStrip(
  WidgetTester tester,
  ProviderContainer container,
) async {
  WaveformStrip? found;
  // Encoding the peak texture awaits real engine futures, which only advance
  // outside the widget test's fake clock. Poll inside one real-time window so a
  // loaded parallel test run does not exhaust a per-iteration budget.
  await tester.runAsync(() async {
    for (var attempt = 0; attempt < 400 && found == null; attempt++) {
      await Future<void>.delayed(const Duration(milliseconds: 2));
      final strip = container.read(waveformStripProvider(_key));
      if (strip != null) {
        found = strip;
      }
    }
  });
  await tester.pump();
  return found;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() {
    _FixedWindow.window = (
      originMs: 0,
      spanMs: _durationMs,
      visibleMs: _durationMs,
    );
  });

  testWidgets('window strip publishes at the texture density', (tester) async {
    final container = await _pumpStrip(tester, transport: _WaveformTransport());
    final strip = await _awaitWindowStrip(tester, container);

    expect(strip, isNotNull, reason: 'the window never published');
    expect(strip!.originMs, 0);
    expect(strip.spanMs, _durationMs);
    expect(strip.widthPx, kMaxPeakTextureWidth);
    expect(strip.texture.count, kMaxPeakTextureWidth);
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

  testWidgets('a new window keeps the previous one painted until it lands', (
    tester,
  ) async {
    final transport = _WaveformTransport();
    final container = await _pumpStrip(tester, transport: transport);
    final first = await _awaitWindowStrip(tester, container);
    expect(first, isNotNull);

    // The next window never resolves, so nothing may replace the painted strip.
    transport.hangWindow = true;
    _FixedWindow.window = (
      originMs: 5000,
      spanMs: _durationMs - 5000,
      visibleMs: 6000,
    );
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
