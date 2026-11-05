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
