import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Narrow width → bottom-anchored sheet; otherwise centered dialog.
const mixarDialogBreakpoint = 768.0;

class MixarDialogAction<T> {
  const new({
    required this.label,
    required this.value,
    this.variant = MixarButtonVariant.primary,
  });

  final String label;
  final T value;
  final MixarButtonVariant variant;
}

/// Shared dialog surface: one framed card, a title band, full-bleed rules, an
/// optional content area, and an optional actions footer.
///
/// Bodies compose this widget instead of drawing their own titles, padding or
/// frames, so every dialog in the app shares the same chrome. Content is padded
/// by [contentPadding]; pass [EdgeInsets.zero] for full-bleed section lists.
/// Action buttons sit in a footer below a rule and are right-aligned.
class MixarDialog extends StatelessWidget {
  const new({
    this.title,
    super.key,
    this.description,
    this.child,
    this.actions = const [],
    this.contentPadding = const EdgeInsets.symmetric(
      horizontal: 24,
      vertical: 16,
    ),
    this.actionsPadding = const EdgeInsets.fromLTRB(24, 12, 24, 16),
  });

  final String? title;

  /// Muted line under the title. Omit for a title-only header.
  final String? description;

  /// Dialog body. Omit for a message-only dialog (confirmations).
  final Widget? child;

  final List<Widget> actions;
  final EdgeInsetsGeometry contentPadding;
  final EdgeInsetsGeometry actionsPadding;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    Widget rule() => SizedBox(
      width: double.infinity,
      height: theme.style.borderWidth,
      child: ColoredBox(color: theme.colors.border),
    );

    return DecoratedBox(
      // Foreground: dialog content runs full-bleed, so the shell's border would
      // otherwise be covered by the card bands.
      position: DecorationPosition.foreground,
      decoration: BoxDecoration(
        border: Border.all(
          color: theme.colors.border,
          width: theme.style.borderWidth,
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          // Right inset leaves room for the shell's close button.
          if (title case final title?) ...[
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 16, 56, 16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 4,
                children: [
                  Text(
                    title,
                    style: theme.typography.body.md.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  if (description case final description?)
                    Text(
                      description,
                      style: theme.typography.body.sm.copyWith(
                        color: theme.colors.mutedForeground,
                      ),
                    ),
                ],
              ),
            ),
            rule(),
          ],
          if (child case final child?)
            Padding(padding: contentPadding, child: child),
          if (actions.isNotEmpty) ...[
            // A message-only dialog already ends with the header rule.
            if (child != null) rule(),
            Padding(
              padding: actionsPadding,
              child: Row(
                mainAxisAlignment: MainAxisAlignment.end,
                spacing: 8,
                children: actions,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

bool _isCompactDialog(BuildContext context) =>
    MediaQuery.sizeOf(context).width < mixarDialogBreakpoint;

BoxConstraints _mixarDialogConstraints(BuildContext context) {
  final size = MediaQuery.sizeOf(context);
  if (size.width < mixarDialogBreakpoint) {
    return BoxConstraints(minWidth: size.width, maxWidth: size.width);
  }
  return const BoxConstraints(maxWidth: 512);
}

BorderRadius _mixarDialogRadius(BuildContext context) {
  return context.theme.style.borderRadius.lg;
}

ShadDialog _mixarDialogShell({
  required BuildContext context,
  required Widget child,
}) {
  final theme = context.theme;
  // The shared header sits in the 16px top inset with body.md (16px / height
  // 1.5 → 24). Center the larger close button on that title band.
  const titleInset = 16.0;
  const titleLineHeight = 24.0;
  const closeSize = 32.0;
  const closeIconSize = 20.0;
  const closeTop = titleInset + titleLineHeight / 2 - closeSize / 2;
  return ShadDialog(
    padding: EdgeInsets.zero,
    // Tall form/picker bodies (collections, pads, selects) must scroll.
    scrollable: true,
    backgroundColor: theme.colors.card,
    radius: _mixarDialogRadius(context),
    border: Border.all(
      color: theme.colors.border,
      width: theme.style.borderWidth,
    ),
    shadows: const [],
    constraints: _mixarDialogConstraints(context),
    alignment: _isCompactDialog(context)
        ? Alignment.bottomCenter
        : Alignment.center,
    removeBorderRadiusWhenTiny: false,
    crossAxisAlignment: CrossAxisAlignment.stretch,
    closeIcon: AppButton.icon(
      variant: MixarButtonVariant.ghost,
      size: MixarButtonSize.sm,
      // Keep a 32×32 hit target around the 20px glyph so top math stays exact.
      padding: const EdgeInsets.all((closeSize - closeIconSize) / 2),
      semanticsLabel: 'Close',
      onPress: () => Navigator.of(context).pop(),
      child: const Icon(LucideIcons.x, size: closeIconSize),
    ),
    closeIconPosition: ShadPosition.directional(
      top: closeTop,
      end: titleInset - (closeSize - closeIconSize) / 2,
      textDirection: Directionality.of(context),
    ),
    child: child,
  );
}

/// Confirmation with title, optional body and in-flow actions.
Future<T?> showMixarConfirm<T>({
  required BuildContext context,
  required String title,
  required List<MixarDialogAction<T>> actions,
  String? body,
}) {
  assert(actions.isNotEmpty, 'showMixarConfirm requires at least one action');
  return showShadDialog<T>(
    context: context,
    builder: (modalContext) {
      return _mixarDialogShell(
        context: modalContext,
        child: MixarDialog(
          title: title,
          description: body,
          actions: [
            for (final action in actions)
              AppButton(
                mainAxisSize: MainAxisSize.min,
                variant: action.variant,
                onPress: () => Navigator.of(modalContext).pop(action.value),
                child: Text(action.label),
              ),
          ],
        ),
      );
    },
  );
}

/// Content-owned dialog (forms / pickers). The builder returns a [MixarDialog].
Future<T?> showMixarDialog<T>({
  required BuildContext context,
  required WidgetBuilder builder,
}) {
  return showShadDialog<T>(
    context: context,
    builder: (modalContext) {
      return _mixarDialogShell(
        context: modalContext,
        child: builder(modalContext),
      );
    },
  );
}
