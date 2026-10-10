import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/shell/app_notice.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';

/// Turns [appNoticeProvider] updates into Mixar toasts.
///
/// Durations / variants:
/// - [AppNoticeKind.error] → destructive, default 5s
/// - [AppNoticeKind.notice] → primary, 4s
class AppNoticeBridge extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.listen<AppNotice?>(appNoticeProvider, (prev, next) {
      if (next == null) {
        return;
      }
      final error = next.kind == AppNoticeKind.error;
      showMixarToast(
        context: context,
        variant: error
            ? MixarToastVariant.destructive
            : MixarToastVariant.primary,
        duration: error
            ? const Duration(seconds: 5)
            : const Duration(seconds: 4),
        title: Text(next.message),
      );
    });
    return const SizedBox.shrink();
  }
}
