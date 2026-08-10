import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_shell.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  if (isDesktopWindow) {
    await windowManager.ensureInitialized();
    const options = WindowOptions(
      size: Size(1280, 800),
      minimumSize: Size(960, 640),
      center: true,
      backgroundColor: Colors.transparent,
      skipTaskbar: false,
      title: 'RUST DJ',
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

  await RustLib.init();
  runApp(const Application());
}

/// Root app: [Forui](https://forui.dev/) light/dark themes + mixer shell.
class Application extends StatelessWidget {
  const Application({super.key});

  @override
  Widget build(BuildContext context) {
    final (lightTheme, darkTheme) = _platformThemes();

    return MaterialApp(
      title: 'RUST DJ',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.system,
      supportedLocales: FLocalizations.supportedLocales,
      localizationsDelegates: FLocalizations.localizationsDelegates,
      theme: lightTheme.toApproximateMaterialTheme(),
      darkTheme: darkTheme.toApproximateMaterialTheme(),
      builder: (context, child) => FTheme(
        data: Theme.brightnessOf(context) == Brightness.dark
            ? darkTheme
            : lightTheme,
        child: FToaster(child: FTooltipGroup(child: child!)),
      ),
      home: const AppShell(),
    );
  }
}

(FThemeData, FThemeData) _platformThemes() {
  final mobile = const {
    TargetPlatform.android,
    TargetPlatform.iOS,
    TargetPlatform.fuchsia,
  }.contains(defaultTargetPlatform);

  // Default Forui neutral themes — https://forui.dev/docs/getting-started
  if (mobile) {
    return (FTheme.neutral.light.touch, FTheme.neutral.dark.touch);
  }
  return (FTheme.neutral.light.desktop, FTheme.neutral.dark.desktop);
}
