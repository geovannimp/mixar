import 'package:flutter/foundation.dart'
    show FlutterError, FlutterErrorDetails, kIsWeb;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/screens/application.dart';
import 'package:gui_flutter/src/rust/api/meta.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  if (kIsWeb) {
    await BrowserContextMenu.disableContextMenu();
  }
  await RustLib.init();
  await _attachAppLog();

  final appTitle = appDisplayName();

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

  runApp(ProviderScope(child: Application(appTitle: appTitle)));
}

/// Attach `mixar.log` next to `library.db`/`settings.json` before anything else
/// runs, so diagnostics exist for the whole session and not only once MIDI
/// starts.
///
/// The path comes from Flutter because the app-support directory is a host
/// concept: every app-data path here is resolved by `path_provider` and handed
/// to Rust (`LibraryTransport.open(dbPath:)`, `SettingsTransport.open(path:)`,
/// `AnalyticsTransport.open(appSupportPath:)`). Rust has no data-dir resolver,
/// and one would disagree with `path_provider` on Windows (`%APPDATA%\Mixar`,
/// not `%APPDATA%\mixar\app`), putting the log in a directory with neither the
/// DB nor settings. Attaching here rather than from the library transport keeps
/// the log alive when the DB itself fails to open.
///
/// Diagnostics only: an unwritable app-support directory must never stop launch.
Future<void> _attachAppLog() async {
  try {
    final support = await getApplicationSupportDirectory();
    await attachAppLogFile(path: p.join(support.path, 'mixar.log'));
  } catch (error, stack) {
    FlutterError.reportError(
      FlutterErrorDetails(exception: error, stack: stack),
    );
  }
}
