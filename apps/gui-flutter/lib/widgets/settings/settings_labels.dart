import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

/// Shared option labels for the theme and language pickers.
///
/// Settings → UI and the first-run setup both offer these options; keeping the
/// wording here is what stops the two surfaces drifting apart.
String themeSettingLabel(AppLocalizations l10n, ThemeSetting value) =>
    switch (value) {
      ThemeSetting.system => l10n.settingsSystemDefault,
      ThemeSetting.light => l10n.settingsUiThemeLight,
      ThemeSetting.dark => l10n.settingsUiThemeDark,
    };

String uiLanguageSettingLabel(AppLocalizations l10n, UiLanguageSetting value) =>
    switch (value) {
      UiLanguageSetting.system => l10n.settingsSystemDefault,
      UiLanguageSetting.en => l10n.settingsLanguageEn,
      UiLanguageSetting.ptBr => l10n.settingsLanguagePtBr,
    };

String accentPaletteLabel(AppLocalizations l10n, AccentPaletteSetting value) =>
    switch (value) {
      AccentPaletteSetting.teal => l10n.settingsUiAccentTeal,
      AccentPaletteSetting.blue => l10n.settingsUiAccentBlue,
      AccentPaletteSetting.violet => l10n.settingsUiAccentViolet,
      AccentPaletteSetting.orange => l10n.settingsUiAccentOrange,
      AccentPaletteSetting.pink => l10n.settingsUiAccentPink,
    };

String selectStyleSettingLabel(
  AppLocalizations l10n,
  SelectStyleSetting value,
) => switch (value) {
  SelectStyleSetting.auto => l10n.settingsUiSelectStyleAuto,
  SelectStyleSetting.desktop => l10n.settingsUiSelectStyleDesktop,
  SelectStyleSetting.mobile => l10n.settingsUiSelectStyleMobile,
};

String selectStyleSettingSubtitle(
  AppLocalizations l10n,
  SelectStyleSetting value,
) => switch (value) {
  SelectStyleSetting.auto => l10n.settingsUiSelectStyleAutoSubtitle,
  SelectStyleSetting.desktop => l10n.settingsUiSelectStyleDesktopSubtitle,
  SelectStyleSetting.mobile => l10n.settingsUiSelectStyleMobileSubtitle,
};
