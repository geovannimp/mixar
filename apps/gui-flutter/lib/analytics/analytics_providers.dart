import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/analytics/analytics_transport.dart';
import 'package:gui_flutter/analytics/error_reporting.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart' show AppSettings;

/// The Analytics switch, or false while settings have not resolved yet.
bool _isAnalyticsEnabled(AsyncValue<AppSettings> settings) =>
    settings.asData?.value.analyticsEnabled ?? false;

/// Install the process-wide handlers that forward runtime errors to PostHog
/// error tracking.
///
/// Call once, before `runApp`, so nothing is missed after the first frame.
/// Consent is re-read per error, so flipping the switch off applies to the next
/// one; an error raised before settings resolve waits for them rather than
/// being dropped. The client itself is armed by [analyticsBootstrapProvider]
/// after the engine starts, so a report that races that arming is dropped by
/// the transport rather than sent early.
void installAnalyticsErrorReporting(ProviderContainer container) {
  // Bound the work: a widget that throws every frame must not leave unbounded
  // futures suspended on a settings provider that never resolves. Beyond this
  // many in flight, reports are dropped — the Rust side would collapse repeats
  // anyway.
  var inFlight = 0;
  const maxInFlight = 8;
  const reportTimeout = Duration(seconds: 5);
  installErrorReporting((report) {
    if (inFlight >= maxInFlight) {
      return;
    }
    inFlight++;
    Future<void> send() async {
      try {
        final settings = await container
            .read(appSettingsProvider.future)
            .timeout(reportTimeout);
        if (!settings.analyticsEnabled) {
          return;
        }
        final analytics = await container
            .read(analyticsTransportProvider.future)
            .timeout(reportTimeout);
        await analytics.reportError(
          source: report.source,
          kind: report.kind,
          message: report.message,
          stack: report.stack,
        );
      } on Object {
        // Analytics must never change how an error reaches the user, including
        // when settings or the transport never answer.
      } finally {
        inFlight--;
      }
    }

    unawaited(send());
  });
}

/// Send one launch event after a successful engine start and explicit opt-in.
final analyticsBootstrapProvider = Provider<void>((ref) {
  var sent = false;

  Future<void> sendIfAllowed() async {
    if (sent) {
      return;
    }
    final engine = ref.read(engineTransportProvider).asData?.value;
    if (!_isAnalyticsEnabled(ref.read(appSettingsProvider)) || engine == null) {
      return;
    }
    sent = true;
    try {
      final analytics = await ref.read(analyticsTransportProvider.future);
      await analytics.setEnabled(enabled: true);
      await analytics.appLaunched();
    } on Object {
      sent = false;
      // Analytics failures must never change engine or UI behavior, but a
      // failed first attempt should be retried on the next trigger.
    }
  }

  ref
    ..listen(appSettingsProvider, (_, _) => unawaited(sendIfAllowed()))
    ..listen(engineTransportProvider, (_, _) => unawaited(sendIfAllowed()));
  unawaited(sendIfAllowed());
});
