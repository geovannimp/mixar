import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';

class EngineUi extends Notifier<EngineUiSnapshot> {
  @override
  EngineUiSnapshot build() => EngineUiSnapshot.empty;

  void apply(EngineEvt evt) => state = applyEngineEvt(state, evt);

  void setRunning(bool running) => state = state.copyWith(running: running);
}

final engineUiProvider = NotifierProvider<EngineUi, EngineUiSnapshot>(
  EngineUi.new,
);

final engineRunningProvider = Provider<bool>(
  (ref) => ref.watch(engineUiProvider).running,
);

final deckTrackTitleProvider = Provider.family<String?, int>(
  (ref, deckId) => ref.watch(engineUiProvider).titleFor(deckId),
);

/// Starts once on desktop. Widget tests set [debugOverrideDesktopWindow] false
/// so this stays null and skips native audio.
final engineTransportProvider = FutureProvider<EngineTransport?>((ref) async {
  if (!isDesktopWindow) {
    return null;
  }
  final library = await ref.watch(libraryTransportProvider.future);
  try {
    final engine = await EngineTransport.start(
      libraryTransport: library,
      config: const EngineStartConfig(backend: 'auto'),
    );
    ref.keepAlive();
    ref.read(engineUiProvider.notifier).setRunning(true);
    return engine;
  } catch (e, st) {
    FlutterError.reportError(FlutterErrorDetails(exception: e, stack: st));
    fatalExit(1);
    rethrow;
  }
});

/// Long-lived engine evt subscription while the transport is open.
final engineEventsBootstrapProvider = Provider<void>((ref) {
  final transportAsync = ref.watch(engineTransportProvider);
  if (transportAsync case AsyncData(:final value) when value != null) {
    final sub = value.subscribeEvents().listen((evt) {
      ref.read(engineUiProvider.notifier).apply(evt);
    });
    ref.onDispose(sub.cancel);
  }
});

Future<void> loadPayloadToDeck(
  WidgetRef ref,
  int deckId,
  TrackDragPayload payload,
) async {
  final engine = await ref.read(engineTransportProvider.future);
  if (engine == null) {
    return;
  }
  await applyTrackDrop(
    deckId: deckId,
    payload: payload,
    loadLibraryTrack: (id, trackId) =>
        engine.loadLibraryTrack(deckId: id, trackId: trackId),
    loadPath: (id, path) => engine.loadPath(deckId: id, path: path),
  );
}
