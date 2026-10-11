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

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AppNotice &&
          kind == other.kind &&
          message == other.message &&
          seq == other.seq;

  @override
  int get hashCode => Object.hash(kind, message, seq);
}

class AppNoticeNotifier extends Notifier<AppNotice?> {
  /// Process-wide so seq does not restart after provider recreation/hot reload.
  static var _globalSeq = 0;

  @override
  AppNotice? build() => null;

  void clear() => state = null;

  void setError(String message) {
    _globalSeq += 1;
    state = AppNotice(
      kind: AppNoticeKind.error,
      message: message,
      seq: _globalSeq,
    );
  }

  void setNotice(String? message) {
    if (message == null || message.isEmpty) {
      state = null;
      return;
    }
    _globalSeq += 1;
    state = AppNotice(
      kind: AppNoticeKind.notice,
      message: message,
      seq: _globalSeq,
    );
  }
}

/// Library / engine notices. Shell bridge shows Mixar toasts; no sticky UI.
final appNoticeProvider = NotifierProvider<AppNoticeNotifier, AppNotice?>(
  AppNoticeNotifier.new,
);
