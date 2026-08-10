import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_shell.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/api/meta.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();

  final appTitle = appDisplayName();

  if (isDesktopWindow) {
    await windowManager.ensureInitialized();
    final options = WindowOptions(
      size: const Size(1280, 800),
      minimumSize: const Size(960, 640),
      center: true,
      backgroundColor: Colors.transparent,
      skipTaskbar: false,
      title: appTitle,
      // Hide OS title bar; in-app header provides drag + controls.
      // https://pub.dev/packages/window_manager
      titleBarStyle: TitleBarStyle.hidden,
    );
    await windowManager.waitUntilReadyToShow(options, () async {
      await windowManager.setTitleBarStyle(TitleBarStyle.hidden);
      await windowManager.show();
      await windowManager.focus();
    });
  }

  runApp(Application(appTitle: appTitle));
}

/// Root app: [Forui](https://forui.dev/) light/dark themes + mixer shell.
class Application extends StatelessWidget {
  const Application({required this.appTitle, super.key});

  final String appTitle;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: appTitle,
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.system,
      supportedLocales: FLocalizations.supportedLocales,
      localizationsDelegates: FLocalizations.localizationsDelegates,
      theme: FTheme.neutral.light.desktop.toApproximateMaterialTheme(),
      darkTheme: FTheme.neutral.dark.desktop.toApproximateMaterialTheme(),
      builder: (context, child) {
        final platforms = Theme.brightnessOf(context) == Brightness.dark
            ? FTheme.neutral.dark
            : FTheme.neutral.light;
        // Resolve touch vs desktop via Forui's platformVariant:
        // https://forui.dev/docs/concepts/responsive
        return FAdaptiveScope(
          child: Builder(
            builder: (context) {
              final data = context.platformVariant.touch
                  ? platforms.touch
                  : platforms.desktop;
              return FTheme(
                data: data,
                child: FToaster(child: FTooltipGroup(child: child!)),
              );
            },
          ),
        );
      },
      home: AppShell(appTitle: appTitle),
    );
  }
}
