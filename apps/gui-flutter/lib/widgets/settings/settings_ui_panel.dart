import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_accent_picker.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_labels.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';

class SettingsUiPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const List<UiLanguageSetting> _languages = UiLanguageSetting.values;

  static String _selectStyleLabel(
    AppLocalizations l10n,
    SelectStyleSetting value,
  ) => switch (value) {
    SelectStyleSetting.auto => l10n.settingsUiSelectStyleAuto,
    SelectStyleSetting.desktop => l10n.settingsUiSelectStyleDesktop,
    SelectStyleSetting.mobile => l10n.settingsUiSelectStyleMobile,
  };

  static String _selectStyleSubtitle(
    AppLocalizations l10n,
    SelectStyleSetting value,
  ) => switch (value) {
    SelectStyleSetting.auto => l10n.settingsUiSelectStyleAutoSubtitle,
    SelectStyleSetting.desktop => l10n.settingsUiSelectStyleDesktopSubtitle,
    SelectStyleSetting.mobile => l10n.settingsUiSelectStyleMobileSubtitle,
  };

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SettingsSectionBody(
      title: l10n.settingsSectionUi,
      description: l10n.settingsUiDescription,
      cards: [
        SettingsSectionCard(
          title: l10n.settingsUiThemeTitle,
          subtitle: l10n.settingsUiThemeDescription,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsField(
                label: l10n.settingsUiTheme,
                child: SettingsSelect<ThemeSetting>(
                  dialogTitle: l10n.settingsUiTheme,
                  value: draft.theme,
                  options: ThemeSetting.values,
                  labelBuilder: (theme) => themeSettingLabel(l10n, theme),
                  onChanged: (theme) =>
                      onChanged(copyAppSettings(draft, theme: theme)),
                ),
              ),
              SettingsField(
                label: l10n.settingsUiAccentColor,
                child: SettingsAccentPicker(
                  value: draft.accentPalette,
                  onChanged: (palette) =>
                      onChanged(copyAppSettings(draft, accentPalette: palette)),
                ),
              ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsUiChromeTitle,
          subtitle: l10n.settingsUiChromeDescription,
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
              SettingsField(
                label: l10n.settingsUiSelectStyle,
                child: SettingsSelect<SelectStyleSetting>(
                  dialogTitle: l10n.settingsUiSelectStyle,
                  value: draft.selectStyle,
                  options: const [
                    SelectStyleSetting.auto,
                    SelectStyleSetting.desktop,
                    SelectStyleSetting.mobile,
                  ],
                  labelBuilder: (mode) => _selectStyleLabel(l10n, mode),
                  subtitleBuilder: (mode) => _selectStyleSubtitle(l10n, mode),
                  onChanged: (mode) =>
                      onChanged(copyAppSettings(draft, selectStyle: mode)),
                ),
              ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsUiOnboardingTitle,
          subtitle: l10n.settingsUiOnboardingDescription,
          trailing: AppButton(
            size: MixarButtonSize.sm,
            variant: MixarButtonVariant.outline,
            onPress: () =>
                onChanged(copyAppSettings(draft, onboardingCompleted: false)),
            child: Text(l10n.settingsUiOnboardingReplay),
          ),
        ),
      ],
    );
  }
}
