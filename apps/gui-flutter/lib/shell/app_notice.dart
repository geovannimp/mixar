import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// User-facing notice kinds routed through [appNoticeProvider] → toast bridge.
enum AppNoticeKind { notice, error }

/// Typed app notice. [seq] bumps so identical text still triggers a toast.
@immutable
class AppNotice {
  const new({required this.kind, required this.message, required this.seq});

  final AppNoticeKind kind;
  final String message;
  final int seq;
}

class AppNoticeNotifier extends Notifier<AppNotice?> {
  var _seq = 0;

  @override
  AppNotice? build() => null;

  void clear() => state = null;

  void setError(String message) {
    _seq += 1;
    state = AppNotice(kind: AppNoticeKind.error, message: message, seq: _seq);
  }

  void setNotice(String? message) {
    if (message == null) {
      state = null;
      return;
    }
    _seq += 1;
    state = AppNotice(kind: AppNoticeKind.notice, message: message, seq: _seq);
  }
}

/// Library / engine notices. Shell bridge shows Mixar toasts; no sticky UI.
final appNoticeProvider = NotifierProvider<AppNoticeNotifier, AppNotice?>(
  AppNoticeNotifier.new,
);
