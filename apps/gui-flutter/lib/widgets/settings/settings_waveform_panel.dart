import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/waveform_spectral_color.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_preview_block.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_visual_choice.dart';

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
      ],
    );
  }
}

/// The same waveform silhouette twice: a spectral gradient for `RGB`, and
/// red outside blue outside green for `Filtered`.
class _WaveformDisplayModePreview extends StatelessWidget {
  const new({required this.mode});

  final WaveformDisplayModeSetting mode;

  /// Peak height as a fraction of the 24px preview, one bar per entry.
  static const _envelope = <double>[
    0.30,
    0.55,
    0.85,
    1.00,
    0.72,
    0.95,
    0.80,
    0.50,
    0.88,
    0.62,
    0.34,
  ];

  @override
  Widget build(BuildContext context) {
    final filtered = mode == WaveformDisplayModeSetting.filtered;
    final bars = SizedBox(
      height: 24,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        spacing: 1,
        children: [
          for (final peak in _envelope)
            SizedBox(
              width: 4,
              height: 24 * peak,
              child: ClipRRect(
                borderRadius: BorderRadius.circular(1.5),
                child: filtered
                    ? const _FilteredBar()
                    : const SettingsPreviewBlock(color: Color(0xFFFFFFFF)),
              ),
            ),
        ],
      ),
    );
    if (filtered) {
      return Center(child: bars);
    }
    return Center(
      child: ShaderMask(
        blendMode: BlendMode.srcIn,
        shaderCallback: (bounds) => const LinearGradient(
          begin: Alignment.centerLeft,
          end: Alignment.centerRight,
          colors: [kLowColor, kMidColor, kHighColor, kMidColor, kLowColor],
          stops: [0, 0.25, 0.5, 0.75, 1],
        ).createShader(bounds),
        child: bars,
      ),
    );
  }
}

/// One Filtered bar: green nested inside blue nested inside red.
class _FilteredBar extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) {
    return const Stack(
      fit: StackFit.expand,
      children: [
        SettingsPreviewBlock(color: kLowColor),
        Align(
          alignment: Alignment.center,
          child: FractionallySizedBox(
            widthFactor: 1,
            heightFactor: 0.66,
            child: SettingsPreviewBlock(color: kHighColor),
          ),
        ),
        Align(
          alignment: Alignment.center,
          child: FractionallySizedBox(
            widthFactor: 1,
            heightFactor: 0.33,
            child: SettingsPreviewBlock(color: kMidColor),
          ),
        ),
      ],
    );
  }
}
