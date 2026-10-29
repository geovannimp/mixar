import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter/foundation.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/bus.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';

/// The one process-wide event bus.
///
/// Engine, library and controller transports all publish on it, and every Dart
/// listener subscribes to this single stream instead of one per transport.
final busTransportProvider = FutureProvider<BusTransport>(
  (ref) => Future.sync(BusTransport.open),
);

/// Long-lived subscription that routes each typed event to the state it drives.
///
/// Mounted once from the shell; the transport itself is kept alive by
/// [busTransportProvider]'s dependents.
final busEventsBootstrapProvider = Provider<void>((ref) {
  final busAsync = ref.watch(busTransportProvider);
  if (busAsync case AsyncData(:final value)) {
    final sub = value.subscribeEvents().listen(
      (event) => dispatchBusEvent(ref, event),
      onError: (Object e, StackTrace _) {
        if (!ref.mounted) {
          return;
        }
        ref.read(appNoticeProvider.notifier).setError('$e');
      },
    );
    ref.onDispose(sub.cancel);
  }
});

/// Route one bus event to the state it drives.
///
/// Exposed for tests, which feed events without a live transport.
@visibleForTesting
void dispatchBusEvent(Ref ref, Event event) {
  final ui = ref.read(engineUiProvider.notifier);
  if (event case Event_Error(:final field0)) {
    ui.apply(event);
    ref.read(appNoticeProvider.notifier).setError(field0.message);
    return;
  }
  if (event case Event_Notice(:final field0)) {
    ref.read(appNoticeProvider.notifier).setNotice(field0.message);
    return;
  }
  ui.apply(event);
}

/// Subscribe to the bus for a widget-level listener that needs raw events.
///
/// Most code should watch derived providers; this is for UI that reacts to an
/// edge (a controller offer, a one-off notice).
StreamSubscription<Event> listenBusEvents(
  Ref ref,
  void Function(Event event) onEvent,
  void Function(Object error)? onError,
) {
  final bus = ref.read(busTransportProvider).asData?.value;
  if (bus == null) {
    return const Stream<Event>.empty().listen((_) {});
  }
  final sub = bus.subscribeEvents().listen(onEvent, onError: onError);
  ref.onDispose(sub.cancel);
  return sub;
}
