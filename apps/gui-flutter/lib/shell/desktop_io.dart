import 'dart:io' show Platform, exit;

bool get ioIsDesktop =>
    Platform.isLinux || Platform.isMacOS || Platform.isWindows;

void fatalExit([int code = 1]) => exit(code);
