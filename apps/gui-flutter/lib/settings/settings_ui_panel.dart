import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsUiPanel extends StatelessWidget {
  const SettingsUiPanel({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const List<UiLanguageSetting> _languages = UiLanguageSetting.values;

  static String _languageLabel(
    AppLocalizations l10n,
    UiLanguageSetting value,
  ) => switch (value) {
    UiLanguageSetting.system => l10n.settingsLanguageSystemDefault,
    UiLanguageSetting.en => 'English',
    UiLanguageSetting.ptBr => 'Português (Brasil)',
  };

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
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        SettingsSectionHeader(
          title: l10n.settingsSectionUi,
          description: l10n.settingsUiDescription,
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsField(
                label: l10n.settingsUiLanguageLabel,
                child: SettingsSelect<UiLanguageSetting>(
                  value: draft.uiLanguage,
                  options: _languages,
                  labelBuilder: (value) => _languageLabel(l10n, value),
                  onChanged: (language) => onChanged(
                    copyAppSettings(draft, uiLanguage: language),
                  ),
                ),
              ),
              SettingsToggle(
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
      ],
    );
  }
}
