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
import 'package:gui_flutter/widgets/shell/desktop_resize_border.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Cached Material / Shad themes so [_ApplicationState.build] and
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
class Application extends ConsumerStatefulWidget {
  const new({required this.appTitle, super.key});

  final String appTitle;

  @override
  ConsumerState<Application> createState() => _ApplicationState();
}

class _ApplicationState extends ConsumerState<Application> {
  /// Set once this launch has seen setup marked done.
  ///
  /// Setup is a launch decision: Settings schedules it for the next run by
  /// saving `onboardingCompleted: false`, so `home` reading that flag
  /// reactively would drop the window into setup the moment the switch is
  /// saved. Latching `true` keeps the mixer for the rest of the launch, while
  /// still letting the setup flow hand over to it when it finishes.
  late bool _setupDone;

  /// Whether [settings] report setup as done; unresolved reads as not done.
  static bool _setupSeen(AsyncValue<AppSettings> settings) =>
      settings.asData?.value.onboardingCompleted ?? false;

  @override
  void initState() {
    super.initState();
    // Resolved by the preload in `main()`; a preload timeout leaves this
    // `false` and the listener below latches once the read lands.
    _setupDone = _setupSeen(ref.read(appSettingsProvider));
    // The latch only moves one way, so a replay scheduled from Settings cannot
    // reopen setup after the flow handed over to the mixer.
    ref.listenManual(appSettingsProvider, (_, next) {
      if (!_setupDone && _setupSeen(next)) {
        setState(() => _setupDone = true);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final settingsAsync = ref.watch(appSettingsProvider);
    final settings = settingsAsync.asData?.value;
    // A running first-run flow owns the theme, accent and locale: its draft
    // previews step choices before Finish writes them.
    final resolved =
        ref.watch(onboardingDraftProvider) ?? settings ?? _fallbackSettings;
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
      title: widget.appTitle,
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
          child: DesktopResizeBorder(
            child: DesktopChrome(
              child: ShadTheme(
                data: _shadTheme(palette, brightness),
                child: child!,
              ),
            ),
          ),
        );
      },
      // Only resolved settings may route to setup: the fallback keeps the
      // mixer, and `main.dart` preloads settings before `runApp` (falling back
      // here only on the preload timeout).
      home: settings != null && !_setupDone
          ? OnboardingPage(appTitle: widget.appTitle)
          : AppShell(appTitle: widget.appTitle),
    );
  }
}
