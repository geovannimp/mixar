import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/waveform_spectral_color.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_visual_choice.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class SettingsWaveformPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SettingsSectionBody(
      title: l10n.settingsSectionWaveform,
      description: l10n.settingsWaveformDescription,
      cards: [
        SettingsSectionCard(
          title: l10n.settingsWaveformDisplayTitle,
          subtitle: l10n.settingsWaveformDisplayDescription,
          child: SettingsField(
            label: l10n.settingsWaveformDisplayMode,
            child: SettingsVisualChoice<WaveformDisplayModeSetting>(
              value: draft.waveformDisplayMode,
              options: [
                for (final value in WaveformDisplayModeSetting.values)
                  SettingsVisualOption<WaveformDisplayModeSetting>(
                    value: value,
                    label: switch (value) {
                      WaveformDisplayModeSetting.rgb =>
                        l10n.settingsWaveformModeRgb,
                      WaveformDisplayModeSetting.filtered =>
                        l10n.settingsWaveformModeFiltered,
                    },
                    preview: _WaveformDisplayModePreview(mode: value),
                  ),
              ],
              onChanged: (mode) =>
                  onChanged(copyAppSettings(draft, waveformDisplayMode: mode)),
            ),
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsWaveformShapingTitle,
          subtitle: l10n.settingsWaveformShapingDescription,
          trailing: SettingsHeaderSwitch(
            value: draft.waveformGainEqShaping,
            semanticsLabel: l10n.settingsWaveformShapingEnable,
            onChanged: (shaped) => onChanged(
              copyAppSettings(draft, waveformGainEqShaping: shaped),
            ),
          ),
        ),
      ],
    );
  }
}

/// The lane as the shader draws it: one mirrored column per logical pixel in
/// `RGB`, or three nested bands (low outside high) in `Filtered`.
class _WaveformDisplayModePreview extends StatelessWidget {
  const new({required this.mode});

  final WaveformDisplayModeSetting mode;

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      painter: _WaveformDisplayModePainter(
        mode: mode,
        background: context.theme.colors.waveformBackground,
      ),
    );
  }
}

class _WaveformDisplayModePainter extends CustomPainter {
  const new({required this.mode, required this.background});

  final WaveformDisplayModeSetting mode;
  final Color background;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.isEmpty) return;

    canvas.save();
    canvas.clipRRect(
      RRect.fromRectAndRadius(Offset.zero & size, const Radius.circular(2)),
    );
    canvas.drawRect(Offset.zero & size, Paint()..color = background);

    final bars = size.width.floor().clamp(1, 160);
    final barWidth = size.width / bars;
    final centre = size.height / 2;
    final maxAmp = size.height * 0.46;
    final filtered = mode == WaveformDisplayModeSetting.filtered;
    final paint = Paint();

    for (var i = 0; i < bars; i++) {
      final t = bars == 1 ? 0.0 : i / (bars - 1);
      final (low, mid, high) = _previewPeaks(t);
      final x = i * barWidth;

      if (filtered) {
        _band(canvas, paint, x, barWidth, centre, maxAmp, low, kLowColor);
        _band(canvas, paint, x, barWidth, centre, maxAmp, mid, kMidColor);
        _band(canvas, paint, x, barWidth, centre, maxAmp, high, kHighColor);
        continue;
      }

      final amplitude = math.max(low, math.max(mid, high));
      if (amplitude <= 0.01) continue;
      final total = low + mid + high + 1e-6;
      final alpha = (0.65 + amplitude * 0.35).clamp(0.0, 1.0);
      final color = Color.fromARGB(
        (alpha * 255).round(),
        ((low * kLowColor.r + mid * kMidColor.r + high * kHighColor.r) /
                total *
                255)
            .round(),
        ((low * kLowColor.g + mid * kMidColor.g + high * kHighColor.g) /
                total *
                255)
            .round(),
        ((low * kLowColor.b + mid * kMidColor.b + high * kHighColor.b) /
                total *
                255)
            .round(),
      );
      _band(canvas, paint, x, barWidth, centre, maxAmp, amplitude, color);
    }

    canvas.restore();
  }

  void _band(
    Canvas canvas,
    Paint paint,
    double x,
    double width,
    double centre,
    double maxAmp,
    double amplitude,
    Color color,
  ) {
    if (amplitude <= 0.01) return;
    final half = amplitude * maxAmp;
    paint.color = color;
    canvas.drawRect(
      Rect.fromLTRB(x, centre - half, x + width * 1.15, centre + half),
      paint,
    );
  }

  @override
  bool shouldRepaint(_WaveformDisplayModePainter oldDelegate) =>
      oldDelegate.mode != mode || oldDelegate.background != background;
}

/// Eight kick-like attacks; the bands peak at different points so `RGB`
/// cycles through blended columns and `Filtered` shows nested edges.
(double, double, double) _previewPeaks(double t) {
  final phase = (t * 8.0) % 1.0;
  final decay = math.pow(1 - phase, 1.3).toDouble();
  final swell = math.sin(phase * math.pi);
  final envelope = (0.12 + 0.88 * math.max(decay, swell)).clamp(0.0, 1.0);
  final sweep = t * math.pi * 5;

  double band(double offset) =>
      ((0.22 + 0.78 * (0.5 + 0.5 * math.sin(sweep + offset))) * envelope).clamp(
        0.0,
        1.0,
      );

  return (band(0), band(2.1), band(4.2));
}
