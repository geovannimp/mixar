import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/mixer/level_meter.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';

class EngineUi extends Notifier<EngineUiSnapshot> {
  @override
  EngineUiSnapshot build() => EngineUiSnapshot.empty;

  void apply(EngineEvt evt) {
    if (evt.kind == EngineEvtKind.position) {
      final id = evt.deckId;
      final ms = evt.positionMs;
      if (id != null && ms != null) {
        ref.read(deckPlayheadsProvider.notifier).put(id, ms);
      }
      return;
    }
    state = applyEngineEvt(state, evt);
    if (evt.kind == EngineEvtKind.updated &&
        evt.deckId != null &&
        evt.positionMs != null) {
      ref
          .read(deckPlayheadsProvider.notifier)
          .put(evt.deckId!, evt.positionMs!);
    }
  }

  void setRunning(bool running) => state = state.copyWith(running: running);

  void setDeckTitle(int deckId, String title) {
    final next = Map<int, String>.from(state.titles);
    if (title.isEmpty) {
      next.remove(deckId);
    } else {
      next[deckId] = title;
    }
    state = state.copyWith(titles: next);
  }
}

final engineUiProvider = NotifierProvider<EngineUi, EngineUiSnapshot>(
  EngineUi.new,
);

class DeckPlayheads extends Notifier<Map<int, int>> {
  @override
  Map<int, int> build() => const {};

  void put(int deckId, int ms) {
    if (state[deckId] == ms) {
      return;
    }
    state = {...state, deckId: ms};
  }
}

final deckPlayheadsProvider = NotifierProvider<DeckPlayheads, Map<int, int>>(
  DeckPlayheads.new,
);

final deckPositionMsProvider = Provider.family<int, int>(
  (ref, deckId) => ref.watch(deckPlayheadsProvider)[deckId] ?? 0,
);

final deckTrackIdProvider = Provider.family<String?, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.trackIdFor(deckId))),
);

final deckDurationMsProvider = Provider.family<int?, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.durationMsFor(deckId))),
);

final deckSpeedRatioProvider = Provider.family<double, int>((ref, deckId) {
  final speed = ref.watch(engineUiProvider.select((s) => s.speedFor(deckId)));
  final range = ref.watch(
    engineUiProvider.select((s) => s.tempoRangeFor(deckId)),
  );
  return normToSpeedRatio(speed, range);
});

final engineRunningProvider = Provider<bool>(
  (ref) => ref.watch(engineUiProvider).running,
);

final deckTrackTitleProvider = Provider.family<String?, int>(
  (ref, deckId) => ref.watch(engineUiProvider).titleFor(deckId),
);

final deckPlayingProvider = Provider.family<bool, int>(
  (ref, deckId) => ref.watch(engineUiProvider).isPlaying(deckId),
);

final deckMixerChannelProvider = Provider.family<MixerChannelUi, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.channelFor(deckId))),
);

final deckLevelsProvider = Provider.family<DeckLevels, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.levelsFor(deckId))),
);

final crossfaderProvider = Provider<double>(
  (ref) => ref.watch(engineUiProvider.select((s) => s.crossfader)),
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
  ref
      .read(engineUiProvider.notifier)
      .setDeckTitle(
        deckId,
        trackDisplayTitle(title: payload.title, path: payload.path),
      );
}

Future<void> toggleDeckPlay(WidgetRef ref, int deckId) async {
  final engine = await ref.read(engineTransportProvider.future);
  if (engine == null) {
    return;
  }
  if (ref.read(engineUiProvider).isPlaying(deckId)) {
    await engine.pause(deckId: deckId);
  } else {
    await engine.play(deckId: deckId);
  }
}

Future<void> setDeckVolume(WidgetRef ref, int deckId, double volume) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setVolume(deckId: deckId, volume: volume);
}

Future<void> setDeckEqBand(
  WidgetRef ref,
  int deckId,
  EqBand band,
  double gain,
) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setEqBand(deckId: deckId, band: band, gain: gain);
}

Future<void> setDeckFilter(WidgetRef ref, int deckId, double filter) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setFilter(deckId: deckId, filter: filter);
}

Future<void> setDeckGainTrim(WidgetRef ref, int deckId, double gainTrim) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setGainTrim(deckId: deckId, gainTrim: gainTrim);
}

Future<void> setDeckHeadphoneCue(
  WidgetRef ref,
  int deckId,
  bool enabled,
) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setHeadphoneCue(deckId: deckId, enabled: enabled);
}

Future<void> setCrossfader(WidgetRef ref, double position) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setCrossfader(position: position);
}

Future<void> seekDeck(WidgetRef ref, int deckId, int positionMs) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.seek(deckId: deckId, positionMs: positionMs);
}
