import 'dart:ui';

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/utils/waveform_shader.dart';
import 'package:gui_flutter/core/utils/waveform_spectral_color.dart';

class WaveformShaderPainter extends CustomPainter {
  new({
    required this.texture,
    required this.background,
    this.mode = WaveformDisplayMode.rgb,
  });

  /// Bucket texture for exactly the pixels this painter covers.
  final WaveformPeaksTexture? texture;
  final Color background;
  final WaveformDisplayMode mode;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = background);
    final program = WaveformShader.program;
    final peaks = texture;
    // A reload keeps the previous value visible until the new one lands, so a
    // painter can be handed a texture whose scheduled disposal already ran.
    // Sampling it would assert in debug and read a freed image in release.
    if (program == null || peaks == null || peaks.disposed || peaks.count < 1) {
      return;
    }
    if (size.width <= 0 || size.height <= 0) {
      return;
    }
    // One FragmentShader per paint. A FragmentShader carries a single mutable
    // uniform buffer, and dart:ui requires two objects for two sets of uniforms
    // that must coexist; recorded pictures here coexist (L0 strip + tiles), so
    // they may not share one.
    final shader = program.fragmentShader();
    shader.getUniformVec2('uSize').set(size.width, size.height);
    shader.getUniformFloat('uPeakCount').set(peaks.count.toDouble());
    shader
        .getUniformFloat('uMode')
        .set(mode == WaveformDisplayMode.filtered ? 1.0 : 0.0);
    shader
        .getUniformVec4('uBackground')
        .set(background.r, background.g, background.b, 1);
    shader
        .getUniformVec4('uLowColor')
        .set(kLowColor.r, kLowColor.g, kLowColor.b, 1);
    shader
        .getUniformVec4('uMidColor')
        .set(kMidColor.r, kMidColor.g, kMidColor.b, 1);
    shader
        .getUniformVec4('uHighColor')
        .set(kHighColor.r, kHighColor.g, kHighColor.b, 1);
    // `low` is bilinear: one linear filter and no mipmap, so a picture wider
    // than the texture interpolates between neighbouring buckets instead of
    // picking one.
    shader.setImageSampler(0, peaks.image, filterQuality: FilterQuality.low);
    canvas.drawRect(
      Offset.zero & size,
      Paint()
        ..isAntiAlias = false
        ..shader = shader,
    );
  }

  @override
  bool shouldRepaint(WaveformShaderPainter oldDelegate) =>
      !identical(texture, oldDelegate.texture) ||
      background != oldDelegate.background ||
      mode != oldDelegate.mode;
}

/// A recorded waveform picture plus the texture its shader samples.
///
/// The texture must outlive the picture: the recorded display list samples it
/// at raster time.
class WaveformPicture {
  new({required this.picture, this.texture});

  final Picture picture;
  final WaveformPeaksTexture? texture;

  void draw(Canvas canvas) => canvas.drawPicture(picture);

  void dispose() {
    picture.dispose();
    texture?.dispose();
  }
}

/// Records [peaks] across [size]. Empty peaks record the opaque lane fill only.
Future<WaveformPicture> recordWaveformPicture({
  required List<SpectralPeak> peaks,
  required Size size,
  required Color background,
  WaveformDisplayMode mode = WaveformDisplayMode.rgb,
}) async {
  final texture = peaks.isEmpty || size.width <= 0
      ? null
      : await encodeWaveformTexture(peaks);
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  WaveformShaderPainter(
    texture: texture,
    background: background,
    mode: mode,
  ).paint(canvas, size);
  return WaveformPicture(picture: recorder.endRecording(), texture: texture);
}
