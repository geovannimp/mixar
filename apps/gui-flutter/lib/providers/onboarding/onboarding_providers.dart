import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

/// Settings the first-run flow is editing, before Finish persists them.
///
/// `Application` reads it for theme, accent and locale so a step choice
/// previews live instead of only landing after Finish.
class OnboardingDraft extends Notifier<AppSettings?> {
  @override
  AppSettings? build() {
    // Clears itself once a save lands with setup completed, so later Settings
    // edits are not masked by a stale preview.
    ref.listen(appSettingsProvider, (_, _) {
      if (_setupSaved) {
        state = null;
      }
    });
    return null;
  }

  /// Whether the saved settings already show setup done.
  bool get _setupSaved =>
      ref.read(appSettingsProvider).asData?.value.onboardingCompleted ?? false;

  void set(AppSettings? draft) {
    // Finish saves before this page stops: by then the listener above has
    // already cleared the draft, and installing one again would keep masking
    // later Settings edits for the rest of the session.
    if (draft != null && _setupSaved) {
      return;
    }
    state = draft;
  }
}

final onboardingDraftProvider = NotifierProvider<OnboardingDraft, AppSettings?>(
  OnboardingDraft.new,
);

/// MIDI inputs the OS reports before the engine exists.
///
/// Drives the setup step: no controller means the step is dropped from the
/// flow, so a person without hardware never sees it. A failed probe counts as
/// none — setup must not depend on MIDI being available.
final onboardingControllersProvider = FutureProvider<List<DetectedController>>(
  (ref) => detectControllers(),
);
