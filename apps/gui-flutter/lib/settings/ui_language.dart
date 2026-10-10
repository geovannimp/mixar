import 'package:flutter/widgets.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

/// Maps saved UI language preference to `MaterialApp.locale`.
///
/// `null` means follow the platform locale (system default).
Locale? localeFromUiLanguage(UiLanguageSetting value) => switch (value) {
  UiLanguageSetting.system => null,
  UiLanguageSetting.en => const Locale('en'),
  UiLanguageSetting.ptBr => const Locale('pt', 'BR'),
};
