import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
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
  if (_isCompactDialog(context)) {
    return const BorderRadius.vertical(top: Radius.circular(16));
  }
  return const BorderRadius.all(Radius.circular(16));
}

ShadDialog _mixarDialogShell({
  required BuildContext context,
  Widget? child,
  Widget? title,
  Widget? description,
  List<Widget> actions = const [],
  EdgeInsetsGeometry padding = EdgeInsets.zero,
  double gap = 0,
}) {
  final theme = context.theme;
  return ShadDialog(
    title: title,
    description: description,
    actions: actions,
    padding: padding,
    gap: gap,
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
    expandActionsWhenTiny: false,
    actionsAxis: Axis.horizontal,
    actionsMainAxisSize: MainAxisSize.min,
    actionsMainAxisAlignment: MainAxisAlignment.end,
    titleStyle: theme.typography.body.md.copyWith(fontWeight: FontWeight.w700),
    descriptionStyle: theme.typography.body.sm.copyWith(
      color: theme.colors.foreground,
    ),
    titleTextAlign: TextAlign.start,
    descriptionTextAlign: TextAlign.start,
    crossAxisAlignment: CrossAxisAlignment.stretch,
    child: child,
  );
}

/// Confirmation with title + in-flow actions.
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
        padding: const EdgeInsets.all(16),
        gap: 16,
        title: Text(title),
        description: body == null ? null : Text(body),
        actions: [
          for (final action in actions)
            AppButton(
              mainAxisSize: MainAxisSize.min,
              variant: action.variant,
              onPress: () => Navigator.of(modalContext).pop(action.value),
              child: Text(action.label),
            ),
        ],
      );
    },
  );
}

/// Content-owned dialog (forms / pickers). Call site supplies title + actions.
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
