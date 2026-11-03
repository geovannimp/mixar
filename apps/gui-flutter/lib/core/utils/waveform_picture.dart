import 'dart:ui';

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/waveform_gain.dart';
import 'package:gui_flutter/core/utils/waveform_shader.dart';
import 'package:gui_flutter/core/utils/waveform_spectral_color.dart';

/// Paints one waveform quad from a cached peak texture.
///
/// The texture is the only per-track cache: gains, display mode and lane fill
/// are uniforms, so a deck's EQ can shape its bars without re-recording
/// anything.
class WaveformShaderPainter extends CustomPainter {
  new({
    required this.texture,
    required this.background,
    this.mode = WaveformDisplayMode.rgb,
    this.gain = 1,
    this.bandGain = flatWaveformBandGain,
  });

  /// Bucket texture for exactly the pixels this painter covers.
  final WaveformPeaksTexture? texture;
  final Color background;
  final WaveformDisplayMode mode;

  /// Channel gain, 1.0 = unity.
  final double gain;

  /// Per-band EQ gain, 1.0 = flat.
  final WaveformBandGain bandGain;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = background);
    final shader = WaveformShader.shader;
    final peaks = texture;
    // A reload keeps the previous value visible until the new one lands, so a
    // painter can be handed a texture whose scheduled disposal already ran.
    // Sampling it would assert in debug and read a freed image in release.
    if (shader == null || peaks == null || peaks.disposed || peaks.count < 1) {
      return;
    }
    if (size.width <= 0 || size.height <= 0) {
      return;
    }
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
    shader.getUniformFloat('uGain').set(gain);
    shader
        .getUniformVec3('uBandGain')
        .set(bandGain.$1, bandGain.$2, bandGain.$3);
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
      mode != oldDelegate.mode ||
      gain != oldDelegate.gain ||
      bandGain != oldDelegate.bandGain;
}
