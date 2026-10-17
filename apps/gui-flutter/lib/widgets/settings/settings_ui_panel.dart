import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';

class SettingsUiPanel extends StatelessWidget {
  const SettingsUiPanel({
    required this.draft,
    required this.onChanged,
    super.key,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const List<UiLanguageSetting> _languages = UiLanguageSetting.values;

  static String _languageLabel(
    AppLocalizations l10n,
    UiLanguageSetting value,
  ) => switch (value) {
    UiLanguageSetting.system => l10n.settingsSystemDefault,
    UiLanguageSetting.en => l10n.settingsLanguageEn,
    UiLanguageSetting.ptBr => l10n.settingsLanguagePtBr,
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
                  labelBuilder: (value) => _languageLabel(l10n, value),
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
          title: l10n.settingsUiMixerMetersTitle,
          subtitle: l10n.settingsUiMixerMetersDescription,
          trailing: SettingsHeaderSwitch(
            value: draft.showMixerMeters,
            semanticsLabel: l10n.settingsUiShowMixerMeters,
            onChanged: (v) =>
                onChanged(copyAppSettings(draft, showMixerMeters: v)),
          ),
          child: draft.showMixerMeters
              ? SettingsField(
                  label: l10n.settingsUiMixerMetersMode,
                  child: SettingsSelect<bool>(
                    dialogTitle: l10n.settingsUiMixerMetersMode,
                    value: draft.mixerMetersMono,
                    options: const [true, false],
                    labelBuilder: (mono) => mono
                        ? l10n.settingsUiMixerMetersMono
                        : l10n.settingsUiMixerMetersStereo,
                    subtitleBuilder: (mono) => mono
                        ? l10n.settingsUiMixerMetersMonoSubtitle
                        : l10n.settingsUiMixerMetersStereoSubtitle,
                    onChanged: (mono) => onChanged(
                      copyAppSettings(draft, mixerMetersMono: mono),
                    ),
                  ),
                )
              : null,
        ),
        SettingsSectionCard(
          title: l10n.settingsUiLibraryListTitle,
          subtitle: l10n.settingsUiLibraryListDescription,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsCheckbox(
                label: l10n.settingsLibraryDimPlayedTracks,
                value: draft.dimPlayedTracks,
                onChanged: (enabled) =>
                    onChanged(copyAppSettings(draft, dimPlayedTracks: enabled)),
              ),
              SettingsField(
                label: l10n.settingsLibraryTrackRowLayout,
                child: SettingsSelect<LibraryRowDensitySetting>(
                  dialogTitle: l10n.settingsLibraryTrackRowLayout,
                  value: draft.libraryRowDensity,
                  options: const [
                    LibraryRowDensitySetting.comfortable,
                    LibraryRowDensitySetting.compact,
                  ],
                  labelBuilder: (mode) =>
                      mode == LibraryRowDensitySetting.compact
                      ? l10n.settingsLibraryRowCompact
                      : l10n.settingsLibraryRowComfortable,
                  subtitleBuilder: (mode) =>
                      mode == LibraryRowDensitySetting.compact
                      ? l10n.settingsLibraryRowCompactSubtitle
                      : l10n.settingsLibraryRowComfortableSubtitle,
                  onChanged: (mode) => onChanged(
                    copyAppSettings(draft, libraryRowDensity: mode),
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
