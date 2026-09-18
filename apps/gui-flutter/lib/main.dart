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
class Application extends StatelessWidget {
  const new({required this.appTitle, super.key});

  final String appTitle;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: appTitle,
      debugShowCheckedModeBanner: false,
      localizationsDelegates: GlobalMaterialLocalizations.delegates,
      theme: _materialLight,
      darkTheme: _materialDark,
      builder: (context, child) {
        final dark = Theme.brightnessOf(context) == Brightness.dark;
        final data = dark ? MixarThemeData.dark() : MixarThemeData.light();
        return LegacyMaterialScope(
          child: DesktopChrome(
            child: MixarTheme(
              data: data,
              child: ShadTheme(
                data: dark ? _shadDark : _shadLight,
                child: child!,
              ),
            ),
          ),
        );
      },
      home: AppShell(appTitle: appTitle),
    );
  }
}
