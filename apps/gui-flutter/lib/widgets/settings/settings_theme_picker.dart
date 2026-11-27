import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_labels.dart';
import 'package:gui_flutter/widgets/settings/settings_segmented_choice.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Theme chooser — the same segmented pill as the accent picker.
///
/// Each segment carries a glyph for its brightness (monitor for the system
/// default, sun for light, moon for dark); the selected one takes the theme's
/// selection tint.
class SettingsThemePicker extends StatelessWidget {
  const new({required this.value, required this.onChanged, super.key});

  final ThemeSetting value;
  final ValueChanged<ThemeSetting> onChanged;

  static const _options = <ThemeSetting>[
    ThemeSetting.system,
    ThemeSetting.light,
    ThemeSetting.dark,
  ];

  static IconData _icon(ThemeSetting theme) => switch (theme) {
    ThemeSetting.system => LucideIcons.monitor,
    ThemeSetting.light => LucideIcons.sun,
    ThemeSetting.dark => LucideIcons.moon,
  };

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SettingsSegmentedChoice<ThemeSetting>(
      value: value,
      options: [
        for (final option in _options)
          SettingsSegmentedOption<ThemeSetting>(
            value: option,
            label: themeSettingLabel(l10n, option),
            leading: Icon(_icon(option)),
          ),
      ],
      onChanged: onChanged,
    );
  }
}
