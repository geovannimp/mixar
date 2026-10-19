import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart'
    show FutureProviderFamily, ProviderFamily;
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/fx_params.dart';
import 'package:gui_flutter/core/utils/jog_ticks.dart';
import 'package:gui_flutter/core/utils/tempo_format.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart'
    show LibraryTrackSummary, SavedLoopInfo;
import 'package:gui_flutter/widgets/mixer/level_meter.dart';
import 'package:gui_flutter/widgets/mixer/pads/hot_cue_pads.dart';

/// Mirrors that use optimistic absolute cmds + generation rollback.
///
/// `fxRouted` is keyed by deck id; `fxArmed` and `fxDivision` are keyed by FX
/// slot index because the unit is shared by every deck.
enum OptimisticField {
  playing,
  quantize,
  slip,
  keyLock,
  fxRouted,
  fxArmed,
  fxDivision,
}

class EngineUi extends Notifier<EngineUiSnapshot> {
  /// Per-field / per-deck generation so rollback only runs while our write owns
  /// the mirror. EngineEvt bumps the token and wins over a failed send.
  final Map<OptimisticField, Map<int, int>> _optimisticGen = {
    for (final field in OptimisticField.values) field: <int, int>{},
  };

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
      if (id != null) {
        if (shadow != null) {
          ref.read(deckSlipShadowsProvider.notifier).put(id, shadow);
        } else {
          ref.read(deckSlipShadowsProvider.notifier).remove(id);
        }
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
      } else {
        ref.read(deckSlipShadowsProvider.notifier).remove(id);
      }
      // External absolute state supersedes in-flight optimistic ownership.
      if (evt.playing != null) {
        _invalidateOptimistic(OptimisticField.playing, id);
      }
      if (evt.quantize != null) {
        _invalidateOptimistic(OptimisticField.quantize, id);
      }
      if (evt.slipEnabled != null) {
        _invalidateOptimistic(OptimisticField.slip, id);
      }
      if (evt.keyLock != null) {
        _invalidateOptimistic(OptimisticField.keyLock, id);
      }
      if (evt.fxRouted != null) {
        _invalidateOptimistic(OptimisticField.fxRouted, id);
      }
      final publishedFx = evt.fx;
      if (publishedFx != null) {
        // Any published slot config supersedes an in-flight arm command.
        for (var slot = 0; slot < publishedFx.length; slot++) {
          _invalidateOptimistic(OptimisticField.fxArmed, slot);
          _invalidateOptimistic(OptimisticField.fxDivision, slot);
        }
      }
    }
  }

  /// Claim ownership after an optimistic mirror write; returns a generation.
  int claimOptimistic(OptimisticField field, int deckId) {
    final map = _optimisticGen[field]!;
    final next = (map[deckId] ?? 0) + 1;
    map[deckId] = next;
    return next;
  }

  bool isOptimisticCurrent(OptimisticField field, int deckId, int generation) =>
      (_optimisticGen[field]![deckId] ?? 0) == generation;

  void _invalidateOptimistic(OptimisticField field, int deckId) {
    final map = _optimisticGen[field]!;
    map[deckId] = (map[deckId] ?? 0) + 1;
  }

  void setRunning(bool running) => state = state.copyWith(running: running);

  /// Optimistic write for a deck's FX routing switch; engine evts supersede it.
  void setFxRouted(int deckId, bool value) {
    final next = Map<int, MixerChannelUi>.from(state.channels);
    next[deckId] = state.channelFor(deckId).withFxRouted(value);
    state = state.copyWith(channels: next);
  }

  /// Optimistic write for the shared unit's arm flag (`slot` = chain index).
  ///
  /// No-op until a snapshot describes that slot, so the strip keeps showing the
  /// engine's state instead of inventing one.
  void setFxUnitArmed(int slot, bool value) {
    if (slot >= state.fxSlots.length) {
      return;
    }
    final current = state.fxSlots[slot];
    if (current.enabled == value) {
      return;
    }
    final next = List<FxSlotChrome>.from(state.fxSlots);
    next[slot] = FxSlotChrome(
      kind: current.kind,
      enabled: value,
      wet: current.wet,
      params: current.params,
    );
    state = state.copyWith(fxSlots: next);
  }

  /// Optimistic BEAT division (knob 0) for the shared unit's [slot].
  void setFxUnitDivisionNorm(int slot, double divisionNorm) {
    if (slot >= state.fxSlots.length) {
      return;
    }
    final current = state.fxSlots[slot];
    if (current.params.isEmpty || current.params[0] == divisionNorm) {
      return;
    }
    final params = Float32List.fromList(current.params)..[0] = divisionNorm;
    final next = List<FxSlotChrome>.from(state.fxSlots);
    next[slot] = FxSlotChrome(
      kind: current.kind,
      enabled: current.enabled,
      wet: current.wet,
      params: params,
    );
    state = state.copyWith(fxSlots: next);
  }

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

  /// Optimistic mirror before absolute play/pause; EngineEvt reconciles.
  void setPlaying(int deckId, bool playing) {
    if (state.isPlaying(deckId) == playing) {
      return;
    }
    final next = Map<int, bool>.from(state.playing);
    next[deckId] = playing;
    state = state.copyWith(playing: next);
  }

  /// Optimistic mirror before absolute setQuantize; EngineEvt reconciles.
  void setQuantize(int deckId, bool enabled) {
    if (state.quantizeFor(deckId) == enabled) {
      return;
    }
    final next = Map<int, bool>.from(state.quantize);
    next[deckId] = enabled;
    state = state.copyWith(quantize: next);
  }

  /// Optimistic mirror before absolute setSlip; EngineEvt reconciles.
  void setSlipEnabled(int deckId, bool enabled) {
    if (state.slipEnabledFor(deckId) == enabled) {
      return;
    }
    final next = Map<int, bool>.from(state.slipEnabled);
    next[deckId] = enabled;
    state = state.copyWith(slipEnabled: next);
  }

  /// Optimistic mirror before absolute setKeyLock; EngineEvt reconciles.
  void setKeyLock(int deckId, bool enabled) {
    if (state.keyLockFor(deckId) == enabled) {
      return;
    }
    final next = Map<int, bool>.from(state.keyLocks);
    next[deckId] = enabled;
    state = state.copyWith(keyLocks: next);
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

/// Per-deck monotonic load generation. Stale engine loads and chrome commits check this.
class DeckLoadGeneration extends Notifier<Map<int, int>> {
  @override
  Map<int, int> build() => const {};

  /// Begin a load attempt; returns its generation token.
  int begin(int deckId) {
    final next = (state[deckId] ?? 0) + 1;
    state = {...state, deckId: next};
    return next;
  }

  bool isCurrent(int deckId, int generation) => state[deckId] == generation;
}

final deckLoadGenerationProvider =
    NotifierProvider<DeckLoadGeneration, Map<int, int>>(DeckLoadGeneration.new);

/// Narrow engine ops for deck load — production wraps [EngineTransport]; tests
/// override [deckLoadTransportProvider] with a fake.
abstract class DeckLoadTransport {
  Future<void> loadLibraryTrack({required int deckId, required String trackId});

  Future<void> loadPath({required int deckId, required String path});
}

class _EngineDeckLoadTransport implements DeckLoadTransport {
  new(this._engine);

  final EngineTransport _engine;

  @override
  Future<void> loadLibraryTrack({
    required int deckId,
    required String trackId,
  }) => _engine.loadLibraryTrack(deckId: deckId, trackId: trackId);

  @override
  Future<void> loadPath({required int deckId, required String path}) =>
      _engine.loadPath(deckId: deckId, path: path);
}

final deckLoadTransportProvider = FutureProvider<DeckLoadTransport?>((
  ref,
) async {
  final engine = await ref.watch(engineTransportProvider.future);
  if (engine == null) {
    return null;
  }
  return _EngineDeckLoadTransport(engine);
});

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

final ProviderFamily<bool, int> deckStemsReadyProvider =
    Provider.family<bool, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.stemsReadyFor(deckId))),
    );

final ProviderFamily<bool, int> deckStemsGeneratingProvider =
    Provider.family<bool, int>((ref, deckId) {
      final ready = ref.watch(deckStemsReadyProvider(deckId));
      if (ready) return false;
      final trackId = ref.watch(deckTrackIdProvider(deckId));
      if (trackId == null || trackId.isEmpty) return false;
      return ref.watch(stemGeneratingTrackIdsProvider).contains(trackId);
    });

final ProviderFamily<List<bool>, int> deckStemMuteProvider =
    Provider.family<List<bool>, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.stemMuteFor(deckId))),
    );

final ProviderFamily<int?, int> deckStemIsolateProvider =
    Provider.family<int?, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.stemIsolateFor(deckId))),
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

final ProviderFamily<double, int> deckKeyShiftProvider =
    Provider.family<double, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.keyShiftFor(deckId))),
    );

final ProviderFamily<int, int> deckKeyboardPageProvider =
    Provider.family<int, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.keyboardPageFor(deckId))),
    );

final ProviderFamily<int, int> deckKeyShiftPageProvider =
    Provider.family<int, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.keyShiftPageFor(deckId))),
    );

final ProviderFamily<int, int> deckKeyboardRootProvider =
    Provider.family<int, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.keyboardRootFor(deckId))),
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

final ProviderFamily<bool, int> deckFxRoutedProvider =
    Provider.family<bool, int>(
      (ref, deckId) => ref.watch(
        engineUiProvider.select((s) => s.channelFor(deckId).fxRouted),
      ),
    );

final fxSlotsProvider = Provider<List<FxSlotChrome>>(
  (ref) => ref.watch(engineUiProvider.select((s) => s.fxSlots)),
);

final ProviderFamily<DeckLevels, int> deckLevelsProvider =
    Provider.family<DeckLevels, int>(
      (ref, deckId) =>
          ref.watch(engineUiProvider.select((s) => s.levelsFor(deckId))),
    );

final masterLevelsProvider = Provider<DeckLevels>(
  (ref) => ref.watch(engineUiProvider.select((s) => s.masterLevels)),
);

final crossfaderProvider = Provider<double>(
  (ref) => ref.watch(engineUiProvider.select((s) => s.crossfader)),
);

final masterVolumeProvider = Provider<double>(
  (ref) => ref.watch(engineUiProvider.select((s) => s.masterVolume)),
);

final cueVolumeProvider = Provider<double>(
  (ref) => ref.watch(engineUiProvider.select((s) => s.cueVolume)),
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
    final sub = value.subscribeEvents().listen(
      (evt) {
        ref.read(engineUiProvider.notifier).apply(evt);
        if (evt.kind == EngineEvtKind.error) {
          ref
              .read(appNoticeProvider.notifier)
              .setError(evt.message ?? 'Engine error');
        }
      },
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

/// Drive play/pause from an absolute desired state (never `!snapshot` after await).
Future<void> setDeckPlaying(WidgetRef ref, int deckId, bool playing) async {
  await _withOptimisticDeckBool(
    ref: ref,
    deckId: deckId,
    field: OptimisticField.playing,
    desired: playing,
    read: (snap) => snap.isPlaying(deckId),
    write: (ui, value) => ui.setPlaying(deckId, value),
    send: (engine) => sendDeckPlaying(
      deckId: deckId,
      playing: playing,
      play: engine.play,
      pause: engine.pause,
    ),
  );
}

/// Absolute play/pause dispatch — kept snapshot-free for dual-control races.
Future<void> sendDeckPlaying({
  required int deckId,
  required bool playing,
  required Future<void> Function({required int deckId}) play,
  required Future<void> Function({required int deckId}) pause,
}) async {
  if (playing) {
    await play(deckId: deckId);
  } else {
    await pause(deckId: deckId);
  }
}

/// Optimistic absolute cmd: mirror first, roll back if transport is missing
/// or the send fails — unless an intervening EngineEvt invalidated our
/// generation (MIDI dual-control), including when it set the same value.
///
/// If ownership is lost while awaiting the transport, skip [send] so a stale
/// UI intent cannot override a newer absolute controller/EngineEvt state.
@visibleForTesting
Future<void> runOptimisticDeckValue<V extends Object, T extends Object>({
  required V previous,
  required V desired,
  required void Function(V value) write,
  required int Function() claim,
  required bool Function(int generation) stillOurs,
  required Future<T?> Function() resolveEngine,
  required Future<void> Function(T engine) send,
}) async {
  write(desired);
  final generation = claim();
  try {
    final engine = await resolveEngine();
    if (engine == null) {
      if (stillOurs(generation)) {
        write(previous);
      }
      return;
    }
    if (!stillOurs(generation)) {
      return;
    }
    await send(engine);
  } on Object {
    if (stillOurs(generation)) {
      write(previous);
    }
    rethrow;
  }
}

/// [runOptimisticDeckValue] for the bool deck toggles.
@visibleForTesting
Future<void> runOptimisticDeckBool<T extends Object>({
  required bool previous,
  required bool desired,
  required void Function(bool value) write,
  required int Function() claim,
  required bool Function(int generation) stillOurs,
  required Future<T?> Function() resolveEngine,
  required Future<void> Function(T engine) send,
}) => runOptimisticDeckValue<bool, T>(
  previous: previous,
  desired: desired,
  write: write,
  claim: claim,
  stillOurs: stillOurs,
  resolveEngine: resolveEngine,
  send: send,
);

Future<void> _withOptimisticDeckValue<V extends Object>({
  required WidgetRef ref,
  required int deckId,
  required OptimisticField field,
  required V desired,
  required V Function(EngineUiSnapshot snap) read,
  required void Function(EngineUi ui, V value) write,
  required Future<void> Function(EngineTransport engine) send,
}) async {
  final previous = read(ref.read(engineUiProvider));
  await runOptimisticDeckValue<V, EngineTransport>(
    previous: previous,
    desired: desired,
    write: (value) => write(ref.read(engineUiProvider.notifier), value),
    claim: () =>
        ref.read(engineUiProvider.notifier).claimOptimistic(field, deckId),
    stillOurs: (generation) => ref
        .read(engineUiProvider.notifier)
        .isOptimisticCurrent(field, deckId, generation),
    resolveEngine: () => ref.read(engineTransportProvider.future),
    send: send,
  );
}

Future<void> _withOptimisticDeckBool({
  required WidgetRef ref,
  required int deckId,
  required OptimisticField field,
  required bool desired,
  required bool Function(EngineUiSnapshot snap) read,
  required void Function(EngineUi ui, bool value) write,
  required Future<void> Function(EngineTransport engine) send,
}) => _withOptimisticDeckValue<bool>(
  ref: ref,
  deckId: deckId,
  field: field,
  desired: desired,
  read: read,
  write: write,
  send: send,
);

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

Future<void> setDeckFxRouted(WidgetRef ref, int deckId, bool routed) async {
  await _withOptimisticDeckBool(
    ref: ref,
    deckId: deckId,
    field: OptimisticField.fxRouted,
    desired: routed,
    read: (snap) => snap.channelFor(deckId).fxRouted,
    write: (ui, value) => ui.setFxRouted(deckId, value),
    send: (engine) => engine.setDeckFxRouted(deckId: deckId, routed: routed),
  );
}

Future<void> setFxUnitSlot(WidgetRef ref, int slot, FxKind kind) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setFxUnitSlot(slot: slot, kind: kind);
}

Future<void> setFxUnitSlotEnabled(WidgetRef ref, int slot, bool enabled) async {
  // Shared unit: optimism is keyed by slot index rather than deck id.
  await _withOptimisticDeckBool(
    ref: ref,
    deckId: slot,
    field: OptimisticField.fxArmed,
    desired: enabled,
    read: (snap) => snap.fxSlots.length > slot && snap.fxSlots[slot].enabled,
    write: (ui, value) => ui.setFxUnitArmed(slot, value),
    send: (engine) => engine.setFxUnitSlotEnabled(slot: slot, enabled: enabled),
  );
}

/// Optimistic BEAT division step for the shared unit's [slot].
///
/// The target index is mirrored before the engine echo, so rapid ◀ ▶ taps
/// step the division instead of collapsing into the build-time index.
Future<void> setFxUnitDivision(WidgetRef ref, int slot, int index) async {
  final norm = fxDivisionNorm(index);
  await _withOptimisticDeckValue<int>(
    ref: ref,
    deckId: slot,
    field: OptimisticField.fxDivision,
    desired: index,
    read: (snap) => snap.fxSlots.length > slot
        ? fxDivisionIndexFrom(snap.fxSlots[slot].params)
        : index,
    write: (ui, value) => ui.setFxUnitDivisionNorm(slot, fxDivisionNorm(value)),
    send: (engine) => engine.setFxUnitParam(slot: slot, param: 0, value: norm),
  );
}

Future<void> setFxUnitWet(WidgetRef ref, int slot, double wet) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setFxUnitWet(slot: slot, wet: wet);
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

Future<void> setMasterVolume(WidgetRef ref, double volume) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setMasterVolume(volume: volume);
}

Future<void> setCueVolume(WidgetRef ref, double volume) async {
  final engine = await ref.read(engineTransportProvider.future);
  await engine?.setCueVolume(volume: volume);
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
  await _withOptimisticDeckBool(
    ref: ref,
    deckId: deckId,
    field: OptimisticField.keyLock,
    desired: enabled,
    read: (snap) => snap.keyLockFor(deckId),
    write: (ui, value) => ui.setKeyLock(deckId, value),
    send: (engine) => engine.setKeyLock(deckId: deckId, enabled: enabled),
  );
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
  await _withOptimisticDeckBool(
    ref: ref,
    deckId: deckId,
    field: OptimisticField.quantize,
    desired: enabled,
    read: (snap) => snap.quantizeFor(deckId),
    write: (ui, value) => ui.setQuantize(deckId, value),
    send: (engine) => engine.setQuantize(deckId: deckId, enabled: enabled),
  );
}

Future<void> setDeckSlip(WidgetRef ref, int deckId, bool enabled) async {
  await _withOptimisticDeckBool(
    ref: ref,
    deckId: deckId,
    field: OptimisticField.slip,
    desired: enabled,
    read: (snap) => snap.slipEnabledFor(deckId),
    write: (ui, value) => ui.setSlipEnabled(deckId, value),
    send: (engine) => engine.setSlip(deckId: deckId, enabled: enabled),
  );
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
