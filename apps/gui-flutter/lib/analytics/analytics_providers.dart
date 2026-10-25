import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/analytics/analytics_transport.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';

/// Send one launch event after a successful engine start and explicit opt-in.
final analyticsBootstrapProvider = Provider<void>((ref) {
  var sent = false;

  Future<void> sendIfAllowed() async {
    if (sent) {
      return;
    }
    final settings = ref.read(appSettingsProvider).asData?.value;
    final engine = ref.read(engineTransportProvider).asData?.value;
    if (settings == null || !settings.analyticsEnabled || engine == null) {
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
