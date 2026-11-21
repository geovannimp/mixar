import 'dart:io';

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
  if (Platform.isLinux) {
    // Before close(): a close that blocks would keep the kill from running.
    if (Process.killPid(pid, ProcessSignal.sigkill)) {
      // SIGKILL lands at once; the lines below run only if it did not.
      return;
    }
  }
  window.close();
  exit(0);
}
