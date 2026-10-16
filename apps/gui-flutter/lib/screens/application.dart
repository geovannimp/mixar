import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/theme/shad_theme.dart';
import 'package:gui_flutter/core/utils/ui_language.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/l10n/kit_localizations.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/app_shell.dart';
import 'package:gui_flutter/widgets/shell/desktop_chrome.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Cached Material / Shad themes so [Application.build] and
/// MaterialApp.builder do not rebuild style graphs every frame.
final ThemeData _materialLight = materialUiThemeFromMixar(
  MixarThemeData.light(),
  scaffoldBackgroundColor: Colors.transparent,
);
final ThemeData _materialDark = materialUiThemeFromMixar(
  MixarThemeData.dark(),
  scaffoldBackgroundColor: Colors.transparent,
);
final ShadThemeData _shadLight = shadThemeFromMixar(MixarThemeData.light());
final ShadThemeData _shadDark = shadThemeFromMixar(MixarThemeData.dark());

/// Root app: Mixar theme tokens + Shad bridge + mixer shell.
class Application extends ConsumerWidget {
  const new({required this.appTitle, super.key});

  final String appTitle;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final locale = ref
        .watch(appSettingsProvider)
        .maybeWhen(
          data: (settings) => localeFromUiLanguage(settings.uiLanguage),
          orElse: () => null,
        );

    return MaterialApp(
      title: appTitle,
      debugShowCheckedModeBanner: false,
      locale: locale,
      localizationsDelegates: mixarLocalizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: _materialLight,
      darkTheme: _materialDark,
      builder: (context, child) {
        final dark = Theme.brightnessOf(context) == Brightness.dark;
        final data = dark ? MixarThemeData.dark() : MixarThemeData.light();
        return DesktopChrome(
          child: MixarTheme(
            data: data,
            child: ShadTheme(
              data: dark ? _shadDark : _shadLight,
              child: child!,
            ),
          ),
        );
      },
      home: AppShell(appTitle: appTitle),
    );
  }
}
