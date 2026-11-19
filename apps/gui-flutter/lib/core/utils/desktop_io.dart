import 'dart:io' show Platform;

bool get ioIsDesktop =>
    Platform.isLinux || Platform.isMacOS || Platform.isWindows;

/// The macOS chrome preview switched on through the environment.
///
/// `bool.fromEnvironment` only sees `--dart-define`, and the everyday
/// `npm run app:linux` path passes no defines. Reading the same switch from
/// the process environment makes `MIXAR_PREVIEW_MACOS_CHROME=true npm run
/// app:linux` behave like the documented `--dart-define` launch.
bool get ioMacOSChromePreview {
  final value = Platform.environment['MIXAR_PREVIEW_MACOS_CHROME'];
  return value == '1' || value?.toLowerCase() == 'true';
}
