import 'package:flutter/widgets.dart';
import 'package:flutter_smart_dialog/flutter_smart_dialog.dart';
import 'package:forui/forui.dart';

enum MixarToastVariant { primary, destructive }

var _mixarToastSeq = 0;

/// Mixar toast over [SmartDialog] (Forui-free chrome; theme tokens OK).
void showMixarToast({
  required Widget title,
  MixarToastVariant variant = MixarToastVariant.primary,
  Widget? description,
  Widget Function(BuildContext context, VoidCallback dismiss)? suffixBuilder,
  Duration? duration = const Duration(seconds: 5),
  VoidCallback? onDismiss,
}) {
  final tag = 'mixar-toast-${_mixarToastSeq++}';

  void dismiss() {
    if (duration == null) {
      SmartDialog.dismiss(tag: tag);
    } else {
      SmartDialog.dismiss(status: SmartStatus.custom);
    }
  }

  Widget builder(BuildContext context) {
    return Padding(
      // Match Forui desktop toaster: bottom-end with edge inset.
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 24),
      child: _MixarToastChrome(
        variant: variant,
        title: title,
        description: description,
        suffix: suffixBuilder?.call(context, dismiss),
      ),
    );
  }

  if (duration == null) {
    SmartDialog.show(
      tag: tag,
      alignment: Alignment.bottomRight,
      usePenetrate: true,
      clickMaskDismiss: false,
      maskColor: const Color(0x00000000),
      onDismiss: onDismiss,
      builder: builder,
    );
    return;
  }

  // displayTime disables [tag]; dismiss closes the top custom dialog.
  SmartDialog.show(
    alignment: Alignment.bottomRight,
    usePenetrate: true,
    clickMaskDismiss: false,
    maskColor: const Color(0x00000000),
    displayTime: duration,
    onDismiss: onDismiss,
    builder: builder,
  );
}

class _MixarToastChrome extends StatelessWidget {
  const _MixarToastChrome({
    required this.variant,
    required this.title,
    this.description,
    this.suffix,
  });

  final MixarToastVariant variant;
  final Widget title;
  final Widget? description;
  final Widget? suffix;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final destructive = variant == MixarToastVariant.destructive;
    final bg = destructive ? colors.destructive : colors.background;
    final fg = destructive ? colors.destructiveForeground : colors.foreground;
    final border = destructive ? colors.destructive : colors.border;

    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 420),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: bg,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: border, width: theme.style.borderWidth),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Flexible(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    DefaultTextStyle(
                      style: theme.typography.body.sm.copyWith(
                        color: fg,
                        fontWeight: FontWeight.w600,
                      ),
                      child: title,
                    ),
                    if (description != null) ...[
                      const SizedBox(height: 4),
                      DefaultTextStyle(
                        style: theme.typography.body.sm.copyWith(
                          color: fg.withValues(alpha: 0.85),
                        ),
                        child: description!,
                      ),
                    ],
                  ],
                ),
              ),
              if (suffix != null) ...[const SizedBox(width: 12), suffix!],
            ],
          ),
        ),
      ),
    );
  }
}
