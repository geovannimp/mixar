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
        WindowCaptionButton.close(
          brightness: brightness,
          onPressed: () => _nativeWindow?.close(),
        ),
      ],
    );
  }
}
