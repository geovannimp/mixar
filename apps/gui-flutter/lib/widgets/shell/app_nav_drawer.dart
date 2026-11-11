import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/shell_tab.dart';
import 'package:gui_flutter/core/utils/app_assets.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/nav_list_item.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Width of the navigation drawer's panel.
const double kAppNavDrawerWidth = 220;

/// Shows the shell's navigation drawer from the left edge, with one row per
/// page.
///
/// A modal overlay: the header stays visible above it, so the logo and the
/// hamburger stay put while the pages slide in below, and a press outside the
/// panel closes it.
Future<void> showAppNavDrawer(
  BuildContext context, {
  required String title,
  required ShellTab current,
  required Future<void> Function(ShellTab) onSelect,
}) {
  return showGeneralDialog<void>(
    context: context,
    barrierDismissible: true,
    barrierLabel: 'Close menu',
    pageBuilder: (context, _, _) =>
        _AppNavDrawer(title: title, current: current, onSelect: onSelect),
    transitionBuilder: (context, animation, _, child) => SlideTransition(
      position: Tween(
        begin: const Offset(-1, 0),
        end: Offset.zero,
      ).animate(CurvedAnimation(parent: animation, curve: Curves.easeOut)),
      child: child,
    ),
  );
}

/// The drawer's panel, inset below the header so it never covers the chrome
/// that opened it.
class _AppNavDrawer extends StatelessWidget {
  const new({
    required this.title,
    required this.current,
    required this.onSelect,
  });

  final String title;
  final ShellTab current;
  final Future<void> Function(ShellTab) onSelect;

  /// Reports the pick, then closes the menu.
  ///
  /// The order matters: the page change can raise its own modal (the settings
  /// unsaved-edits guard), and a dialog stacked on top of a route that is
  /// still animating out leaves a barrier behind that eats every later press.
  Future<void> _select(BuildContext context, ShellTab tab) async {
    final navigator = Navigator.of(context);
    await onSelect(tab);
    if (navigator.mounted) {
      await navigator.maybePop();
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    // Escape is the desktop way out; the barrier's own label covers screen
    // readers, and the route handles the back button and gesture.
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): () =>
            Navigator.of(context).maybePop(),
      },
      child: Focus(
        autofocus: true,
        child: Align(
          alignment: Alignment.centerLeft,
          child: Padding(
            padding: const EdgeInsets.only(top: kAppHeaderHeight),
            child: SizedBox(
              width: kAppNavDrawerWidth,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: theme.colors.card,
                  border: Border(
                    right: BorderSide(
                      color: theme.colors.border,
                      width: theme.style.borderWidth,
                    ),
                  ),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    // The menu carries the wordmark: it reads as the app's
                    // menu, not a page-local drawer.
                    Padding(
                      padding: const EdgeInsets.fromLTRB(18, 14, 12, 12),
                      child: Align(
                        alignment: Alignment.centerLeft,
                        child: ColorFiltered(
                          colorFilter: ColorFilter.mode(
                            theme.colors.foreground,
                            BlendMode.srcIn,
                          ),
                          child: Image.asset(
                            kMixarLogoAsset,
                            height: 12,
                            semanticLabel: title,
                          ),
                        ),
                      ),
                    ),
                    // Same rows as the settings sidebar: one navigation idiom.
                    NavListItem(
                      icon: LucideIcons.play,
                      label: 'Play',
                      selected: current == ShellTab.mixer,
                      onPress: () => _select(context, ShellTab.mixer),
                    ),
                    NavListItem(
                      icon: LucideIcons.settings,
                      label: 'Settings',
                      selected: current == ShellTab.settings,
                      onPress: () => _select(context, ShellTab.settings),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
