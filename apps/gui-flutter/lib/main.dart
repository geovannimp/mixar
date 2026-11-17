import 'package:flutter/foundation.dart'
    show TargetPlatform, defaultTargetPlatform, kIsWeb;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/analytics/analytics_providers.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/waveform_shader.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/application.dart';
import 'package:gui_flutter/src/rust/api/meta.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';
import 'package:path_provider/path_provider.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  if (kIsWeb) {
    await BrowserContextMenu.disableContextMenu();
  }
  // RustLib.init() resolves app data in Rust and attaches `mixar.log` there,
  // before any of the providers below open a transport.
  await RustLib.init();
  // Waveform lanes are fragment-shader only, so a shader that will not load
  // leaves them blank for the whole session. Fail here, with the reason on
  // screen, instead of starting a UI whose lanes can never draw bars.
  try {
    await WaveformShader.load();
  } on Object catch (error) {
    runApp(ProviderScope(child: StartupFailureApp(message: '$error')));
    return;
  }

  final isAndroid = !kIsWeb && defaultTargetPlatform == TargetPlatform.android;
  if (isAndroid) {
    // Android app processes carry no HOME/XDG_DATA_HOME, so Rust cannot derive
    // the per-app files dir; without this every write targets `/` (EROFS).
    // A failure here is fatal for the library, so surface the cause instead of
    // starting a shell whose every write fails.
    try {
      final supportDir = await getApplicationSupportDirectory();
      await setAppSupportDir(path: supportDir.path);
    } on Object catch (error) {
      runApp(ProviderScope(child: StartupFailureApp(message: '$error')));
      return;
    }
    // The deck UI is a two-deck horizontal layout: stay landscape and give it
    // the whole screen (immersive hides the status and navigation bars).
    await SystemChrome.setPreferredOrientations(const [
      DeviceOrientation.landscapeLeft,
      DeviceOrientation.landscapeRight,
    ]);
    await SystemChrome.setEnabledSystemUIMode(SystemUiMode.immersiveSticky);
  }

  final appTitle = appDisplayName();

  // Resolve saved settings before the window and the first frame: MaterialApp
  // boots on defaults otherwise and repaints once the bridge answers, which
  // reads as a dark flash before a saved light theme.
  final container = ProviderContainer();
  // Before settings load, so an error raised while resolving them is reported
  // too (the sink itself checks the Analytics switch per error).
  installAnalyticsErrorReporting(container);
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

/// Last-resort screen when start-up cannot reach a writable data directory.
///
/// Deliberately plain (no Mixar/Shad theme or providers): it renders before
/// settings are loaded and only has to show why the app cannot continue.
class StartupFailureApp extends StatelessWidget {
  const new({required this.message, super.key});

  final String message;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      home: Scaffold(
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Text(
              'Mixar could not start:\n$message',
              textAlign: TextAlign.center,
            ),
          ),
        ),
      ),
    );
  }
}
