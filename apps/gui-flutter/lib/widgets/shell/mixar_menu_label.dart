import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_dialog_chrome.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Static, non-interactive menu row (section header / disabled label).
///
/// Unlike `MixarMenuItem` this never wraps in an interactive widget, so it is
/// not announced as a button and stays in normal reading order. Use it for
/// labels like "Load to deck" that head a group of actions.
class MixarMenuLabel extends StatelessWidget {
  const new({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final dialogChrome = MixarMenuDialogChrome.isActive(context);
    return Padding(
      // Dialog rows are full-bleed; labels take the rows' 24px text inset so
      // they align with the dialog title.
      padding: dialogChrome
          ? const EdgeInsets.fromLTRB(24, 12, 24, 8)
          : const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      child: DefaultTextStyle(
        style: theme.typography.body.sm.copyWith(
          color: theme.colors.mutedForeground,
        ),
        child: child,
      ),
    );
  }
}
