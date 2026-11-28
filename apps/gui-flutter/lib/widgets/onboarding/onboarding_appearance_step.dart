import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:gui_flutter/widgets/settings/settings_accent_picker.dart';
import 'package:gui_flutter/widgets/settings/settings_theme_picker.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Step 3: theme and accent, chosen on live previews.
///
/// Every tap repaints the whole window: `Application` reads the draft, so the
/// preview is the real app, not a mock. Both pickers are the same segmented
/// controls Settings uses.
class OnboardingAppearanceStep extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OnboardingBody(
      eyebrow: l10n.onboardingKickerAppearance,
      icon: LucideIcons.palette,
      title: l10n.onboardingAppearanceTitle,
      description: l10n.onboardingAppearanceBody,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        spacing: 28,
        children: [
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 10,
            children: [
              OnboardingGroupLabel(label: l10n.settingsUiTheme),
              SettingsThemePicker(
                value: draft.theme,
                onChanged: (theme) =>
                    onChanged(copyAppSettings(draft, theme: theme)),
              ),
            ],
          ),
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 10,
            children: [
              OnboardingGroupLabel(label: l10n.settingsUiAccentColor),
              SettingsAccentPicker(
                value: draft.accentPalette,
                onChanged: (accent) =>
                    onChanged(copyAppSettings(draft, accentPalette: accent)),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
