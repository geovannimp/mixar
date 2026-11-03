import 'dart:typed_data';
import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/utils/waveform_gain.dart';
import 'package:gui_flutter/core/utils/waveform_picture.dart';
import 'package:gui_flutter/core/utils/waveform_shader.dart';
import 'package:gui_flutter/core/utils/waveform_spectral_color.dart';

const _background = Color(0xFFF4F4F5);
const _bg = (0xF4, 0xF4, 0xF5);
const _low = (255, 72, 48);
const _mid = (118, 228, 88);
const _high = (72, 188, 255);

/// Rasterised picture bytes, addressable by pixel.
class _Raster {
  new(this._bytes, this.width);

  final Uint8List _bytes;
  final int width;

  (int, int, int, int) at(int x, int y) {
    final i = (y * width + x) * 4;
    return (_bytes[i], _bytes[i + 1], _bytes[i + 2], _bytes[i + 3]);
  }

  (int, int, int) rgb(int x, int y) {
    final (r, g, b, _) = at(x, y);
    return (r, g, b);
  }
}

/// Paints the shader into a picture the way the lane and overview do, then
/// rasters it. The texture the painter samples is owned by the call.
Future<_Raster> _paint({
  required List<SpectralPeak> peaks,
  required Size size,
  Color background = _background,
  WaveformDisplayMode mode = WaveformDisplayMode.rgb,
  double gain = 1,
  WaveformBandGain bandGain = flatWaveformBandGain,
}) async {
  final texture = peaks.isEmpty ? null : await encodeWaveformTexture(peaks);
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  WaveformShaderPainter(
    texture: texture,
    background: background,
    mode: mode,
    gain: gain,
    bandGain: bandGain,
  ).paint(canvas, size);
  final picture = recorder.endRecording();
  try {
    final image = await picture.toImage(
      size.width.toInt(),
      size.height.toInt(),
    );
    try {
      final data = await image.toByteData();
      return _Raster(data!.buffer.asUint8List(), size.width.toInt());
    } finally {
      image.dispose();
    }
  } finally {
    picture.dispose();
    texture?.dispose();
  }
}

Matcher _near(int r, int g, int b, {int tolerance = 0}) => predicate(
  (value) =>
      value is (int, int, int) &&
      (value.$1 - r).abs() <= tolerance &&
      (value.$2 - g).abs() <= tolerance &&
      (value.$3 - b).abs() <= tolerance,
  'colour within $tolerance of ($r, $g, $b)',
);

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(WaveformShader.load);

  test('empty peaks paint nothing but the opaque lane fill', () async {
    // L1 tiles are drawn over the L0 strip, so a lane fill that is not opaque
    // would let the coarse strip show through the gaps between bars.
    final raster = await _paint(peaks: const [], size: const Size(2, 2));
    for (var y = 0; y < 2; y++) {
      for (var x = 0; x < 2; x++) {
        expect(raster.at(x, y), (0xF4, 0xF4, 0xF5, 0xFF));
      }
    }
  });

  test('rgb bar spans 0.46 x height around the lane centre', () async {
    final raster = await _paint(
      peaks: const [SpectralPeak(low: 1, mid: 0, high: 0)],
      size: const Size(4, 100),
    );

    for (var y = 0; y < 100; y++) {
      for (var x = 0; x < 4; x++) {
        // Anti-aliasing never lightens a solid bar, so alpha stays opaque
        // across the whole lane.
        expect(raster.at(x, y).$4, 0xFF);
        final inside = y >= 4 && y <= 95;
        expect(
          raster.rgb(x, y),
          inside ? _low : _bg,
          reason: 'row $y column $x',
        );
      }
    }
  });

  test('rgb bar fades with the 0.65 + amp * 0.35 alpha ramp', () async {
    final raster = await _paint(
      peaks: const [SpectralPeak(low: 128 / 255, mid: 0, high: 0)],
      size: const Size(4, 100),
    );
    // 0.65 + (128/255) * 0.35 = 0.8257 over the #F4F4F5 lane with #FF4830.
    expect(raster.rgb(0, 50), _near(253, 102, 82, tolerance: 2));
  });

  test('filtered mode stacks low under mid under high', () async {
    final raster = await _paint(
      peaks: const [SpectralPeak(low: 1, mid: 128 / 255, high: 64 / 255)],
      size: const Size(2, 100),
      mode: WaveformDisplayMode.filtered,
    );

    for (final y in [50, 44]) {
      expect(raster.rgb(0, y), _high, reason: 'row $y');
    }
    expect(raster.rgb(0, 36), _mid);
    expect(raster.rgb(0, 25), _low);
    for (final y in [3, 96]) {
      expect(raster.rgb(0, y), _bg, reason: 'row $y');
    }

    // Row 38 straddles the high band's edge (38.45): 0.545 of the pixel is
    // inside it, so the colour is a mix of mid and high, not either one.
    final edge = raster.rgb(0, 38);
    const t = 0.54509;
    final expected = (
      (_mid.$1 + (_high.$1 - _mid.$1) * t).round(),
      (_mid.$2 + (_high.$2 - _mid.$2) * t).round(),
      (_mid.$3 + (_high.$3 - _mid.$3) * t).round(),
    );
    expect(edge, _near(expected.$1, expected.$2, expected.$3, tolerance: 3));
    expect(edge, isNot(_mid));
    expect(edge, isNot(_high));
  });

  test('texture is one texel per bucket', () async {
    const peaks = [
      SpectralPeak(low: 1, mid: 0, high: 0),
      SpectralPeak(low: 0, mid: 1, high: 0),
      SpectralPeak(low: 0, mid: 0, high: 1),
    ];
    final texture = await encodeWaveformTexture(peaks);
    addTearDown(texture.dispose);
    expect(texture.count, 3);
    expect(texture.image.height, 1);
  });

  test('display mode changes the same peak from one bar to three', () async {
    const peaks = [SpectralPeak(low: 1, mid: 128 / 255, high: 64 / 255)];
    final rgbCentre = (await _paint(
      peaks: peaks,
      size: const Size(2, 100),
    )).rgb(0, 50);
    final filteredCentre = (await _paint(
      peaks: peaks,
      size: const Size(2, 100),
      mode: WaveformDisplayMode.filtered,
    )).rgb(0, 50);
    expect(filteredCentre, _high);
    expect(rgbCentre, isNot(filteredCentre));
    // The RGB bar is the band-weighted mix of the same three colours, so it
    // sits between them rather than on one.
    expect(rgbCentre, isNot(_low));
    expect(rgbCentre, isNot(_mid));
  });

  test(
    'columns walk the peak texture without clamping at either edge',
    () async {
      // Two buckets, low then high. The shader samples bucket centres with
      // linear filtering, so every column is a blend whose weight moves from
      // low to high across the picture. If the mapping overran the texture, the
      // first and last columns would clamp to a pure band colour instead.
      final raster = await _paint(
        peaks: const [
          SpectralPeak(low: 1, mid: 0, high: 0),
          SpectralPeak(low: 0, mid: 0, high: 1),
        ],
        size: const Size(4, 100),
      );
      final columns = [for (var x = 0; x < 4; x++) raster.rgb(x, 50)];

      for (var i = 1; i < columns.length; i++) {
        expect(
          columns[i].$3,
          greaterThan(columns[i - 1].$3),
          reason: 'blue must rise towards the last bucket',
        );
        expect(
          columns[i].$1,
          lessThan(columns[i - 1].$1),
          reason: 'red must fall away from the first bucket',
        );
      }
      expect(columns.first.$1, greaterThan(columns.first.$3));
      expect(columns.last.$3, greaterThan(columns.last.$1));
      expect(columns.first, isNot(_low), reason: 'left edge is interpolated');
      expect(columns.last, isNot(_high), reason: 'right edge is interpolated');
    },
  );

  test('channel gain and EQ scale bar height and band presence', () async {
    // Unity gain, flat EQ: the full 0.46 half-height.
    final flat = await _paint(
      peaks: const [SpectralPeak(low: 1, mid: 0, high: 0)],
      size: const Size(2, 100),
    );
    expect(flat.rgb(0, 4), isNot(_bg));
    expect(flat.rgb(0, 3), _bg);

    // Half channel gain halves the bar: 0.5 * 46 = 23 px around row 50.
    final quieter = await _paint(
      peaks: const [SpectralPeak(low: 1, mid: 0, high: 0)],
      size: const Size(2, 100),
      gain: 0.5,
    );
    expect(quieter.rgb(0, 26), _bg);
    expect(quieter.rgb(0, 27), isNot(_bg));
    expect(quieter.rgb(0, 72), isNot(_bg));
    expect(quieter.rgb(0, 73), _bg);

    // A killed band takes its bars with it; the other bands stay.
    final killed = await _paint(
      peaks: const [SpectralPeak(low: 1, mid: 0, high: 1)],
      size: const Size(2, 100),
      mode: WaveformDisplayMode.filtered,
      bandGain: (1, 1, 0),
    );
    expect(killed.rgb(0, 50), _low, reason: 'low band survives the high kill');
    expect(killed.rgb(0, 4), _low);
  });

  test('a flat channel maps to unity and a killed band to zero', () {
    // 0.5 is a flat engine knob; if it did not map to 1.0 every loaded deck
    // would render half-height bars.
    expect(waveformBandGainFor(MixerChannelUi.defaults), (1, 1, 1));
    expect(
      waveformBandGainFor(
        const MixerChannelUi(eqLow: 0, eqMid: 1, eqHigh: 0.25),
      ),
      (0, 2, 0.5),
    );
  });

  test('a disposed texture only paints the lane fill', () async {
    // Provider reloads keep the previous value visible until the next one
    // lands, so a painter can be handed a texture whose scheduled disposal
    // already ran. Sampling it asserts in debug and reads a freed image in
    // release, so the painter must degrade instead.
    final texture = await encodeWaveformTexture(const [
      SpectralPeak(low: 1, mid: 0, high: 0),
    ]);
    texture.dispose();
    expect(texture.dispose, returnsNormally, reason: 'disposal is idempotent');
    expect(texture.disposed, isTrue);

    final recorder = PictureRecorder();
    WaveformShaderPainter(texture: texture, background: _background).paint(
      Canvas(recorder, const Rect.fromLTWH(0, 0, 2, 2)),
      const Size(2, 2),
    );
    final picture = recorder.endRecording();
    final image = await picture.toImage(2, 2);
    final data = await image.toByteData();
    image.dispose();
    picture.dispose();
    expect(data!.buffer.asUint8List(), [
      for (var i = 0; i < 4; i++) ...[0xF4, 0xF4, 0xF5, 0xFF],
    ]);
  });
}
