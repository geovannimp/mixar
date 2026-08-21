import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_shell.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/shell/desktop_chrome.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/src/rust/api/meta.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';
import 'package:material_ui/material_ui.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await BrowserContextMenu.disableContextMenu();
  await RustLib.init();

  final appTitle = appDisplayName();

  if (isDesktopWindow) {
    await windowManager.ensureInitialized();
    final options = WindowOptions(
      size: const Size(1280, 800),
      minimumSize: const Size(1024, 768),
      center: true,
      // Transparent so ClipRRect corners reveal the compositor, not opaque black.
      backgroundColor: Colors.transparent,
      skipTaskbar: false,
      title: appTitle,
      // Hide OS title bar; in-app header provides drag + controls.
      // https://pub.dev/packages/window_manager
      titleBarStyle: TitleBarStyle.hidden,
    );
    await windowManager.waitUntilReadyToShow(options, () async {
      await windowManager.setTitleBarStyle(TitleBarStyle.hidden);
      await windowManager.setBackgroundColor(Colors.transparent);
      await windowManager.show();
      await windowManager.focus();
    });
  }

  runApp(ProviderScope(child: Application(appTitle: appTitle)));
}

/// Root app: [Forui](https://forui.dev/) light/dark themes + mixer shell.
class Application extends StatelessWidget {
  const Application({required this.appTitle, super.key});

  final String appTitle;

  @override
  Widget build(BuildContext context) {
    // Transparent Material canvas so desktop rounded corners aren't filled square.
    final light = materialUiThemeFromForui(
      FTheme.neutral.light.desktop,
      scaffoldBackgroundColor: Colors.transparent,
    );
    final dark = materialUiThemeFromForui(
      FTheme.neutral.dark.desktop,
      scaffoldBackgroundColor: Colors.transparent,
    );

    return MaterialApp(
      title: appTitle,
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.system,
      supportedLocales: FLocalizations.supportedLocales,
      // Forui ships SDK flutter_localizations delegates; material_ui needs its own.
      localizationsDelegates: [
        FLocalizations.delegate,
        ...GlobalMaterialLocalizations.delegates,
      ],
      theme: light,
      darkTheme: dark,
      builder: (context, child) {
        final platforms = Theme.brightnessOf(context) == Brightness.dark
            ? FTheme.neutral.dark
            : FTheme.neutral.light;
        // Resolve touch vs desktop via Forui's platformVariant:
        // https://forui.dev/docs/concepts/responsive
        // Bridge legacy flutter/material Theme for Forui / trina_grid / etc.
        return MaterialUiCompatibilityBridge(
          // ignore: deprecated_member_use
          child: FAdaptiveScope(
            child: Builder(
              builder: (context) {
                final data = context.platformVariant.touch
                    ? platforms.touch
                    : platforms.desktop;
                return DesktopChrome(
                  child: FTheme(
                    data: data,
                    child: FToaster(child: FTooltipGroup(child: child!)),
                  ),
                );
              },
            ),
          ),
        );
      },
      home: AppShell(appTitle: appTitle),
    );
  }
}
