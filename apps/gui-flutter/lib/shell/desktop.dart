import 'dart:io' show Platform;

import 'package:flutter/foundation.dart' show kIsWeb;

/// Test override — set `false` in widget tests that don't init [window_manager].
bool? debugOverrideDesktopWindow;

/// Desktop hosts where [window_manager] applies (not web / mobile).
bool get isDesktopWindow =>
    debugOverrideDesktopWindow ??
    (!kIsWeb && (Platform.isLinux || Platform.isMacOS || Platform.isWindows));
