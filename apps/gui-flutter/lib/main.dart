import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/shell/app_shell.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/shell/desktop_chrome.dart';
import 'package:gui_flutter/shell/legacy_material_scope.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/shad_theme.dart';
import 'package:gui_flutter/src/rust/api/meta.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  if (kIsWeb) {
    await BrowserContextMenu.disableContextMenu();
  }
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

/// Root app: Mixar theme tokens + Shad bridge + mixer shell.
class Application extends StatelessWidget {
  const new({required this.appTitle, super.key});

  final String appTitle;

  @override
  Widget build(BuildContext context) {
    final lightTokens = MixarThemeData.light();
    final darkTokens = MixarThemeData.dark();
    // Transparent Material canvas so desktop rounded corners aren't filled square.
    final light = materialUiThemeFromMixar(
      lightTokens,
      scaffoldBackgroundColor: Colors.transparent,
    );
    final dark = materialUiThemeFromMixar(
      darkTokens,
      scaffoldBackgroundColor: Colors.transparent,
    );

    return MaterialApp(
      title: appTitle,
      debugShowCheckedModeBanner: false,
      localizationsDelegates: GlobalMaterialLocalizations.delegates,
      theme: light,
      darkTheme: dark,
      builder: (context, child) {
        final data = MixarThemeData.forBrightness(Theme.brightnessOf(context));
        return LegacyMaterialScope(
          child: DesktopChrome(
            child: MixarTheme(
              data: data,
              child: ShadTheme(data: shadThemeFromMixar(data), child: child!),
            ),
          ),
        );
      },
      home: AppShell(appTitle: appTitle),
    );
  }
}
