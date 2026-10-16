import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/screens/application.dart';
import 'package:gui_flutter/src/rust/api/meta.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';
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
