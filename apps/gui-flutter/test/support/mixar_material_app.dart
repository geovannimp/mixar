import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/l10n/kit_localizations.dart';
import 'package:gui_flutter/shell/legacy_material_scope.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/shad_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

ThemeData mixarMaterialTheme(MixarThemeData theme) =>
    materialUiThemeFromMixar(theme);

/// [MaterialApp.builder] with Mixar + Shad themes + [LegacyMaterialScope].
TransitionBuilder mixarMaterialAppBuilder(
  MixarThemeData theme, {
  Widget Function(Widget child)? wrapChild,
}) {
  return (context, child) {
    final content = wrapChild != null ? wrapChild(child!) : child!;
    return LegacyMaterialScope(
      child: MixarTheme(
        data: theme,
        child: ShadTheme(data: shadThemeFromMixar(theme), child: content),
      ),
    );
  };
}

/// Common [MaterialApp] localization wiring for widget tests.
List<LocalizationsDelegate<dynamic>> get mixarTestLocalizationsDelegates =>
    mixarLocalizationsDelegates;

List<Locale> get mixarTestSupportedLocales => AppLocalizations.supportedLocales;

/// Shared [MaterialApp] for widget tests.
///
/// Defaults to English so assertions on copy stay deterministic; pass [locale]
/// when a test needs another supported locale (or omit via null to follow the
/// platform — not recommended for most widget tests).
Widget mixarTestMaterialApp({
  required MixarThemeData theme,
  required Widget home,
  Locale? locale = const Locale('en'),
  Widget Function(Widget child)? wrapChild,
}) {
  return MaterialApp(
    locale: locale,
    localizationsDelegates: mixarTestLocalizationsDelegates,
    supportedLocales: mixarTestSupportedLocales,
    theme: materialUiThemeFromMixar(theme),
    builder: mixarMaterialAppBuilder(theme, wrapChild: wrapChild),
    home: home,
  );
}
