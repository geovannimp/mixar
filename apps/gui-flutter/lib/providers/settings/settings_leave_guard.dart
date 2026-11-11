import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Asks the open Settings page whether it may be left, running its
/// unsaved-edits guard first.
///
/// The shell owns navigation — the pages menu behind the header's hamburger —
/// but the draft being edited lives in the page, so the check has to come from
/// there. The page registers its guard on mount and clears it when the shell's
/// copy is still its own.
final settingsLeaveGuardProvider =
    NotifierProvider<SettingsLeaveGuard, Future<bool> Function()?>(
      SettingsLeaveGuard.new,
    );

class SettingsLeaveGuard extends Notifier<Future<bool> Function()?> {
  @override
  Future<bool> Function()? build() => null;

  /// Registers the open page's guard; null once it is gone.
  void set(Future<bool> Function()? guard) => state = guard;
}
