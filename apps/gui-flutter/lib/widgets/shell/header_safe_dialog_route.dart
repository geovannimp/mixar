import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';

/// A modal route that leaves the app titlebar clickable.
///
/// A normal [PopupRoute] installs a full-window [ModalBarrier]; that barrier
/// sits over the header too, so a click on the titlebar dismisses the overlay
/// instead of reaching its buttons or the window controls. This route installs
/// no barrier of its own: pages put a [HeaderSafeBarrier] below the header.
class HeaderSafeDialogRoute<T> extends RawDialogRoute<T> {
  new({
    required super.pageBuilder,
    super.transitionBuilder,
    super.transitionDuration,
    super.settings,
    String barrierLabel = 'Close',
  }) : super(
         barrierDismissible: true,
         barrierLabel: barrierLabel,
         barrierColor: const Color(0x00000000),
       );

  @override
  Widget buildModalBarrier() => const SizedBox.shrink();
}

/// Body-only dim and dismissal layer for [HeaderSafeDialogRoute].
///
/// Returns a [Positioned] and must be a direct child of a [Stack]. It covers
/// everything below [kAppHeaderHeight], so the titlebar keeps its normal
/// colour and hit tests still reach the header.
class HeaderSafeBarrier extends StatelessWidget {
  const new({
    required this.onDismiss,
    this.color = const Color(0x80000000),
    this.label = 'Close',
    super.key,
  });

  final VoidCallback onDismiss;
  final Color color;
  final String label;

  @override
  Widget build(BuildContext context) {
    final animation =
        ModalRoute.of(context)?.animation ?? kAlwaysCompleteAnimation;
    return Positioned(
      top: kAppHeaderHeight,
      left: 0,
      right: 0,
      bottom: 0,
      child: FadeTransition(
        opacity: animation,
        child: ModalBarrier(
          onDismiss: onDismiss,
          color: color,
          semanticsLabel: label,
        ),
      ),
    );
  }
}
