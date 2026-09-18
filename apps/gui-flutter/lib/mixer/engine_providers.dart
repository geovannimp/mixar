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
import 'package:riverpod/src/providers/future_provider.dart';
import 'package:riverpod/src/providers/provider.dart';

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
      final shadow = evt.slipShadowPositionMs;
      if (id != null && shadow != null) {
        ref.read(deckSlipShadowsProvider.notifier).put(id, shadow);
      }
      return;
    }
    state = applyEngineEvt(state, evt);
    if (evt.kind == EngineEvtKind.updated && evt.deckId != null) {
      final id = evt.deckId!;
      // Unload before positionMs: authored null duration can still carry positionMs: 0.
      if (evt.durationKnown && evt.durationMs == null) {
        ref.read(deckPlayheadsProvider.notifier).remove(id);
        ref.read(deckSlipShadowsProvider.notifier).remove(id);
      } else if (evt.positionMs != null) {
        ref.read(deckPlayheadsProvider.notifier).put(id, evt.positionMs!);
      }
      if (evt.slipShadowPositionMs != null) {
        ref
            .read(deckSlipShadowsProvider.notifier)
            .put(id, evt.slipShadowPositionMs!);
      }
    }
  }

  void setRunning(bool running) => state = state.copyWith(running: running);

  void setDeckTrackPath(int deckId, String? path) {
    final next = Map<int, String>.from(state.trackPaths);
    if (path == null || path.isEmpty) {
      next.remove(deckId);
    } else {
      next[deckId] = path;
    }
    state = state.copyWith(trackPaths: next);
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

class DeckSlipShadows extends Notifier<Map<int, int>> {
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

final deckSlipShadowsProvider =
    NotifierProvider<DeckSlipShadows, Map<int, int>>(DeckSlipShadows.new);

final ProviderFamily<int, int> deckPositionMsProvider =
    Provider.family<int, int>(
      (ref, deckId) =>
          ref.watch(deckPlayheadsProvider.select((m) => m[deckId] ?? 0)),
    );

final ProviderFamily<String?, int> deckTrackIdProvider =
    Provider.family<String?, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.trackIdFor(deckId))),
    );

final ProviderFamily<int?, int> deckDurationMsProvider =
    Provider.family<int?, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.durationMsFor(deckId))),
    );

final ProviderFamily<double, int> deckSpeedRatioProvider =
    Provider.family<double, int>((ref, deckId) {
      final speed = ref.watch(
        engineUiProvider.select((s) => s.speedFor(deckId)),
      );
      final range = ref.watch(
        engineUiProvider.select((s) => s.tempoRangeFor(deckId)),
      );
      return normToSpeedRatio(speed, range);
    });

final engineRunningProvider = Provider<bool>(
  (ref) => ref.watch(engineUiProvider.select((s) => s.running)),
);

final ProviderFamily<String?, int> deckTrackTitleProvider =
    Provider.family<String?, int>((ref, deckId) {
      final lib = ref.watch(deckLibraryTrackProvider(deckId));
      final libTitle = lib?.title?.trim();
      if (libTitle != null && libTitle.isNotEmpty) {
        return libTitle;
      }
      if (lib != null) {
        final fromLibPath = fileStemFromPath(lib.path);
        if (fromLibPath.isNotEmpty) {
          return fromLibPath;
        }
      }
      final path = ref.watch(
        engineUiProvider.select((s) => s.trackPathFor(deckId)),
      );
      if (path == null || path.isEmpty) {
        return null;
      }
      final stem = fileStemFromPath(path);
      return stem.isEmpty ? null : stem;
    });

final ProviderFamily<bool, int> deckHasTrackProvider =
    Provider.family<bool, int>(
      (ref, deckId) => ref.watch(
        engineUiProvider.select(
          (s) =>
              s.durationMsFor(deckId) != null ||
              s.trackIdFor(deckId) != null ||
              s.trackPathFor(deckId) != null,
        ),
      ),
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

final ProviderFamily<bool, int> deckLoadingProvider =
    Provider.family<bool, int>(
      (ref, deckId) => ref.watch(
        deckLoadInFlightProvider.select((m) => (m[deckId] ?? 0) > 0),
      ),
    );

/// True while the engine is loading this deck, or its overview / beat grid
/// is still fetching after the track id lands.
final ProviderFamily<bool, int> deckSkeletonProvider =
    Provider.family<bool, int>((ref, deckId) {
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

final ProviderFamily<double?, int> deckBpmProvider =
    Provider.family<double?, int>((ref, deckId) {
      final trackId = ref.watch(deckTrackIdProvider(deckId));
      if (trackId == null) {
        return null;
      }
      return ref.watch(beatGridProvider(trackId))?.bpm;
    });

final ProviderFamily<bool, int> deckPlayingProvider =
    Provider.family<bool, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.isPlaying(deckId))),
    );

final ProviderFamily<PadMode, int> deckPadModeProvider =
    Provider.family<PadMode, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.padModeFor(deckId))),
    );

final ProviderFamily<String?, int> deckActiveSamplerBankIdProvider =
    Provider.family<String?, int>(
      (ref, deckId) => ref.watch(
        engineUiProvider.select((s) => s.activeSamplerBankIdFor(deckId)),
      ),
    );

final ProviderFamily<List<SamplerSlotChrome>, int> deckSamplerSlotsProvider =
    Provider.family<List<SamplerSlotChrome>, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.samplerSlotsFor(deckId))),
    );

final ProviderFamily<double, int> deckSpeedProvider =
    Provider.family<double, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.speedFor(deckId))),
    );

final ProviderFamily<double, int> deckTempoRangeProvider =
    Provider.family<double, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.tempoRangeFor(deckId))),
    );

final ProviderFamily<bool, int> deckKeyLockProvider =
    Provider.family<bool, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.keyLockFor(deckId))),
    );

final ProviderFamily<SyncMode, int> deckSyncModeProvider =
    Provider.family<SyncMode, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.syncModeFor(deckId))),
    );

final ProviderFamily<bool, int> deckIsMasterProvider =
    Provider.family<bool, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.isMaster(deckId))),
    );

final ProviderFamily<bool, int> deckQuantizeProvider =
    Provider.family<bool, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.quantizeFor(deckId))),
    );

final ProviderFamily<bool, int> deckSlipEnabledProvider =
    Provider.family<bool, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.slipEnabledFor(deckId))),
    );

final ProviderFamily<int?, int> deckSlipShadowMsProvider =
    Provider.family<int?, int>(
      (ref, deckId) =>
          ref.watch(deckSlipShadowsProvider.select((m) => m[deckId])),
    );

final ProviderFamily<bool, int> deckJogTouchingProvider =
    Provider.family<bool, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.jogTouchingFor(deckId))),
    );

final ProviderFamily<double?, int> deckLoudnessLufsProvider =
    Provider.family<double?, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.loudnessLufsFor(deckId))),
    );

final ProviderFamily<double, int> deckAutoGainDbProvider =
    Provider.family<double, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.autoGainDbFor(deckId))),
    );

final ProviderFamily<LibraryTrackSummary?, int> deckLibraryTrackProvider =
    Provider.family<LibraryTrackSummary?, int>((ref, deckId) {
      final id = ref.watch(deckTrackIdProvider(deckId));
      if (id == null) {
        return null;
      }
      // Prefer the visible table, then other in-memory lists, then a tab-stable
      // getTrack fetch — libraryTableTracksProvider is empty on History and may
      // omit the loaded track on Drive.
      return libraryTrackById(
            ref.watch(libraryTableTracksProvider).asData?.value,
            id,
          ) ??
          libraryTrackById(
            ref.watch(collectionTracksProvider).asData?.value,
            id,
          ) ??
          libraryTrackById(
            ref.watch(driveResolvedByPathProvider).asData?.value.values,
            id,
          ) ??
          ref.watch(libraryTrackByIdProvider(id)).asData?.value;
    });

/// Playing-deck key used for harmonic library coloring (master deck, then any playing deck).
final harmonicReferenceKeyProvider = Provider<String?>((ref) {
  if (!ref.watch(engineRunningProvider)) {
    return null;
  }
  String? keyForDeck(int deckId) {
    if (!ref.watch(deckPlayingProvider(deckId))) {
      return null;
    }
    final key = ref.watch(deckLibraryTrackProvider(deckId))?.key?.trim();
    if (key == null || key.isEmpty) {
      return null;
    }
    return key;
  }

  final master = ref.watch(engineUiProvider.select((s) => s.masterDeck));
  return keyForDeck(master) ?? keyForDeck(0) ?? keyForDeck(1);
});

/// Tab-stable library row for a track id (survives History / Drive switches).
final FutureProviderFamily<LibraryTrackSummary?, String>
libraryTrackByIdProvider = FutureProvider.family<LibraryTrackSummary?, String>((
  ref,
  trackId,
) async {
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

final ProviderFamily<List<DeckHotCue>, int> deckHotCuesProvider =
    Provider.family<List<DeckHotCue>, int>((ref, deckId) {
      final trackId = ref.watch(deckTrackIdProvider(deckId));
      if (trackId == null) {
        return const [];
      }
      final rows = ref.watch(trackHotCuesProvider.select((m) => m[trackId]));
      if (rows == null) {
        return const [];
      }
      return [
        for (final row in rows)
          DeckHotCue(
            slot: row.slot,
            positionMs: row.positionMs,
            label: row.label,
          ),
      ];
    });

final ProviderFamily<ActiveLoopInfo?, int> deckActiveLoopProvider =
    Provider.family<ActiveLoopInfo?, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.activeLoopFor(deckId))),
    );

final ProviderFamily<int?, int> deckPendingLoopInMsProvider =
    Provider.family<int?, int>(
      (ref, deckId) => ref.watch(
        engineUiProvider.select((s) => s.pendingLoopInMsFor(deckId)),
      ),
    );

final ProviderFamily<List<SavedLoopInfo>, int> deckSavedLoopsProvider =
    Provider.family<List<SavedLoopInfo>, int>((ref, deckId) {
      final trackId = ref.watch(deckTrackIdProvider(deckId));
      if (trackId == null) {
        return const [];
      }
      return ref.watch(trackSavedLoopsProvider.select((m) => m[trackId])) ??
          const [];
    });

final ProviderFamily<MixerChannelUi, int> deckMixerChannelProvider =
    Provider.family<MixerChannelUi, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.channelFor(deckId))),
    );

final ProviderFamily<DeckLevels, int> deckLevelsProvider =
    Provider.family<DeckLevels, int>(
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
    fatalExit();
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
    ref.read(engineUiProvider.notifier).setDeckTrackPath(deckId, payload.path);
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

Future<void> setDeckSlip(WidgetRef ref, int deckId, bool enabled) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setSlip(deckId: deckId, enabled: enabled);
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
  ref.read(engineUiProvider.notifier).setDeckTrackPath(deckId, null);
  ref.read(engineUiProvider.notifier).setDeckTrackId(deckId, null);
}

Future<void> pickTrackForDeck(WidgetRef ref, int deckId) async {
  final result = await FilePicker.pickFiles(
    type: FileType.custom,
    allowedExtensions: supportedAudioExtensions,
  );
  final path = result?.files.single.path;
  if (path == null || path.isEmpty) {
    return;
  }
  await loadPayloadToDeck(ref, deckId, payloadFromOsPath(path));
}
