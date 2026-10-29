import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Stage every library form drawer shares.
///
/// The panel slides in from the right edge, starts below the titlebar so it
/// never covers the window controls, and never grows wider than the viewport.
/// Dismissal is a barrier tap or the panel's own close button, so a form
/// reports "nothing" by popping without a value.
Future<T?> showLibraryFormDrawer<T>(
  BuildContext context, {
  required String barrierLabel,
  required double width,
  required WidgetBuilder builder,
}) {
  return showGeneralDialog<T>(
    context: context,
    barrierDismissible: true,
    barrierLabel: barrierLabel,
    transitionBuilder: (context, animation, _, child) {
      return SlideTransition(
        position: Tween<Offset>(begin: const Offset(1, 0), end: Offset.zero)
            .animate(
              CurvedAnimation(parent: animation, curve: Curves.easeOutCubic),
            ),
        child: child,
      );
    },
    pageBuilder: (ctx, _, _) => Padding(
      // The drawer stage is the body below the titlebar, as with the library
      // drawer: the panel never covers the window controls.
      padding: const EdgeInsets.only(top: kAppHeaderHeight),
      child: Align(
        alignment: Alignment.centerRight,
        child: SizedBox(
          width: math.min(width, MediaQuery.sizeOf(ctx).width),
          height: double.infinity,
          child: builder(ctx),
        ),
      ),
    ),
  );
}

/// Chrome of a library form drawer: card surface with a ruled inner edge, a
/// title row carrying the close button, a scrolling body, and a pinned action
/// row so the primary button stays reachable in short windows.
class LibraryFormDrawerShell extends StatelessWidget {
  const new({
    required this.title,
    required this.child,
    required this.action,
    this.subtitle,
    super.key,
  });

  final String title;
  final String? subtitle;
  final Widget child;
  final Widget action;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.card,
        // Square corners, flush to the screen edge: only the inner edge rules.
        border: Border(
          left: BorderSide(
            color: theme.colors.border,
            width: theme.style.borderWidth,
          ),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        title,
                        style: theme.typography.body.md.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      if (subtitle != null && subtitle!.isNotEmpty) ...[
                        const SizedBox(height: 4),
                        Text(
                          subtitle!,
                          style: theme.typography.body.sm.copyWith(
                            color: theme.colors.mutedForeground,
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
                AppButton.icon(
                  variant: MixarButtonVariant.ghost,
                  size: MixarButtonSize.sm,
                  semanticsLabel: 'Close',
                  onPress: () => Navigator.of(context).pop(),
                  child: const Icon(LucideIcons.x, size: 20),
                ),
              ],
            ),
          ),
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(16),
              child: child,
            ),
          ),
          Padding(padding: const EdgeInsets.all(16), child: action),
        ],
      ),
    );
  }
}
