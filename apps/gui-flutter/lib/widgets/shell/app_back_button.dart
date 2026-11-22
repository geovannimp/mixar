import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// How long the leading back button takes to leave.
const kAppBackExitDuration = Duration(milliseconds: 180);

/// How far left the back button travels while it fades.
const kAppBackExitDistance = 14.0;

/// The header's leading way back to the booth.
///
/// It appears with the page that needs it. When [visible] goes false it plays a
/// short exit — left into the corner and a fade — before its slot empties, so
/// the page change never snaps the chrome out from under the press.
class AppBackButton extends StatefulWidget {
  const new({required this.visible, required this.onPress, super.key});

  final bool visible;
  final VoidCallback onPress;

  @override
  State<AppBackButton> createState() => _AppBackButtonState();
}

class _AppBackButtonState extends State<AppBackButton>
    with SingleTickerProviderStateMixin {
  late final AnimationController _exit = AnimationController(
    vsync: this,
    duration: kAppBackExitDuration,
  );
  late bool _present = widget.visible;

  @override
  void didUpdateWidget(AppBackButton oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.visible == oldWidget.visible) {
      return;
    }
    if (widget.visible) {
      // Back on a page that needs the way out: show it at once, not midway
      // through an exit another press started.
      _exit
        ..stop()
        ..value = 0;
      _present = true;
      return;
    }
    if (MediaQuery.disableAnimationsOf(context)) {
      // Reduced motion: the corner empties with the page.
      _present = false;
      return;
    }
    unawaited(
      _exit.forward(from: 0).whenComplete(() {
        if (mounted) {
          setState(() => _present = false);
        }
      }),
    );
  }

  @override
  void dispose() {
    _exit.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (!_present) {
      return const SizedBox.shrink();
    }
    return AnimatedBuilder(
      animation: _exit,
      builder: (context, child) {
        // Accelerating away: the press reads as a throw out of the corner.
        final t = Curves.easeInCubic.transform(_exit.value);
        return IgnorePointer(
          // A disappearing control must not take a second press.
          ignoring: _exit.value > 0,
          child: Opacity(
            opacity: 1 - t,
            child: Transform.translate(
              offset: Offset(-kAppBackExitDistance * t, 0),
              child: child,
            ),
          ),
        );
      },
      child: AppButton(
        variant: .ghost,
        size: .sm,
        mainAxisSize: .min,
        semanticsLabel: 'Back to Booth',
        onPress: widget.onPress,
        child: const Row(
          mainAxisSize: .min,
          children: [
            Icon(LucideIcons.arrowLeft),
            SizedBox(width: 6),
            Text('Booth'),
          ],
        ),
      ),
    );
  }
}
