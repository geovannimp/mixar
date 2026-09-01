import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/mixer/jog_ticks.dart';
import 'package:gui_flutter/mixer/level_meter.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' hide PadMode;
import 'package:gui_flutter/src/rust/api/library.dart'
    show LibraryTrackSummary, SavedLoopInfo;

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
    if (evt.kind == EngineEvtKind.updated && evt.deckId != null) {
      final id = evt.deckId!;
      // Unload before positionMs: authored null duration can still carry positionMs: 0.
      if (evt.durationKnown && evt.durationMs == null) {
        ref.read(deckPlayheadsProvider.notifier).remove(id);
      } else if (evt.positionMs != null) {
        ref.read(deckPlayheadsProvider.notifier).put(id, evt.positionMs!);
      }
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

  void setDeckTrackId(int deckId, String? trackId) {
    final next = Map<int, String>.from(state.trackIds);
    if (trackId == null || trackId.isEmpty) {
      next.remove(deckId);
    } else {
      next[deckId] = trackId;
    }
    state = state.copyWith(trackIds: next);
  }

  void setJogTouching(int deckId, bool touching) {
    if (state.jogTouchingFor(deckId) == touching) {
      return;
    }
    final next = Map<int, bool>.from(state.jogTouching);
    next[deckId] = touching;
    state = state.copyWith(jogTouching: next);
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

  void remove(int deckId) {
    if (!state.containsKey(deckId)) {
      return;
    }
    state = {...state}..remove(deckId);
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

/// Decks whose engine load is still in flight (drop/load started, not finished).
class DeckLoadInFlight extends Notifier<Map<int, int>> {
  @override
  Map<int, int> build() => const {};

  void set(int deckId, bool loading) {
    final n = state[deckId] ?? 0;
    if (loading) {
      state = {...state, deckId: n + 1};
      return;
    }
    if (n <= 1) {
      if (n == 0) {
        return;
      }
      state = {...state}..remove(deckId);
      return;
    }
    state = {...state, deckId: n - 1};
  }
}

final deckLoadInFlightProvider =
    NotifierProvider<DeckLoadInFlight, Map<int, int>>(DeckLoadInFlight.new);

final deckLoadingProvider = Provider.family<bool, int>(
  (ref, deckId) => (ref.watch(deckLoadInFlightProvider)[deckId] ?? 0) > 0,
);

/// True while the engine is loading this deck, or its overview / beat grid
/// is still fetching after the track id lands.
final deckSkeletonProvider = Provider.family<bool, int>((ref, deckId) {
  if (ref.watch(deckLoadingProvider(deckId))) {
    return true;
  }
  final trackId = ref.watch(deckTrackIdProvider(deckId));
  if (trackId == null) {
    return false;
  }
  return ref.watch(waveformOverviewProvider(trackId)).isLoading ||
      ref.watch(beatGridLoadingProvider(trackId));
});

final deckBpmProvider = Provider.family<double?, int>((ref, deckId) {
  final trackId = ref.watch(deckTrackIdProvider(deckId));
  if (trackId == null) {
    return null;
  }
  return ref.watch(beatGridProvider(trackId))?.bpm;
});

final deckPlayingProvider = Provider.family<bool, int>(
  (ref, deckId) => ref.watch(engineUiProvider).isPlaying(deckId),
);

final deckPadModeProvider = Provider.family<PadMode, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.padModeFor(deckId))),
);

final deckActiveSamplerBankIdProvider = Provider.family<String?, int>(
  (ref, deckId) => ref.watch(
    engineUiProvider.select((s) => s.activeSamplerBankIdFor(deckId)),
  ),
);

final deckSamplerSlotsProvider = Provider.family<List<SamplerSlotChrome>, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.samplerSlotsFor(deckId))),
);

final deckSpeedProvider = Provider.family<double, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.speedFor(deckId))),
);

final deckTempoRangeProvider = Provider.family<double, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.tempoRangeFor(deckId))),
);

final deckKeyLockProvider = Provider.family<bool, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.keyLockFor(deckId))),
);

final deckSyncModeProvider = Provider.family<SyncMode, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.syncModeFor(deckId))),
);

final deckIsMasterProvider = Provider.family<bool, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.isMaster(deckId))),
);

final deckQuantizeProvider = Provider.family<bool, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.quantizeFor(deckId))),
);

final deckJogTouchingProvider = Provider.family<bool, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.jogTouchingFor(deckId))),
);

final deckLoudnessLufsProvider = Provider.family<double?, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.loudnessLufsFor(deckId))),
);

final deckAutoGainDbProvider = Provider.family<double, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.autoGainDbFor(deckId))),
);

final deckLibraryTrackProvider = Provider.family<LibraryTrackSummary?, int>((
  ref,
  deckId,
) {
  final id = ref.watch(deckTrackIdProvider(deckId));
  if (id == null) {
    return null;
  }
  // Prefer the visible table, then other in-memory lists, then a tab-stable
  // getTrack fetch — libraryTableTracksProvider is empty on History and may
  // omit the loaded track on Drive.
  return libraryTrackById(ref.watch(libraryTableTracksProvider).asData?.value, id) ??
      libraryTrackById(ref.watch(collectionTracksProvider).asData?.value, id) ??
      libraryTrackById(
        ref.watch(driveResolvedByPathProvider).asData?.value?.values,
        id,
      ) ??
      ref.watch(libraryTrackByIdProvider(id)).asData?.value;
});

/// Playing-deck key used for harmonic library coloring (master deck, then any playing deck).
final harmonicReferenceKeyProvider = Provider<String?>((ref) {
  final ui = ref.watch(engineUiProvider);
  if (!ui.running) {
    return null;
  }
  String? keyForDeck(int deckId) {
    if (!ui.isPlaying(deckId)) {
      return null;
    }
    final key = ref.watch(deckLibraryTrackProvider(deckId))?.key?.trim();
    if (key == null || key.isEmpty) {
      return null;
    }
    return key;
  }

  return keyForDeck(ui.masterDeck) ?? keyForDeck(0) ?? keyForDeck(1);
});

/// Tab-stable library row for a track id (survives History / Drive switches).
final libraryTrackByIdProvider =
    FutureProvider.family<LibraryTrackSummary?, String>((ref, trackId) async {
      final transport = await ref.watch(libraryTransportProvider.future);
      return transport.getTrack(trackId: trackId);
    });

/// Find [id] in [tracks]; used by [deckLibraryTrackProvider] and tests.
LibraryTrackSummary? libraryTrackById(
  Iterable<LibraryTrackSummary>? tracks,
  String id,
) {
  if (tracks == null) {
    return null;
  }
  for (final track in tracks) {
    if (track.id == id) {
      return track;
    }
  }
  return null;
}

final deckHotCuesProvider = Provider.family<List<DeckHotCue>, int>((
  ref,
  deckId,
) {
  final trackId = ref.watch(deckTrackIdProvider(deckId));
  if (trackId == null) {
    return const [];
  }
  final rows = ref.watch(trackHotCuesProvider)[trackId];
  if (rows == null) {
    return const [];
  }
  return [
    for (final row in rows)
      DeckHotCue(slot: row.slot, positionMs: row.positionMs, label: row.label),
  ];
});

final deckActiveLoopProvider = Provider.family<ActiveLoopInfo?, int>(
  (ref, deckId) =>
      ref.watch(engineUiProvider.select((s) => s.activeLoopFor(deckId))),
);

final deckSavedLoopsProvider = Provider.family<List<SavedLoopInfo>, int>((
  ref,
  deckId,
) {
  final trackId = ref.watch(deckTrackIdProvider(deckId));
  if (trackId == null) {
    return const [];
  }
  return ref.watch(trackSavedLoopsProvider)[trackId] ?? const [];
});

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

final cueMixProvider = Provider<double>(
  (ref) => ref.watch(engineUiProvider.select((s) => s.cueMix)),
);

final masterCueProvider = Provider<bool>(
  (ref) => ref.watch(engineUiProvider.select((s) => s.masterCue)),
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
      if (evt.kind == EngineEvtKind.error) {
        ref
            .read(libraryMessageProvider.notifier)
            .setError(evt.message ?? 'Engine error');
      }
    });
    ref.onDispose(sub.cancel);
  }
});

Future<void> loadPayloadToDeck(
  WidgetRef ref,
  int deckId,
  TrackDragPayload payload,
) async {
  final loading = ref.read(deckLoadInFlightProvider.notifier);
  loading.set(deckId, true);
  try {
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
    ref.read(engineUiProvider.notifier).setDeckTrackId(deckId, payload.trackId);
  } finally {
    loading.set(deckId, false);
  }
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

Future<void> setCueMix(WidgetRef ref, double mix) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setCueMix(mix: mix);
}

Future<void> setMasterCue(WidgetRef ref, bool enabled) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setMasterCue(enabled: enabled);
}

Future<void> seekDeck(WidgetRef ref, int deckId, int positionMs) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.seek(deckId: deckId, positionMs: positionMs);
}

Future<void> setDeckSpeed(WidgetRef ref, int deckId, double speed) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setSpeed(deckId: deckId, speed: speed);
}

Future<void> setDeckTempoRange(
  WidgetRef ref,
  int deckId,
  double tempoRange,
) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setTempoRange(deckId: deckId, tempoRange: tempoRange);
}

Future<void> setDeckKeyLock(WidgetRef ref, int deckId, bool enabled) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setKeyLock(deckId: deckId, enabled: enabled);
}

Future<void> toggleDeckSync(
  WidgetRef ref,
  int deckId, {
  required bool beatSync,
}) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.toggleSync(deckId: deckId, beatSync: beatSync);
}

Future<void> setMasterDeck(WidgetRef ref, int deckId) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setMasterDeck(deckId: deckId);
}

Future<void> setDeckQuantize(WidgetRef ref, int deckId, bool enabled) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setQuantize(deckId: deckId, enabled: enabled);
}

Future<void> beginDeckCueHold(WidgetRef ref, int deckId) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.beginCueHold(deckId: deckId);
}

Future<void> endDeckCueHold(WidgetRef ref, int deckId) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.endCueHold(deckId: deckId);
}

Future<void> setDeckCuePoint(WidgetRef ref, int deckId) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setCuePoint(deckId: deckId);
}

Future<void> jogTouch(WidgetRef ref, int deckId, bool touching) async {
  // Eager so paused jogTurn can nudge the playhead before DeckUpdated lands.
  ref.read(engineUiProvider.notifier).setJogTouching(deckId, touching);
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.jogTouch(deckId: deckId, touching: touching);
}

Future<void> jogTurn(WidgetRef ref, int deckId, int delta) async {
  if (delta == 0) {
    return;
  }
  // Paused vinyl: playhead only moves in the audio callback, so Position can
  // lag. Nudge the UI playhead from ticks; engine Position corrects drift.
  if (!ref.read(deckPlayingProvider(deckId)) &&
      ref.read(deckJogTouchingProvider(deckId))) {
    final cur = ref.read(deckPositionMsProvider(deckId));
    ref
        .read(deckPlayheadsProvider.notifier)
        .put(deckId, cur + vinylTicksToDeltaMs(delta));
  }
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.jogTurn(deckId: deckId, delta: delta);
}

Future<void> unloadDeck(WidgetRef ref, int deckId) async {
  final engine = await ref.read(engineTransportProvider.future);
  if (engine == null) {
    return;
  }
  await engine.unload(deckId: deckId);
  ref.read(engineUiProvider.notifier).setDeckTitle(deckId, '');
  ref.read(engineUiProvider.notifier).setDeckTrackId(deckId, null);
}

Future<void> pickTrackForDeck(WidgetRef ref, int deckId) async {
  final result = await FilePicker.platform.pickFiles(
    type: FileType.custom,
    allowedExtensions: supportedAudioExtensions,
  );
  final path = result?.files.single.path;
  if (path == null || path.isEmpty) {
    return;
  }
  await loadPayloadToDeck(ref, deckId, payloadFromOsPath(path));
}
