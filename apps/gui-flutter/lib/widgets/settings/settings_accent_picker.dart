import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_labels.dart';
import 'package:gui_flutter/widgets/settings/settings_segmented_choice.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Accent palette chooser — one segmented pill, like the deck selector.
///
/// Each segment paints its own tone: the dot is the palette's `s500` step and
/// the label takes [textColorStep], so the hue reads the same in light and
/// dark. Selection outlines the segment in that same tone.
class SettingsAccentPicker extends StatelessWidget {
  const new({required this.value, required this.onChanged, super.key});

  final AccentPaletteSetting value;
  final ValueChanged<AccentPaletteSetting> onChanged;

  static const _options = <AccentPaletteSetting>[
    AccentPaletteSetting.teal,
    AccentPaletteSetting.blue,
    AccentPaletteSetting.violet,
    AccentPaletteSetting.orange,
    AccentPaletteSetting.pink,
  ];

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final brightness = context.theme.colors.brightness;
    Color ink(AccentPaletteSetting palette) =>
        textColorStep(accentPaletteFromSettings(palette).scale, brightness);
    return SettingsSegmentedChoice<AccentPaletteSetting>(
      value: value,
      options: [
        for (final palette in _options)
          SettingsSegmentedOption<AccentPaletteSetting>(
            value: palette,
            label: accentPaletteLabel(l10n, palette),
            labelColor: ink(palette),
            selectedBorder: ink(palette),
            leading: _AccentDot(palette: palette),
          ),
      ],
      onChanged: onChanged,
    );
  }
}

/// The palette's own mid tone, matching the accent marks elsewhere.
class _AccentDot extends StatelessWidget {
  const new({required this.palette});

  final AccentPaletteSetting palette;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return SizedBox.square(
      dimension: 10,
      child: DecoratedBox(
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: accentPaletteFromSettings(palette).scale.s500,
          border: Border.all(
            color: theme.colors.border,
            width: theme.style.borderWidth,
          ),
        ),
      ),
    );
  }
}
