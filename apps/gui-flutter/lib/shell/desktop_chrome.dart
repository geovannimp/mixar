import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:window_manager/window_manager.dart';

/// GNOME/Adwaita-style radius for undecorated windows (matches compositor shadow).
const double kDesktopWindowRadius = 12;

/// Clips desktop content to rounded top corners so compositor shadows aren't squared.
///
/// Requires a transparent native window background (see `linux/runner/my_application.cc`
/// RGBA workaround for https://github.com/leanflutter/window_manager/issues/179
/// and [WindowOptions.backgroundColor]). Radius drops to 0 when maximized.
class DesktopChrome extends StatefulWidget {
  const DesktopChrome({required this.child, super.key});

  final Widget child;

  @override
  State<DesktopChrome> createState() => _DesktopChromeState();
}

class _DesktopChromeState extends State<DesktopChrome> with WindowListener {
  var _maximized = false;

  @override
  void initState() {
    super.initState();
    if (!isDesktopWindow) {
      return;
    }
    windowManager.addListener(this);
    windowManager.isMaximized().then((value) {
      if (mounted) {
        setState(() => _maximized = value);
      }
    });
  }

  @override
  void dispose() {
    if (isDesktopWindow) {
      windowManager.removeListener(this);
    }
    super.dispose();
  }

  @override
  void onWindowMaximize() => setState(() => _maximized = true);

  @override
  void onWindowUnmaximize() => setState(() => _maximized = false);

  @override
  Widget build(BuildContext context) {
    if (!isDesktopWindow) {
      return widget.child;
    }
    return ClipRRect(
      borderRadius: _maximized
          ? BorderRadius.zero
          : const BorderRadius.only(
              topLeft: Radius.circular(kDesktopWindowRadius),
              topRight: Radius.circular(kDesktopWindowRadius),
            ),
      child: widget.child,
    );
  }
}
