import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';

/// Min / maximize / close — mirrors Tauri [WindowTitleBarControls].
class WindowTitleBarControls extends StatefulWidget {
  const WindowTitleBarControls({super.key});

  @override
  State<WindowTitleBarControls> createState() => _WindowTitleBarControlsState();
}

class _WindowTitleBarControlsState extends State<WindowTitleBarControls>
    with WindowListener {
  var _maximized = false;

  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
    _refreshMaximized();
  }

  @override
  void dispose() {
    windowManager.removeListener(this);
    super.dispose();
  }

  @override
  void onWindowMaximize() => setState(() => _maximized = true);

  @override
  void onWindowUnmaximize() => setState(() => _maximized = false);

  Future<void> _refreshMaximized() async {
    final maximized = await windowManager.isMaximized();
    if (mounted) {
      setState(() => _maximized = maximized);
    }
  }

  @override
  Widget build(BuildContext context) {
    final brightness = Theme.brightnessOf(context);

    return Row(
      mainAxisSize: .min,
      children: [
        WindowCaptionButton.minimize(
          brightness: brightness,
          onPressed: () async {
            if (await windowManager.isMinimized()) {
              await windowManager.restore();
            } else {
              await windowManager.minimize();
            }
          },
        ),
        if (_maximized)
          WindowCaptionButton.unmaximize(
            brightness: brightness,
            onPressed: windowManager.unmaximize,
          )
        else
          WindowCaptionButton.maximize(
            brightness: brightness,
            onPressed: windowManager.maximize,
          ),
        WindowCaptionButton.close(
          brightness: brightness,
          onPressed: windowManager.close,
        ),
      ],
    );
  }
}
