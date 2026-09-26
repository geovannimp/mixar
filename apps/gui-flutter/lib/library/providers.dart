import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/focused_load.dart';
import 'package:gui_flutter/library/history_refresh.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

/// Opens the app-support `library.db` (app id `top.mixar.app`).
final libraryTransportProvider = FutureProvider<LibraryTransport>((ref) async {
  final support = await getApplicationSupportDirectory();
  final dbPath = p.join(support.path, 'library.db');
  return LibraryTransport.open(dbPath: dbPath);
});

final collectionsProvider = FutureProvider<List<LibraryCollectionSummary>>((
  ref,
) async {
  final transport = await ref.watch(libraryTransportProvider.future);
  return transport.listCollections();
});

/// Explicit user selection; `null` means “use the first collection”.
class SelectedCollectionId extends Notifier<String?> {
  @override
  String? build() => null;

  void set(String? id) => state = id;
}

final selectedCollectionIdProvider =
    NotifierProvider<SelectedCollectionId, String?>(SelectedCollectionId.new);

/// Resolved selection: user pick if still present, otherwise the first collection.
final activeCollectionIdProvider = Provider<String?>((ref) {
  final selected = ref.watch(selectedCollectionIdProvider);
  final collections = ref.watch(collectionsProvider).asData?.value;
  if (collections == null || collections.isEmpty) {
    return null;
  }
  if (selected != null && collections.any((c) => c.id == selected)) {
    return selected;
  }
  return collections.first.id;
});

final collectionTracksProvider = FutureProvider<List<LibraryTrackSummary>>((
  ref,
) async {
  final id = ref.watch(activeCollectionIdProvider);
  if (id == null) {
    return const [];
  }
  final transport = await ref.watch(libraryTransportProvider.future);
  return transport.listCollectionEntries(collectionId: id);
});

class TrackFilter extends Notifier<String> {
  @override
  String build() => '';

  void set(String value) => state = value;
}

final trackFilterProvider = NotifierProvider<TrackFilter, String>(
  TrackFilter.new,
);

String trackTitleLabel(LibraryTrackSummary t) =>
    (t.title?.isNotEmpty ?? false) ? t.title! : t.displayName;

/// File tracks use the path as [LibraryTrackSummary.id], so `id != path` is not a library check.
bool trackIsInLibrary(
  LibraryTrackSummary t, {
  required LibrarySourceTab tab,
  Map<String, LibraryTrackSummary> driveResolvedByPath = const {},
}) {
  return switch (tab) {
    LibrarySourceTab.collections => true,
    LibrarySourceTab.drive => driveResolvedByPath.containsKey(t.path),
    LibrarySourceTab.history => false,
  };
}

final filteredTracksProvider = Provider<AsyncValue<List<LibraryTrackSummary>>>((
  ref,
) {
  final filter = ref.watch(trackFilterProvider).trim().toLowerCase();
  final tracks = ref.watch(collectionTracksProvider);
  return tracks.whenData((list) {
    if (filter.isEmpty) {
      return list;
    }
    return [
      for (final t in list)
        if (trackTitleLabel(t).toLowerCase().contains(filter) ||
            (t.artist ?? '').toLowerCase().contains(filter))
          t,
    ];
  });
});

// --- Library events (Task 3) ---

class AnalyzingTrackIds extends Notifier<Set<String>> {
  final Map<String, Timer> _stuckClears = {};

  @override
  Set<String> build() {
    ref.onDispose(() {
      for (final timer in _stuckClears.values) {
        timer.cancel();
      }
    });
    return const {};
  }

  void add(String id) {
    _stuckClears.remove(id)?.cancel();
    state = {...state, id};
    // ponytail: clear stuck spinner if evt never arrives. Upgrade: correlate cmd/evt ids.
    // Stems + Demucs can exceed a minute; keep the loader until stems_ready when enabled.
    _stuckClears[id] = Timer(const Duration(minutes: 30), () {
      clearIf(id);
    });
  }

  void clearIf(String? trackId) {
    if (trackId == null || !state.contains(trackId)) {
      return;
    }
    _stuckClears.remove(trackId)?.cancel();
    final next = {...state}..remove(trackId);
    state = next;
  }

  void clear() {
    for (final timer in _stuckClears.values) {
      timer.cancel();
    }
    _stuckClears.clear();
    state = const {};
  }
}

final analyzingTrackIdsProvider =
    NotifierProvider<AnalyzingTrackIds, Set<String>>(AnalyzingTrackIds.new);

/// Coarse analyze/stems phase label + optional fraction per track.
class TrackProgressInfo {
  const TrackProgressInfo({required this.phase, this.fraction});

  final String phase;
  final double? fraction;

  String get label {
    final pct = fraction == null
        ? null
        : '${(fraction!.clamp(0.0, 1.0) * 100).round()}%';
    final base = switch (phase) {
      'decode' => 'Decoding',
      'analyze' => 'Analyzing',
      'bpm' => 'Detecting BPM',
      'key' => 'Detecting key',
      'loudness' => 'Measuring loudness',
      'waveform' => 'Waveform',
      'stems_queued' => 'Queuing stems',
      'stems_model' => 'Loading stem model',
      'stems_separate' => 'Separating stems',
      'stems_encode' => 'Encoding stems',
      'stems_ready' => 'Stems ready',
      'stems_failed' => 'Stems failed',
      _ => phase,
    };
    return pct == null ? base : '$base $pct';
  }
}

class TrackProgressMap extends Notifier<Map<String, TrackProgressInfo>> {
  @override
  Map<String, TrackProgressInfo> build() => const {};

  void set(String trackId, String phase, double? fraction) {
    if (phase == 'stems_ready' || phase == 'stems_failed') {
      final next = {...state}..remove(trackId);
      state = next;
      return;
    }
    final existing = state[trackId];
    // Analyze runs in parallel with stems; keep stems_* label visible.
    if (existing != null &&
        _isStemProgressPhase(existing.phase) &&
        !_isStemProgressPhase(phase)) {
      return;
    }
    state = {
      ...state,
      trackId: TrackProgressInfo(phase: phase, fraction: fraction),
    };
  }

  void clearIf(String? trackId) {
    if (trackId == null || !state.containsKey(trackId)) return;
    final next = {...state}..remove(trackId);
    state = next;
  }
}

final trackProgressProvider =
    NotifierProvider<TrackProgressMap, Map<String, TrackProgressInfo>>(
      TrackProgressMap.new,
    );

/// Track IDs with an in-flight stem job (for pad banner).
class StemGeneratingTrackIds extends Notifier<Set<String>> {
  @override
  Set<String> build() => const {};

  void setGenerating(String trackId, bool generating) {
    if (generating) {
      if (state.contains(trackId)) return;
      state = {...state, trackId};
    } else {
      if (!state.contains(trackId)) return;
      state = {...state}..remove(trackId);
    }
  }
}

final stemGeneratingTrackIdsProvider =
    NotifierProvider<StemGeneratingTrackIds, Set<String>>(
      StemGeneratingTrackIds.new,
    );

class LibraryMessage extends Notifier<String?> {
  @override
  String? build() => null;

  void clear() => state = null;

  void setError(String message) => state = message;

  void setNotice(String? message) => state = message;
}

final libraryMessageProvider = NotifierProvider<LibraryMessage, String?>(
  LibraryMessage.new,
);

class LibraryAnalysisEpoch extends Notifier<int> {
  @override
  int build() => 0;

  void bump() => state++;
}

final libraryAnalysisEpochProvider =
    NotifierProvider<LibraryAnalysisEpoch, int>(LibraryAnalysisEpoch.new);

class TrackHotCues extends Notifier<Map<String, List<HotCueInfo>>> {
  @override
  Map<String, List<HotCueInfo>> build() => const {};

  void set(String trackId, List<HotCueInfo> cues) {
    state = {...state, trackId: cues};
  }
}

final trackHotCuesProvider =
    NotifierProvider<TrackHotCues, Map<String, List<HotCueInfo>>>(
      TrackHotCues.new,
    );

class TrackSavedLoops extends Notifier<Map<String, List<SavedLoopInfo>>> {
  @override
  Map<String, List<SavedLoopInfo>> build() => const {};

  void set(String trackId, List<SavedLoopInfo> loops) {
    state = {...state, trackId: loops};
  }
}

final trackSavedLoopsProvider =
    NotifierProvider<TrackSavedLoops, Map<String, List<SavedLoopInfo>>>(
      TrackSavedLoops.new,
    );

class TrackBeatGrids extends Notifier<Map<String, BeatGridData?>> {
  @override
  Map<String, BeatGridData?> build() => const {};

  void set(String trackId, BeatGridData? grid) {
    state = {...state, trackId: grid};
  }

  void remove(String trackId) {
    if (!state.containsKey(trackId)) {
      return;
    }
    final next = Map<String, BeatGridData?>.from(state);
    next.remove(trackId);
    state = next;
  }
}

final trackBeatGridsProvider =
    NotifierProvider<TrackBeatGrids, Map<String, BeatGridData?>>(
      TrackBeatGrids.new,
    );

class FocusedTrackRowIndex extends Notifier<int> {
  var _count = 0;

  @override
  int build() => 0;

  void setCount(int count) {
    _count = count < 0 ? 0 : count;
    if (_count == 0) {
      state = 0;
      return;
    }
    if (state >= _count) {
      state = _count - 1;
    }
  }

  void set(int index) {
    if (_count == 0) {
      state = 0;
      return;
    }
    final next = index < 0 ? 0 : (index >= _count ? _count - 1 : index);
    if (state != next) {
      state = next;
    }
  }

  void navigate(int delta) {
    state = navigateIndex(state, _count, delta);
  }
}

final focusedTrackRowIndexProvider =
    NotifierProvider<FocusedTrackRowIndex, int>(FocusedTrackRowIndex.new);

bool _isStemProgressPhase(String phase) =>
    phase == 'decode' || phase.startsWith('stems_');

bool _stemsStillRunning(Ref ref, String trackId) {
  if (ref.read(stemGeneratingTrackIdsProvider).contains(trackId)) {
    return true;
  }
  final progress = ref.read(trackProgressProvider)[trackId];
  return progress != null && _isStemProgressPhase(progress.phase);
}

void _handleLibraryEvt(Ref ref, LibraryEvt evt) {
  switch (evt.kind) {
    case LibraryEvtKind.trackUpdated:
    case LibraryEvtKind.trackAnalyzed:
      ref.invalidate(collectionTracksProvider);
      ref.invalidate(collectionsProvider);
      if (evt.kind == LibraryEvtKind.trackAnalyzed) {
        final trackId = evt.trackId ?? evt.track?.id;
        final stemsWaiting =
            trackId != null && _stemsStillRunning(ref, trackId);
        if (!stemsWaiting) {
          ref.read(analyzingTrackIdsProvider.notifier).clearIf(trackId);
        }
        if (trackId != null) {
          ref.read(trackBeatGridsProvider.notifier).remove(trackId);
          final progress = ref.read(trackProgressProvider)[trackId];
          if (progress != null &&
              !progress.phase.startsWith('stems_') &&
              progress.phase != 'decode') {
            ref.read(trackProgressProvider.notifier).clearIf(trackId);
          }
        }
        ref.read(libraryAnalysisEpochProvider.notifier).bump();
      }
    case LibraryEvtKind.trackProgress:
      final trackId = evt.trackId;
      final phase = evt.phase;
      if (trackId == null || phase == null) {
        break;
      }
      ref
          .read(trackProgressProvider.notifier)
          .set(trackId, phase, evt.fraction);
      final generating =
          phase == 'stems_queued' ||
          phase == 'decode' ||
          phase == 'stems_model' ||
          phase == 'stems_separate' ||
          phase == 'stems_encode';
      final done = phase == 'stems_ready' || phase == 'stems_failed';
      if (generating) {
        ref
            .read(stemGeneratingTrackIdsProvider.notifier)
            .setGenerating(trackId, true);
      } else if (done) {
        ref
            .read(stemGeneratingTrackIdsProvider.notifier)
            .setGenerating(trackId, false);
        ref.read(analyzingTrackIdsProvider.notifier).clearIf(trackId);
      }
    case LibraryEvtKind.error:
      ref
          .read(libraryMessageProvider.notifier)
          .setError(evt.message ?? 'Error');
      if (evt.trackId != null) {
        ref.read(analyzingTrackIdsProvider.notifier).clearIf(evt.trackId);
        ref
            .read(stemGeneratingTrackIdsProvider.notifier)
            .setGenerating(evt.trackId!, false);
        ref.read(trackProgressProvider.notifier).clearIf(evt.trackId);
      } else {
        ref.read(analyzingTrackIdsProvider.notifier).clear();
      }
    case LibraryEvtKind.notice:
      ref.read(libraryMessageProvider.notifier).setNotice(evt.message);
    case LibraryEvtKind.hotCuesChanged:
      final trackId = evt.trackId;
      if (trackId != null) {
        ref
            .read(trackHotCuesProvider.notifier)
            .set(trackId, evt.hotCues ?? const []);
      }
    case LibraryEvtKind.loopsChanged:
      final trackId = evt.trackId;
      if (trackId != null) {
        ref
            .read(trackSavedLoopsProvider.notifier)
            .set(trackId, evt.loops ?? const []);
      }
    case LibraryEvtKind.beatGridChanged:
      final trackId = evt.trackId;
      if (trackId != null) {
        ref.read(trackBeatGridsProvider.notifier).set(trackId, evt.beatGrid);
      }
    case LibraryEvtKind.navigate:
      ref.read(focusedTrackRowIndexProvider.notifier).navigate(evt.delta ?? 0);
    case LibraryEvtKind.load:
      final deck = evt.deck;
      if (deck != null) {
        unawaited(loadFocusedRowToDeck(ref, deck));
      }
    case LibraryEvtKind.historySessionUpdated:
      ref.read(historyRefreshTickProvider.notifier).bump();
  }
}

/// MIDI / controller load: focused table row → deck via [focusedLoadPayload].
Future<void> loadFocusedRowToDeck(Ref ref, int deckId) async {
  final tracks = ref.read(libraryTableTracksProvider).asData?.value;
  if (tracks == null || tracks.isEmpty) {
    return;
  }
  final index = ref.read(focusedTrackRowIndexProvider);
  final tab = ref.read(librarySourceTabProvider);
  final resolved =
      ref.read(driveResolvedByPathProvider).asData?.value ?? const {};
  final payload = focusedLoadPayload(
    tracks,
    index,
    inLibrary: (t) =>
        trackIsInLibrary(t, tab: tab, driveResolvedByPath: resolved),
  );
  if (payload == null) {
    return;
  }

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

/// Long-lived `subscribeEvents` while transport is open; watch from library UI.
final libraryEventsBootstrapProvider = Provider<void>((ref) {
  final transportAsync = ref.watch(libraryTransportProvider);
  if (transportAsync case AsyncData(:final value)) {
    final sub = value.subscribeEvents().listen((evt) {
      _handleLibraryEvt(ref, evt);
    });
    ref.onDispose(sub.cancel);
  }
});

Future<void> analyzeTrackAction(WidgetRef ref, String trackId) async {
  ref.read(analyzingTrackIdsProvider.notifier).add(trackId);
  ref.read(libraryMessageProvider.notifier).clear();
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    await transport.analyzeTrack(trackId: trackId, force: false);
  } catch (e) {
    ref.read(analyzingTrackIdsProvider.notifier).clearIf(trackId);
    ref.read(libraryMessageProvider.notifier).setError('$e');
  }
}

/// Queue stem generation for one track.
///
/// Deliberately independent of [analyzeTrackAction]: a full Demucs pass is far
/// more expensive than analysis, so it is opt-in per track. Server-side this
/// no-ops when a valid stem cache already exists.
Future<void> generateStemsAction(WidgetRef ref, String trackId) async {
  ref.read(libraryMessageProvider.notifier).clear();
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    await transport.generateStems(trackId: trackId);
  } catch (e) {
    ref.read(libraryMessageProvider.notifier).setError('$e');
  }
}

Future<void> refreshTrackAction(WidgetRef ref, String trackId) async {
  ref.read(libraryMessageProvider.notifier).clear();
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    await transport.refreshTrack(trackId: trackId);
  } catch (e) {
    ref.read(libraryMessageProvider.notifier).setError('$e');
  }
}

// --- Drive browse (Task 6) ---

enum LibrarySourceTab { collections, drive, history }

class LibrarySourceTabNotifier extends Notifier<LibrarySourceTab> {
  @override
  LibrarySourceTab build() => LibrarySourceTab.collections;

  void set(LibrarySourceTab tab) => state = tab;
}

final librarySourceTabProvider =
    NotifierProvider<LibrarySourceTabNotifier, LibrarySourceTab>(
      LibrarySourceTabNotifier.new,
    );

final driveVolumesProvider = FutureProvider<List<FsVolumeInfo>>(
  (ref) => listFsVolumes(),
);

class DriveCurrentPath extends Notifier<String?> {
  @override
  String? build() => null;

  void set(String? path) {
    if (state == path) {
      return;
    }
    state = path;
  }
}

final driveCurrentPathProvider = NotifierProvider<DriveCurrentPath, String?>(
  DriveCurrentPath.new,
);

final driveListingProvider = FutureProvider<FsDirectoryListing?>((ref) async {
  final path = ref.watch(driveCurrentPathProvider);
  if (path == null) {
    return null;
  }
  return browseFsDirectory(path: path);
});

/// Longest matching volume root for [driveCurrentPathProvider] (Tauri `findActiveVolume`).
final driveActiveVolumeProvider = Provider<FsVolumeInfo?>((ref) {
  final path = ref.watch(driveCurrentPathProvider);
  final volumes = ref.watch(driveVolumesProvider).asData?.value;
  if (path == null || volumes == null || volumes.isEmpty) {
    return null;
  }
  final sorted = [...volumes]
    ..sort((a, b) => b.path.length.compareTo(a.path.length));
  for (final volume in sorted) {
    if (path == volume.path) {
      return volume;
    }
    if (volume.path != '/' && path.startsWith('${volume.path}/')) {
      return volume;
    }
    if (volume.path == '/' && path.startsWith('/')) {
      return volume;
    }
  }
  return null;
});

final driveResolvedByPathProvider =
    FutureProvider<Map<String, LibraryTrackSummary>>((ref) async {
      final listing = await ref.watch(driveListingProvider.future);
      if (listing == null || listing.audioFiles.isEmpty) {
        return const {};
      }
      final transport = await ref.read(libraryTransportProvider.future);
      final resolved = await transport.resolveTracksForPaths(
        paths: [for (final f in listing.audioFiles) f.path],
      );
      return {for (final r in resolved) r.requestPath: r.track};
    });

LibraryTrackSummary _driveFileSummary(
  FsEntry file,
  Map<String, LibraryTrackSummary> byPath,
) {
  return byPath[file.path] ??
      LibraryTrackSummary(
        id: file.path,
        displayName: file.name,
        path: file.path,
      );
}

/// Drive audio files as table rows (library metadata when resolved).
final driveTableTracksProvider =
    Provider<AsyncValue<List<LibraryTrackSummary>>>((ref) {
      final path = ref.watch(driveCurrentPathProvider);
      if (path == null) {
        return const AsyncData([]);
      }
      final listing = ref.watch(driveListingProvider);
      final byPath =
          ref.watch(driveResolvedByPathProvider).asData?.value ??
          const <String, LibraryTrackSummary>{};
      final filter = ref.watch(trackFilterProvider).trim().toLowerCase();
      return listing.when(
        loading: () => const AsyncLoading(),
        error: AsyncError.new,
        data: (dir) {
          if (dir == null) {
            return const AsyncData([]);
          }
          final tracks = [
            for (final f in dir.audioFiles) _driveFileSummary(f, byPath),
          ];
          if (filter.isEmpty) {
            return AsyncData(tracks);
          }
          return AsyncData([
            for (final t in tracks)
              if (trackTitleLabel(t).toLowerCase().contains(filter) ||
                  (t.artist ?? '').toLowerCase().contains(filter))
                t,
          ]);
        },
      );
    });

/// Right-pane rows: collection tracks or drive files, depending on the tab.
final libraryTableTracksProvider =
    Provider<AsyncValue<List<LibraryTrackSummary>>>((ref) {
      switch (ref.watch(librarySourceTabProvider)) {
        case LibrarySourceTab.collections:
          return ref.watch(filteredTracksProvider);
        case LibrarySourceTab.drive:
          return ref.watch(driveTableTracksProvider);
        case LibrarySourceTab.history:
          return const AsyncData([]);
      }
    });
