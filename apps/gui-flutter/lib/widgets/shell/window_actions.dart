import 'dart:io';

import 'package:flutter/foundation.dart' show visibleForTesting;
import 'package:window_manager/window_manager.dart';

/// Ends the app from a window control.
///
/// `close()` only destroys the window: window_manager 0.7 quits the process
/// itself just for platforms without a native close, and the Flutter runner
/// never ends on the last window being gone.
///
/// Exiting is not straightforward either. `exit()` reaches `exit_group` at
/// once, but the kernel then waits on the loaded GPU/audio stack (NVIDIA +
/// gallium + PipeWire), which took ~10s here: the window stayed on screen and
/// GNOME offered to force-quit. The same process dies in ~0.2s on SIGKILL, so
/// on Linux the process is killed first — before any native close call that
/// could block — exactly as a force-quit would. Nothing is persisted at exit;
/// other platforms close the window and exit.
void closeApp() {
  final window = WindowManager.instance.getCurrent();
  if (window == null) {
    // No native window (widget tests): leave the host alone.
    return;
  }
  runCloseSequence(
    linux: Platform.isLinux,
    // Before close(): a close that blocks would keep the kill from running.
    forceKill: () => Process.killPid(pid, ProcessSignal.sigkill),
    closeWindow: window.close,
    exitApp: exit,
  );
}

/// The close sequence, split out so both host paths are testable.
///
/// On Linux a delivered SIGKILL ends the process at once, so the native close
/// and `exit` never run behind it; a kill that did not land, and every other
/// host, close the window and exit.
@visibleForTesting
void runCloseSequence({
  required bool linux,
  required bool Function() forceKill,
  required void Function() closeWindow,
  required void Function(int code) exitApp,
}) {
  if (linux && forceKill()) return;
  closeWindow();
  exitApp(0);
}
