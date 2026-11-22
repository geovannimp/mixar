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

  /// True from the page change until the exit finishes: the painted button is
  /// already gone as far as input is concerned.
  var _leaving = false;

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
      _leaving = false;
      return;
    }
    // Inert from this frame on, however long the fade takes.
    _leaving = true;
    if (MediaQuery.disableAnimationsOf(context)) {
      // Reduced motion: the corner empties with the page.
      _present = false;
      _leaving = false;
      return;
    }
    // `whenCompleteOrCancel` is the ticker API that runs for both endings; a
    // cancelled exit (re-show, unmount) must not leave an error behind.
    _exit.forward(from: 0).whenCompleteOrCancel(() {
      // Only a finished exit clears the row; a re-show has already put it
      // back, and its cancelled ticker must not undo that.
      if (!mounted || !_leaving) {
        return;
      }
      setState(() {
        _present = false;
        _leaving = false;
      });
    });
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
          // A disappearing control must not take a second press, not even on
          // the frame the exit starts.
          ignoring: _leaving,
          child: ExcludeFocus(
            // Nor keep the keyboard's focus while it fades away.
            excluding: _leaving,
            child: Opacity(
              opacity: 1 - t,
              child: Transform.translate(
                offset: Offset(-kAppBackExitDistance * t, 0),
                child: child,
              ),
            ),
          ),
        );
      },
      child: AppButton(
        variant: .ghost,
        size: .sm,
        mainAxisSize: .min,
        semanticsLabel: 'Back to Booth',
        onPress: _leaving ? null : widget.onPress,
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
