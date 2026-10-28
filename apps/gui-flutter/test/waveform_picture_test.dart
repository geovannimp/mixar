import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/utils/waveform_picture.dart';

const _background = Color(0xFFF4F4F5);

Future<List<int>> _pixels(Picture picture, int size) async {
  final image = await picture.toImage(size, size);
  final bytes = await image.toByteData();
  image.dispose();
  return bytes!.buffer.asUint8List();
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('recorded pictures are opaque lane background under the bars', () async {
    // Detail tiles are drawn over the low-res overview. A tile only replaces
    // it when the whole picture is opaque: a translucent tile would let the
    // coarse overview show through the gaps between its bars.
    final empty = recordWaveformPicture(
      overview: const [],
      durationMs: 1000,
      originMs: 0,
      spanMs: 1000,
      size: const Size(2, 2),
      background: _background,
    );
    addTearDown(empty.dispose);
    expect(await _pixels(empty, 2), [
      for (var i = 0; i < 4; i++) ...[0xF4, 0xF4, 0xF5, 0xFF],
    ]);

    final bars = recordWaveformPicture(
      overview: const [SpectralPeak(low: 1, mid: 1, high: 1)],
      durationMs: 1000,
      originMs: 0,
      spanMs: 1000,
      size: const Size(2, 2),
      background: _background,
    );
    addTearDown(bars.dispose);
    final painted = await _pixels(bars, 2);
    var barPixels = 0;
    for (var i = 0; i < painted.length; i += 4) {
      final (r, g, b, a) = (
        painted[i],
        painted[i + 1],
        painted[i + 2],
        painted[i + 3],
      );
      expect(a, 0xFF);
      if (r != 0xF4 || g != 0xF4 || b != 0xF5) {
        barPixels++;
      }
    }
    // The opaque background alone would satisfy the alpha check, so require at
    // least one pixel the bars actually changed.
    expect(barPixels, greaterThan(0));
  });
}
