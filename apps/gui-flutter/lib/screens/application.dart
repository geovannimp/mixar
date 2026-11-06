import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/core/theme/shad_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/ui_language.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/l10n/kit_localizations.dart';
import 'package:gui_flutter/providers/onboarding/onboarding_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/app_shell.dart';
import 'package:gui_flutter/screens/onboarding_page.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/shell/desktop_chrome.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Cached Material / Shad themes so [Application.build] and
/// MaterialApp.builder do not rebuild style graphs every frame.
final _materialThemes = <(MixarAccentPalette, Brightness), ThemeData>{};
final _shadThemes = <(MixarAccentPalette, Brightness), ShadThemeData>{};

ThemeData _materialTheme(MixarAccentPalette palette, Brightness brightness) =>
    _materialThemes.putIfAbsent(
      (palette, brightness),
      () => materialUiThemeFromMixar(
        MixarThemeData.of(palette, brightness),
        scaffoldBackgroundColor: Colors.transparent,
      ),
    );

ShadThemeData _shadTheme(MixarAccentPalette palette, Brightness brightness) =>
    _shadThemes.putIfAbsent((
      palette,
      brightness,
    ), () => shadThemeFromMixar(MixarThemeData.of(palette, brightness)));

/// Fallback while settings are still loading. Never mutated.
final AppSettings _fallbackSettings = defaultAppSettings();

/// Root app: Mixar theme tokens + Shad bridge + mixer shell.
class Application extends ConsumerWidget {
  const new({required this.appTitle, super.key});

  final String appTitle;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settingsAsync = ref.watch(appSettingsProvider);
    // A running first-run flow owns the theme, accent and locale: its draft
    // previews step choices before Finish writes them.
    final resolved =
        ref.watch(onboardingDraftProvider) ??
        settingsAsync.asData?.value ??
        _fallbackSettings;
    final locale = localeFromUiLanguage(resolved.uiLanguage);
    // `system` stays the default so an install without a saved theme keeps
    // following the OS light/dark preference.
    final themeMode = switch (resolved.theme) {
      ThemeSetting.system => ThemeMode.system,
      ThemeSetting.light => ThemeMode.light,
      ThemeSetting.dark => ThemeMode.dark,
    };
    // Accent palette is resolved once, then threaded through MaterialApp's
    // themes and the builder so both bridges stay on the same palette.
    final palette = accentPaletteFromSettings(resolved.accentPalette);

    return MaterialApp(
      title: appTitle,
      debugShowCheckedModeBanner: false,
      locale: locale,
      localizationsDelegates: mixarLocalizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: _materialTheme(palette, Brightness.light),
      darkTheme: _materialTheme(palette, Brightness.dark),
      themeMode: themeMode,
      builder: (context, child) {
        final brightness = Theme.brightnessOf(context) == Brightness.dark
            ? Brightness.dark
            : Brightness.light;
        // Theme first: the window chrome paints its frame from Mixar tokens.
        return MixarTheme(
          data: MixarThemeData.of(palette, brightness),
          child: DesktopChrome(
            child: ShadTheme(
              data: _shadTheme(palette, brightness),
              child: child!,
            ),
          ),
        );
      },
      // Only resolved settings may route to setup: the fallback keeps the
      // mixer, and `main.dart` preloads settings before `runApp` (falling back
      // here only on the preload timeout).
      home: settingsAsync.maybeWhen(
        data: (settings) => settings.onboardingCompleted
            ? AppShell(appTitle: appTitle)
            : OnboardingPage(appTitle: appTitle),
        orElse: () => AppShell(appTitle: appTitle),
      ),
    );
  }
}
