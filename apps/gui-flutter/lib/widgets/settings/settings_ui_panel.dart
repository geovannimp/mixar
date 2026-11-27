import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_accent_picker.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_labels.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/settings/settings_theme_picker.dart';

class SettingsUiPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const List<UiLanguageSetting> _languages = UiLanguageSetting.values;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SettingsSectionBody(
      cards: [
        SettingsSectionCard(
          title: l10n.settingsUiChromeTitle,
          subtitle: l10n.settingsUiChromeDescription,
          searchTerms: [
            l10n.settingsUiLanguageLabel,
            l10n.settingsShowTooltips,
          ],
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsField(
                label: l10n.settingsUiLanguageLabel,
                child: SettingsSelect<UiLanguageSetting>(
                  value: draft.uiLanguage,
                  options: _languages,
                  labelBuilder: (value) => uiLanguageSettingLabel(l10n, value),
                  onChanged: (language) =>
                      onChanged(copyAppSettings(draft, uiLanguage: language)),
                ),
              ),
              SettingsCheckbox(
                label: l10n.settingsShowTooltips,
                value: draft.showTooltips,
                onChanged: (enabled) =>
                    onChanged(copyAppSettings(draft, showTooltips: enabled)),
              ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsUiThemeTitle,
          subtitle: l10n.settingsUiThemeDescription,
          child: SettingsField(
            label: l10n.settingsUiTheme,
            child: SettingsThemePicker(
              value: draft.theme,
              onChanged: (theme) =>
                  onChanged(copyAppSettings(draft, theme: theme)),
            ),
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsUiAccentColor,
          subtitle: l10n.settingsUiAccentDescription,
          child: SettingsAccentPicker(
            value: draft.accentPalette,
            onChanged: (palette) =>
                onChanged(copyAppSettings(draft, accentPalette: palette)),
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsUiOnboardingTitle,
          subtitle: l10n.settingsUiOnboardingDescription,
          // The docs and the switch's own name both say "Show onboarding next
          // run", while the card reads "First-run setup": index the former so
          // a search for the name the user was told finds the card.
          searchTerms: [l10n.settingsUiOnboardingNextRun],
          // Setup is a launch decision: the switch only records that the next
          // launch should open it. `Application` reads the flag once, so
          // saving here never swaps the window away from the mixer.
          trailing: SettingsHeaderSwitch(
            value: !draft.onboardingCompleted,
            semanticsLabel: l10n.settingsUiOnboardingNextRun,
            onChanged: (scheduled) => onChanged(
              copyAppSettings(draft, onboardingCompleted: !scheduled),
            ),
          ),
        ),
      ],
    );
  }
}
