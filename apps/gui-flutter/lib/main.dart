import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/application.dart';
import 'package:gui_flutter/src/rust/api/meta.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  if (kIsWeb) {
    await BrowserContextMenu.disableContextMenu();
  }
  // RustLib.init() resolves app data in Rust and attaches `mixar.log` there,
  // before any of the providers below open a transport.
  await RustLib.init();

  final appTitle = appDisplayName();

  // Resolve saved settings before the window and the first frame: MaterialApp
  // boots on defaults otherwise and repaints once the bridge answers, which
  // reads as a dark flash before a saved light theme.
  final container = ProviderContainer();
  await preloadAppSettings(container);

  if (isDesktopWindow) {
    await windowManager.ensureInitialized();
    final options = WindowOptions(
      size: const Size(1280, 800),
      minimumSize: const Size(1024, 768),
      center: true,
      // Transparent so the Linux rounded corners reveal the compositor, not
      // opaque black (other hosts draw a square window).
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

  runApp(
    UncontrolledProviderScope(
      container: container,
      child: Application(appTitle: appTitle),
    ),
  );
}
