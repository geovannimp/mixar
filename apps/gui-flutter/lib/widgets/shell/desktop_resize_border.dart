import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:window_manager/window_manager.dart';

/// Edge and corner resize handles for the app's undecorated desktop window.
///
/// The Linux runner turns GTK decorations off (see
/// `linux/runner/my_application.cc`), which removes the window manager's
/// resize frame; window_manager 0.7 expects a custom-title-bar app to supply
/// the handles itself. This is the Flutter half of the Tauri
/// `WindowResizeBorder` the shell replaced.
class DesktopResizeBorder extends StatefulWidget {
  const new({required this.child, super.key});

  final Widget child;

  @override
  State<DesktopResizeBorder> createState() => _DesktopResizeBorderState();
}

class _DesktopResizeBorderState extends State<DesktopResizeBorder> {
  var _maximized = debugOverrideWindowMaximized ?? false;
  ListenerId? _listener;

  @override
  void initState() {
    super.initState();
    // A test override replaces the native window entirely.
    if (debugOverrideWindowMaximized != null) {
      return;
    }
    final window = currentNativeWindow();
    if (window == null) {
      return;
    }
    _maximized = window.isMaximized;
    _listener = WindowManager.instance.addListener((event) {
      final maximized = switch (event) {
        WindowMaximizedEvent() => true,
        WindowRestoredEvent() => false,
        _ => null,
      };
      if (maximized != null && mounted && maximized != _maximized) {
        setState(() => _maximized = maximized);
      }
    });
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
    if (!isDesktopWindow) {
      return widget.child;
    }
    return DragToResizeArea(
      resizeEdgeSize: 8,
      // A maximized window has no edges to drag; leaving the handles up would
      // swallow clicks on the viewport border for a no-op resize.
      enableResizeEdges: _maximized ? const [] : null,
      child: widget.child,
    );
  }
}
