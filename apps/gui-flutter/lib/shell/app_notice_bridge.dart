import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/shell/app_notice.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';

/// Turns [appNoticeProvider] updates into Mixar toasts.
///
/// Durations / variants:
/// - [AppNoticeKind.error] → destructive, default 5s
/// - [AppNoticeKind.notice] → primary, 4s
class AppNoticeBridge extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<AppNoticeBridge> createState() => _AppNoticeBridgeState();
}

class _AppNoticeBridgeState extends ConsumerState<AppNoticeBridge> {
  ProviderSubscription<AppNotice?>? _listen;

  @override
  void initState() {
    super.initState();
    _listen = ref.listenManual<AppNotice?>(appNoticeProvider, (prev, next) {
      if (!mounted || next == null) {
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
  }

  @override
  void dispose() {
    _listen?.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
}
