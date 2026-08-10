import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:gui_flutter/shell/desktop_stub.dart'
    if (dart.library.io) 'package:gui_flutter/shell/desktop_io.dart'
    as platform;

/// Test override — set `false` in widget tests that don't init [window_manager].
bool? debugOverrideDesktopWindow;

/// Desktop hosts where [window_manager] applies (not web / mobile).
bool get isDesktopWindow =>
    debugOverrideDesktopWindow ?? (!kIsWeb && platform.ioIsDesktop);
