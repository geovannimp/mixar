import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:gui_flutter/core/models/waveform_peaks.dart';

/// Loads `shaders/waveform.frag` once and hands out the program.
///
/// `main()` awaits [load] before `runApp`; painters draw only the lane fill
/// until [program] is non-null, so a missing shader asset surfaces as an
/// exception at startup instead of silently blank waveforms.
abstract final class WaveformShader {
  static const asset = 'shaders/waveform.frag';

  static ui.FragmentProgram? _program;
  static ui.FragmentShader? _shader;

  static ui.FragmentProgram? get program => _program;

  /// The one shader instance every painter configures and draws with.
  ///
  /// Re-setting uniforms and drawing is safe in sequence: the engine snapshots
  /// a draw's uniforms and sampler into the recorded display list, so a picture
  /// recorded earlier keeps its own values (pinned by the reuse test). One
  /// instance therefore avoids a native uniform-buffer allocation per paint,
  /// which the lane would otherwise pay for every frame.
  static ui.FragmentShader? get shader {
    final program = _program;
    if (program == null) {
      return null;
    }
    return _shader ??= program.fragmentShader();
  }

  static Future<void> load() async {
    if (_program != null) {
      return;
    }
    _program = await ui.FragmentProgram.fromAsset(asset);
    _shader = null;
  }
}

/// Bucket data for the shader: width == bucket count, height 1,
/// RGBA8 with R low, G mid, B high, A 255.
class WaveformPeaksTexture {
  new(this.image);

  final ui.Image image;

  var _disposed = false;

  bool get disposed => _disposed;

  int get count => image.width;

  /// Idempotent: a provider reload can hand the same texture to a painter
  /// after its scheduled disposal has already run, and double-disposing a
  /// `ui.Image` trips an assert.
  void dispose() {
    if (_disposed) {
      return;
    }
    _disposed = true;
    image.dispose();
  }
}

/// Encodes [peaks] as an RGBA8 data texture. 8-bit is lossless for the real
/// data path: overview and window buckets are already uint8/255.
Future<WaveformPeaksTexture> encodeWaveformTexture(
  List<SpectralPeak> peaks,
) async {
  assert(peaks.isNotEmpty, 'A peak texture needs at least one bucket.');
  final bytes = Uint8List(peaks.length * 4);
  for (var i = 0; i < peaks.length; i++) {
    final peak = peaks[i];
    final o = i * 4;
    bytes[o] = _channel(peak.low);
    bytes[o + 1] = _channel(peak.mid);
    bytes[o + 2] = _channel(peak.high);
    bytes[o + 3] = 255;
  }
  final buffer = await ui.ImmutableBuffer.fromUint8List(bytes);
  try {
    final descriptor = ui.ImageDescriptor.raw(
      buffer,
      width: peaks.length,
      height: 1,
      pixelFormat: ui.PixelFormat.rgba8888,
    );
    try {
      final codec = await descriptor.instantiateCodec();
      try {
        final frame = await codec.getNextFrame();
        return WaveformPeaksTexture(frame.image);
      } finally {
        codec.dispose();
      }
    } finally {
      // The descriptor holds a handle on the buffer, so it goes first.
      descriptor.dispose();
    }
  } finally {
    buffer.dispose();
  }
}

int _channel(double value) => (value.clamp(0.0, 1.0) * 255).round();
