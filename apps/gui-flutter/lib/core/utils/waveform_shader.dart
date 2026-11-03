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

/// Widest peak texture we will ask the GPU for.
///
/// Vulkan only guarantees `maxImageDimension2D = 4096`, and GLES devices report
/// anything from 4096 up, while the library emits 4096-16384 overview buckets.
/// Wider graphs are max-pooled down to this, which is the correct reduction for
/// a peak (a bar shows the loudest bucket it covers) and keeps the loudest
/// transients visible.
const kMaxPeakTextureWidth = 4096;

/// Encodes [peaks] as an RGBA8 data texture. 8-bit is lossless for the real
/// data path: overview and window buckets are already uint8/255.
Future<WaveformPeaksTexture> encodeWaveformTexture(
  List<SpectralPeak> peaks,
) async {
  assert(peaks.isNotEmpty, 'A peak texture needs at least one bucket.');
  final width = peaks.length.clamp(1, kMaxPeakTextureWidth);
  final perTexel = peaks.length / width;
  final bytes = Uint8List(width * 4);
  for (var i = 0; i < width; i++) {
    // Max-pool the buckets this texel covers; equal to a copy when the graph
    // already fits.
    final start = (i * perTexel).floor();
    final end = ((i + 1) * perTexel).ceil().clamp(start + 1, peaks.length);
    var low = 0.0;
    var mid = 0.0;
    var high = 0.0;
    for (var b = start; b < end; b++) {
      final peak = peaks[b];
      if (peak.low > low) {
        low = peak.low;
      }
      if (peak.mid > mid) {
        mid = peak.mid;
      }
      if (peak.high > high) {
        high = peak.high;
      }
    }
    final o = i * 4;
    bytes[o] = _channel(low);
    bytes[o + 1] = _channel(mid);
    bytes[o + 2] = _channel(high);
    bytes[o + 3] = 255;
  }
  final buffer = await ui.ImmutableBuffer.fromUint8List(bytes);
  try {
    final descriptor = ui.ImageDescriptor.raw(
      buffer,
      width: width,
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
