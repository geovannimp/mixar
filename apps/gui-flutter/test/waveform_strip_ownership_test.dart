import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_strip.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

import 'support/fake_library_transport.dart';

const _trackId = 't1';
const _durationMs = 13000;

List<int> _rgb(int buckets) => [
  for (var i = 0; i < buckets; i++) ...[255, 128, 64],
];

class _WaveformTransport extends FakeLibraryTransport {
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
  }) async => WaveformPeaks(
    rgb: Uint8List.fromList(_rgb(buckets)),
    count: buckets,
    startMs: startMs,
    endMs: endMs,
  );
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('publishing a tile keeps the L0 texture the new strip shares', (
    tester,
  ) async {
    // L1 tiles are published by deriving a strip with `withTile`, which shares
    // L0 and the earlier tiles. If `_fillL1` forgets to hand ownership over
    // before publishing, `publish()` disposes the L0 texture the new strip
    // still paints and the lane goes blank until the next tile lands.
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryTransportProvider.overrideWith(
            (ref) async => _WaveformTransport(),
          ),
        ],
        child: Consumer(
          builder: (context, ref, _) {
            ref.watch(waveformStripProvider((_trackId, _durationMs)));
            return const SizedBox();
          },
        ),
      ),
    );
    final container = ProviderScope.containerOf(
      tester.element(find.byType(SizedBox)),
    );

    WaveformStrip? published;
    for (var attempt = 0; attempt < 200; attempt++) {
      // The tile path awaits real engine futures (encode) and drops replaced
      // strips in a post-frame callback, so it needs both clocks.
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 5)),
      );
      await tester.pump();
      final strip = container.read(
        waveformStripProvider((_trackId, _durationMs)),
      );
      if (strip != null && strip.tiles.isNotEmpty) {
        published = strip;
        break;
      }
    }

    expect(published, isNotNull, reason: 'the tile never published');
    // One more frame: that is when the deferred drop of the derived-from strip
    // runs, so a missing ownership transfer would show up as a disposed L0.
    await tester.pump();
    final seen = container.read(
      waveformStripProvider((_trackId, _durationMs)),
    )!;

    expect(
      seen.l0.disposed,
      isFalse,
      reason: 'L0 is owned by the strip that published the tile',
    );
    expect(seen.tiles.single.texture.disposed, isFalse);

    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 1));
  });
}
