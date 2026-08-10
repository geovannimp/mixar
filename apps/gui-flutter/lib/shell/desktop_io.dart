import 'dart:io' show Platform;

bool get ioIsDesktop =>
    Platform.isLinux || Platform.isMacOS || Platform.isWindows;
