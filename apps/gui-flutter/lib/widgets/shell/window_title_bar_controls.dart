import 'dart:io';

import 'package:material_ui/material_ui.dart';
import 'package:window_manager/window_manager.dart';

/// Min / maximize / close — mirrors Tauri [WindowTitleBarControls].
class WindowTitleBarControls extends StatefulWidget {
  const new({super.key});

  @override
  State<WindowTitleBarControls> createState() => _WindowTitleBarControlsState();
}

class _WindowTitleBarControlsState extends State<WindowTitleBarControls> {
  Window? _window;
  ListenerId? _listener;
  var _maximized = false;

  @override
  void initState() {
    super.initState();
    final window = WindowManager.instance.getCurrent();
    if (window == null) {
      // Hosts without a native window (widget tests) keep the drawn controls
      // inert instead of failing on every press.
      return;
    }
    _window = window;
    _maximized = window.isMaximized;
    // Typed events replace the 0.5.x WindowListener callbacks. Restoring also
    // fires for un-maximize, which is what flips the button back.
    _listener = WindowManager.instance.addListener((event) {
      // A close requested by the window manager (or by the panel) waits for
      // the app, so it has to end the process here as well.
      if (event is WindowCloseRequestedEvent) {
        _close();
        return;
      }
      final maximized = switch (event) {
        WindowMaximizedEvent() => true,
        WindowRestoredEvent() => false,
        _ => null,
      };
      if (maximized != null && mounted) {
        setState(() => _maximized = maximized);
      }
    });
  }

  /// The native window, resolved on demand: the header can build before it
  /// exists, and caching a null from [initState] left the controls dead.
  Window? get _nativeWindow => _window ?? WindowManager.instance.getCurrent();

  /// Ends the app.
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
  /// could block — exactly as a force-quit would. Nothing is persisted at
  /// exit; other platforms close the window and exit.
  void _close() {
    final window = _nativeWindow;
    if (window == null) {
      return;
    }
    if (Platform.isLinux) {
      // Before close(): a close that blocks would keep the kill from running.
      try {
        Process.killPid(pid, ProcessSignal.sigkill);
      } on Object {
        // Fall through to the orderly exit below.
      }
    }
    window.close();
    exit(0);
  }

  @override
  void dispose() {
    final listener = _listener;
    if (listener != null) {
      WindowManager.instance.removeListener(listener);
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final brightness = Theme.brightnessOf(context);

    return Row(
      mainAxisSize: .min,
      children: [
        WindowCaptionButton.minimize(
          brightness: brightness,
          onPressed: () {
            final window = _nativeWindow;
            if (window == null) {
              return;
            }
            if (window.isMinimized) {
              window.restore();
            } else {
              window.minimize();
            }
          },
        ),
        if (_maximized)
          WindowCaptionButton.unmaximize(
            brightness: brightness,
            onPressed: () => _nativeWindow?.unmaximize(),
          )
        else
          WindowCaptionButton.maximize(
            brightness: brightness,
            onPressed: () => _nativeWindow?.maximize(),
          ),
        WindowCaptionButton.close(brightness: brightness, onPressed: _close),
      ],
    );
  }
}
