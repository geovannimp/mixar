import 'dart:async';

import 'package:flutter/foundation.dart'
    show TargetPlatform, defaultTargetPlatform, kIsWeb;
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:window_manager/window_manager.dart';

/// GNOME/Adwaita-style radius for undecorated windows (matches compositor shadow).
const double kDesktopWindowRadius = 12;

/// Whether this host's window chrome is rounded.
///
/// Only GNOME-style Linux windows are: Windows draws square window corners, so
/// a rounded clip leaves bright corner wedges against the desktop, and macOS
/// owns its own window shape. Those stay flat.
bool get isRoundedDesktopChrome =>
    !kIsWeb && defaultTargetPlatform == TargetPlatform.linux;

/// Clips desktop content to rounded top corners so compositor shadows
/// aren't squared, and strokes the window edge with the section hairline.
///
/// Requires a transparent native window background (see
/// `linux/runner/my_application.cc` RGBA workaround for
/// https://github.com/leanflutter/window_manager/issues/179 and
/// window_manager `WindowOptions.backgroundColor`). Radius and frame drop
/// when maximized, and are absent on every host except Linux — see
/// [isRoundedDesktopChrome].
///
/// On those hosts it must sit under a [MixarTheme]: the frame is painted with
/// the theme's `colors.border` and `style.borderWidth`, the same pair every
/// card and divider uses.
class DesktopChrome extends StatefulWidget {
  const new({required this.child, super.key});

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
    unawaited(
      windowManager.isMaximized().then((value) {
        if (mounted) {
          setState(() => _maximized = value);
        }
      }),
    );
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
    if (!isDesktopWindow || !isRoundedDesktopChrome) {
      return widget.child;
    }
    final theme = context.theme;
    // Maximized windows square off and drop the frame: a stroke there would
    // ring the screen edge rather than the window.
    final radius = _maximized
        ? BorderRadius.zero
        : const BorderRadius.only(
            topLeft: Radius.circular(kDesktopWindowRadius),
            topRight: Radius.circular(kDesktopWindowRadius),
          );
    return ClipRRect(
      borderRadius: radius,
      // The frame is the same hairline as every section (cards, dividers): the
      // window edge against the desktop is a section border too.
      child: DecoratedBox(
        // Foreground: the shell paints an opaque background, so a stroke drawn
        // behind it would never show.
        position: DecorationPosition.foreground,
        decoration: BoxDecoration(
          border: _maximized
              ? null
              : Border.all(
                  color: theme.colors.border,
                  width: theme.style.borderWidth,
                ),
          borderRadius: radius,
        ),
        child: widget.child,
      ),
    );
  }
}
