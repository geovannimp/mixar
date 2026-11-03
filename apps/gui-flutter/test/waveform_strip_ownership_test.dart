import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/utils/waveform_shader.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_strip.dart';

Future<WaveformPeaksTexture> _texture() =>
    encodeWaveformTexture(const [SpectralPeak(low: 1, mid: 0.5, high: 0.2)]);

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('a tile publish keeps the shared L0 texture alive', () async {
    // L1 tiles are published by deriving a strip that shares L0 and the earlier
    // tiles (withTile). Disposing the derived-from strip would dispose those
    // shared textures and blank the lane until the next tile landed.
    final l0 = await _texture();
    final tileTexture = await _texture();
    final strip = WaveformStrip(
      durationMs: 100000,
      widthPx: 1000,
      heightPx: 64,
      l0: l0,
    );
    addTearDown(tileTexture.dispose);

    final next = strip.withTile(
      WaveformTile(startPx: 0, widthPx: 512, texture: tileTexture),
    );
    strip
      ..relinquish()
      ..dispose();

    expect(l0.disposed, isFalse, reason: 'L0 is now owned by the new strip');
    expect(tileTexture.disposed, isFalse);

    next.dispose();
    expect(l0.disposed, isTrue, reason: 'the head disposes what it owns');
    expect(tileTexture.disposed, isTrue);
  });
}
