import 'dart:async';

import 'package:context_show/context_show.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

enum MixarToastVariant { primary, destructive }

var _mixarToastSeq = 0;

/// Mixar toast over [context_show] (Forui-free chrome; theme tokens OK).
///
/// Pass a stable [id] to dismiss later via [dismissMixarToast] (controller offers).
void showMixarToast({
  required BuildContext context,
  required Widget title,
  MixarToastVariant variant = MixarToastVariant.primary,
  Widget? description,
  Widget Function(BuildContext context, VoidCallback dismiss)? suffixBuilder,
  Duration? duration = const Duration(seconds: 5),
  VoidCallback? onDismiss,
  String? id,
}) {
  final toastId = id ?? 'mixar-toast-${_mixarToastSeq++}';
  // Duration.zero = persistent (controller offer). Null maps the same way.
  final showFor = duration ?? Duration.zero;

  unawaited(
    context
        .show(
          (overlay) {
            void dismiss() {
              unawaited(overlay.close());
            }

            return Builder(
              builder: (overlayContext) {
                return Padding(
                  // Match Forui desktop toaster: bottom-end with edge inset.
                  padding: const EdgeInsets.fromLTRB(16, 16, 16, 24),
                  child: _MixarToastChrome(
                    variant: variant,
                    title: title,
                    description: description,
                    suffix: suffixBuilder?.call(overlayContext, dismiss),
                  ),
                );
              },
            );
          },
          id: toastId,
          duration: showFor,
          alignment: Alignment.bottomRight,
          safeArea: false,
          // Default background is a full-screen hit target; keep UI clickable.
          background: (_) => const SizedBox.shrink(),
        )
        .whenComplete(() => onDismiss?.call()),
  );
}

/// Close toasts previously shown with [showMixarToast]'s [id].
void dismissMixarToast(BuildContext context, {required String id}) {
  unawaited(context.close(Overlays.all(id: id)));
}

class _MixarToastChrome extends StatelessWidget {
  const new({
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
          borderRadius: theme.style.borderRadius.md,
          border: Border.all(color: border, width: theme.style.borderWidth),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
          // Shrink-wrap to content; action sits under copy (not beside it).
          child: IntrinsicWidth(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
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
                if (suffix != null) ...[
                  const SizedBox(height: 12),
                  Align(alignment: Alignment.centerRight, child: suffix!),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}
