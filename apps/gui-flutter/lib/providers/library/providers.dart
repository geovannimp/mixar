import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/color_labels.dart';
import 'package:gui_flutter/core/utils/folder_path.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/library/focused_load.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/providers/library/history_refresh.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/providers/shell/bus_providers.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:path/path.dart' as p;

/// Opens the app-support `library.db`; Rust resolves the directory.
///
/// The bridge call is sync, so it sits behind a `Future`: an uninitialised Rust
/// side (widget tests) then surfaces as a provider error the way the previous
/// platform-channel lookup did, instead of throwing out of `create()`.
final libraryTransportProvider = FutureProvider<LibraryTransport>((ref) async {
  final bus = await ref.watch(busTransportProvider.future);
  return LibraryTransport.open(bus: bus);
});

final collectionsProvider = FutureProvider<List<LibraryCollectionSummary>>((
  ref,
) async {
  final transport = await ref.watch(libraryTransportProvider.future);
  return transport.listCollections();
});

/// Folder paths that already back a collection.
///
/// One folder backs at most one collection, so the drive pane hides the
/// “Create collection” action for these paths instead of asking the library
/// to add a duplicate.
final collectedFolderPathsProvider = Provider<Set<String>>((ref) {
  final collections = ref.watch(collectionsProvider).asData?.value;
  if (collections == null) {
    return const {};
  }
  return {
    for (final collection in collections)
      if (collection.kind == 'folder' && collection.path != null)
        normalizeFolderPath(collection.path!),
  };
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

/// Collection track rows: SQLite load plus in-memory patch on trackUpdated.
///
/// Metadata events patch the loaded list when the row is already present.
/// Membership/structural changes (missing track payload, or id not in the
/// list) invalidate and reload. Chatty bursts coalesce to one microtask flush.
class CollectionTracks extends AsyncNotifier<List<LibraryTrackSummary>> {
  Map<String, LibraryTrackSummary> _pendingPatches = {};
  bool _pendingStructuralInvalidate = false;
  bool _flushScheduled = false;

  /// Active collection when the pending burst was first queued. Dropped on
  /// flush if the user switched collections before the microtask ran.
  String? _pendingForCollectionId;

  /// Membership set for O(1) patch vs structural decisions.
  Set<String> _trackIds = {};

  void _setTrackIds(List<LibraryTrackSummary> tracks) {
    _trackIds = {for (final t in tracks) t.id};
  }

  /// Drop any coalesced burst queued before this rebuild. A later microtask
  /// `_flush` becomes a no-op; the fresh load (or a new event) is authoritative.
  void _clearPendingBurst() {
    _pendingPatches = {};
    _pendingStructuralInvalidate = false;
    _pendingForCollectionId = null;
  }

  @override
  Future<List<LibraryTrackSummary>> build() async {
    _clearPendingBurst();
    final id = ref.watch(activeCollectionIdProvider);
    if (id == null) {
      _setTrackIds(const []);
      return const [];
    }
    final transport = await ref.watch(libraryTransportProvider.future);
    final tracks = await transport.listCollectionEntries(collectionId: id);
    _setTrackIds(tracks);
    return tracks;
  }

  /// Apply a [LibraryEvtKind.trackUpdated] / [LibraryEvtKind.trackAnalyzed] row.
  void scheduleTrackRowEvent(LibraryTrackSummary? track) {
    var queued = false;
    if (track == null) {
      // Membership/structural signal with no row payload.
      _pendingStructuralInvalidate = true;
      queued = true;
    } else {
      final current = state.asData?.value;
      if (current == null) {
        // Loading or errored: no list to patch. Skip metadata events so a
        // failing reload + chatty analyze cannot invalidateSelf in a loop.
        // A later successful load reads the already-written SQLite row.
      } else if (_trackIds.contains(track.id)) {
        _pendingPatches[track.id] = track;
        queued = true;
      } else {
        _pendingStructuralInvalidate = true;
        queued = true;
      }
    }
    if (!queued) {
      return;
    }
    _pendingForCollectionId ??= ref.read(activeCollectionIdProvider);
    if (_flushScheduled) {
      return;
    }
    _flushScheduled = true;
    scheduleMicrotask(() {
      _flushScheduled = false;
      if (!ref.mounted) {
        return;
      }
      _flush();
    });
  }

  void _flush() {
    final structural = _pendingStructuralInvalidate;
    final patches = _pendingPatches;
    final scheduledFor = _pendingForCollectionId;
    _pendingStructuralInvalidate = false;
    _pendingPatches = {};
    _pendingForCollectionId = null;

    // Collection switched before flush: drop the burst. The new list load
    // (or a later event) picks up SQLite state for the active collection.
    if (scheduledFor != ref.read(activeCollectionIdProvider)) {
      return;
    }

    if (structural) {
      ref
        ..invalidateSelf()
        ..invalidate(collectionsProvider);
      return;
    }
    if (patches.isEmpty) {
      return;
    }
    final current = state.asData?.value;
    if (current == null) {
      // List disappeared before flush (reload/error); drop patches rather
      // than kick another invalidate that could thrash on persistent failure.
      return;
    }
    final next = [for (final t in current) patches[t.id] ?? t];
    _setTrackIds(next);
    state = AsyncData(next);
  }
}

final collectionTracksProvider =
    AsyncNotifierProvider<CollectionTracks, List<LibraryTrackSummary>>(
      CollectionTracks.new,
    );

/// Test override that supplies a custom load while keeping patch/coalesce.
///
/// Watches [activeCollectionIdProvider] so a collection switch rebuilds like
/// production — callers must also override [collectionsProvider] (or
/// [activeCollectionIdProvider]) so that watch does not open a real DB.
/// Structural [_flush] still invalidates [collectionsProvider] for production
/// parity (track counts); seeded tests that count loads should await
/// [collectionsProvider] first so the active id is stable before the first
/// track load.
@visibleForTesting
class SeededCollectionTracks extends CollectionTracks {
  SeededCollectionTracks(this._load);

  final Future<List<LibraryTrackSummary>> Function(Ref ref) _load;

  @override
  Future<List<LibraryTrackSummary>> build() async {
    _clearPendingBurst();
    // Subscribe even if [_load] ignores the id, matching production rebuilds.
    ref.watch(activeCollectionIdProvider);
    final tracks = await _load(ref);
    _setTrackIds(tracks);
    return tracks;
  }
}

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

/// Normalizes the filter box text once per list rebuild.
String normalizeTrackFilter(String filter) => filter.trim().toLowerCase();

/// Whether [track] matches an already-normalized [needle].
///
/// Searches the text fields a user would type plus the numeric/colour fields
/// the list can now show; an empty needle matches everything. Shared by the
/// collection and drive tables so both tabs filter identically.
bool trackMatchesNeedle(LibraryTrackSummary track, String needle) {
  if (needle.isEmpty) {
    return true;
  }
  bool has(String? value) => (value ?? '').toLowerCase().contains(needle);
  return has(trackTitleLabel(track)) ||
      has(track.artist) ||
      has(track.album) ||
      has(track.genre) ||
      has(track.comment) ||
      has(track.colorLabel?.name) ||
      has(track.year?.toString()) ||
      has(track.trackNumber?.toString()) ||
      has(track.discNumber?.toString());
}

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
  final needle = normalizeTrackFilter(ref.watch(trackFilterProvider));
  final tracks = ref.watch(collectionTracksProvider);
  return tracks.whenData(
    (list) => [
      for (final t in list)
        if (trackMatchesNeedle(t, needle)) t,
    ],
  );
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
    // shortcut: clear stuck spinner if evt never arrives. Upgrade: correlate cmd/evt ids.
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
  const new({required this.phase, this.fraction});

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

/// In-flight jobs per track, one entry per lane.
///
/// Analysis and stem separation run independently and can overlap on the same
/// track, so each lane keeps its own phase instead of overwriting the other.
class TrackProgressMap extends Notifier<Map<String, List<TrackProgressInfo>>> {
  @override
  Map<String, List<TrackProgressInfo>> build() => const {};

  void set(String trackId, String phase, double? fraction) {
    final lane = isStemProgressPhase(phase);
    if (phase == 'stems_ready' || phase == 'stems_failed') {
      _drop(trackId, lane: lane);
      return;
    }
    // Engine is the untrusted source. A non-finite ratio would otherwise
    // render as a false "100%", since num.clamp maps NaN to the upper bound.
    final info = TrackProgressInfo(
      phase: phase,
      fraction: fraction != null && fraction.isFinite ? fraction : null,
    );
    final existing = state[trackId];
    // Replace the lane in place so an update to one lane does not reorder the
    // other, which would churn the list identity for no visible reason.
    final next = [...?existing];
    final idx = next.indexWhere((i) => isStemProgressPhase(i.phase) == lane);
    if (idx >= 0) {
      next[idx] = info;
    } else {
      next.add(info);
    }
    // The engine re-reports an unchanged phase: `ensure_track_stems_with_progress`
    // emits `stems_separate` with no fraction from both the pre-call report and
    // the first progress tick. Skipping the write keeps the list identity, so
    // the row overlay's select suppresses the rebuild instead of re-rendering
    // a pill for an event that changes nothing.
    if (existing != null && _sameJobs(existing, next)) return;
    state = {...state, trackId: next};
  }

  /// Value equality for lane entries; [TrackProgressInfo] has no `==`.
  static bool _sameJobs(List<TrackProgressInfo> a, List<TrackProgressInfo> b) {
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (a[i].phase != b[i].phase || a[i].fraction != b[i].fraction) {
        return false;
      }
    }
    return true;
  }

  /// Clear every lane for [trackId].
  void clearIf(String? trackId) => _drop(trackId);

  /// Clear one lane, leaving a concurrent job on the other lane running.
  void clearLane(String? trackId, {required bool lane}) =>
      _drop(trackId, lane: lane);

  void _drop(String? trackId, {bool? lane}) {
    if (trackId == null) return;
    final existing = state[trackId];
    if (existing == null) return;
    final kept = lane == null
        ? const <TrackProgressInfo>[]
        : existing
              .where((info) => isStemProgressPhase(info.phase) != lane)
              .toList();
    if (kept.length == existing.length) return;
    final next = {...state};
    if (kept.isEmpty) {
      next.remove(trackId);
    } else {
      next[trackId] = kept;
    }
    state = next;
  }
}

final trackProgressProvider =
    NotifierProvider<TrackProgressMap, Map<String, List<TrackProgressInfo>>>(
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

class LibraryAnalysisEpoch extends Notifier<int> {
  @override
  int build() => 0;

  void bump() => state++;
}

final libraryAnalysisEpochProvider =
    NotifierProvider<LibraryAnalysisEpoch, int>(LibraryAnalysisEpoch.new);

class TrackHotCues extends Notifier<Map<String, List<HotCue>>> {
  @override
  Map<String, List<HotCue>> build() => const {};

  void set(String trackId, List<HotCue> cues) {
    state = {...state, trackId: cues};
  }
}

final trackHotCuesProvider =
    NotifierProvider<TrackHotCues, Map<String, List<HotCue>>>(TrackHotCues.new);

class TrackSavedLoops extends Notifier<Map<String, List<SavedLoop>>> {
  @override
  Map<String, List<SavedLoop>> build() => const {};

  void set(String trackId, List<SavedLoop> loops) {
    state = {...state, trackId: loops};
  }
}

final trackSavedLoopsProvider =
    NotifierProvider<TrackSavedLoops, Map<String, List<SavedLoop>>>(
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

/// Which library pane holds browser focus: the sidebar (collections / drive /
/// history) or the track list.
///
/// Controller mappings address panes, never Flutter widgets: a focus step
/// (`LibraryNavigation::focus_next` / `focus_prev`) moves this, and the browse
/// rotary acts on whichever pane owns it.
enum LibraryFocusZone { sidebar, trackList }

/// Focus step order. Two panes today, so next and previous both land on the
/// other one; walking an ordered list keeps adding a pane from changing callers.
const _focusZoneOrder = [LibraryFocusZone.sidebar, LibraryFocusZone.trackList];

class LibraryFocusZoneNotifier extends Notifier<LibraryFocusZone> {
  @override
  LibraryFocusZone build() => LibraryFocusZone.trackList;

  /// Step focus one pane over. The count of steps is the sign only: a rotary
  /// tick is never more than one pane wide.
  ///
  /// The evt handler rejects a zero delta before reaching here; this guard is
  /// the notifier's own contract for direct callers.
  void move(int delta) {
    if (delta == 0) {
      return;
    }
    final count = _focusZoneOrder.length;
    final next = (_focusZoneOrder.indexOf(state) + delta.sign + count) % count;
    state = _focusZoneOrder[next];
  }

  /// Park focus on one pane (a click in that pane).
  void set(LibraryFocusZone zone) => state = zone;
}

final libraryFocusZoneProvider =
    NotifierProvider<LibraryFocusZoneNotifier, LibraryFocusZone>(
      LibraryFocusZoneNotifier.new,
    );

/// Which library page the narrow (sub-1024px) layout shows: the sidebar list,
/// or the content pane it navigates to.
enum LibraryNarrowPage { list, content }

class LibraryNarrowPageNotifier extends Notifier<LibraryNarrowPage> {
  @override
  LibraryNarrowPage build() => LibraryNarrowPage.list;

  void set(LibraryNarrowPage page) => state = page;
}

final libraryNarrowPageProvider =
    NotifierProvider<LibraryNarrowPageNotifier, LibraryNarrowPage>(
      LibraryNarrowPageNotifier.new,
    );

/// The active list's focused-row payload, so controller / MIDI "load focused
/// row" acts on whichever list is on screen (library or history).
///
/// A plain holder rather than a `Notifier`: a list relinquishes it from
/// `dispose` by clearing [owner], and Riverpod forbids a provider write during
/// teardown. Consumers read it at load time, so no reactivity is needed.
class FocusedTrackPayload {
  TrackDragPayload? payload;

  /// Token of the list that currently owns [payload]; `null` once it unmounts.
  Object? owner;
}

final focusedTrackPayloadProvider = Provider<FocusedTrackPayload>(
  (ref) => FocusedTrackPayload(),
);

/// Phases driven by the stem pipeline rather than track analysis. `decode`
/// belongs to the stem pass: it re-decodes the source for the separator.
bool isStemProgressPhase(String phase) =>
    phase == 'decode' || phase.startsWith('stems_');

bool _stemsStillRunning(Ref ref, String trackId) {
  if (ref.read(stemGeneratingTrackIdsProvider).contains(trackId)) {
    return true;
  }
  final jobs = ref.read(trackProgressProvider)[trackId];
  return jobs != null && jobs.any((info) => isStemProgressPhase(info.phase));
}

/// Bus record → the row shape the library UI consumes.
LibraryTrackSummary libraryTrackSummary(TrackSummary track) =>
    LibraryTrackSummary(
      id: track.id,
      displayName: track.displayName,
      artist: track.artist,
      title: track.title,
      album: track.album,
      genre: track.genre,
      bpm: track.bpm,
      key: track.key,
      durationMs: track.durationMs,
      path: track.path,
      isrc: track.isrc,
      rating: track.rating,
      comment: track.comment,
      year: track.year,
      trackNumber: track.trackNumber,
      discNumber: track.discNumber,
      colorLabel: _colorLabelFromWire(track.colorLabel),
    );

/// Bus beat grid → the row shape the waveform lane consumes.
BeatGridData beatGridData(BeatGrid grid) =>
    BeatGridData(beats: grid.beats, downbeats: grid.downbeats, bpm: grid.bpm);

/// Wire colour label → the closed UI enum; unknown values are dropped.
LibraryColorLabel? _colorLabelFromWire(String? label) => switch (label) {
  'pink' => LibraryColorLabel.pink,
  'red' => LibraryColorLabel.red,
  'orange' => LibraryColorLabel.orange,
  'yellow' => LibraryColorLabel.yellow,
  'green' => LibraryColorLabel.green,
  'aqua' => LibraryColorLabel.aqua,
  'blue' => LibraryColorLabel.blue,
  'purple' => LibraryColorLabel.purple,
  _ => null,
};

/// The library half of [dispatchBusEvent].
final libraryEvtDispatchProvider = Provider<void Function(Event)>((ref) {
  return (event) {
    if (!ref.mounted) {
      return;
    }
    handleLibraryEvt(ref, event);
  };
});

/// Apply a library egress event to Riverpod state
/// (list patch/invalidate, progress, …). Events that are not library egress
/// are ignored, so the bus bootstrap can hand every event to every handler.
void handleLibraryEvt(Ref ref, Event event) {
  switch (event) {
    case Event_TrackUpdated(:final field0):
      ref
          .read(collectionTracksProvider.notifier)
          .scheduleTrackRowEvent(libraryTrackSummary(field0.track));
    case Event_TrackAnalyzed(:final field0):
      final track = field0.track;
      ref
          .read(collectionTracksProvider.notifier)
          .scheduleTrackRowEvent(libraryTrackSummary(track));
      final trackId = track.id;
      final stemsWaiting = _stemsStillRunning(ref, trackId);
      if (!stemsWaiting) {
        ref.read(analyzingTrackIdsProvider.notifier).clearIf(trackId);
      }
      ref.read(trackBeatGridsProvider.notifier).remove(trackId);
      // Analysis finished: drop only its lane so a concurrent stem job keeps
      // reporting.
      ref.read(trackProgressProvider.notifier).clearLane(trackId, lane: false);
      ref.read(libraryAnalysisEpochProvider.notifier).bump();
    case Event_TrackProgress(:final field0):
      final trackId = field0.trackId;
      final phase = field0.phase;
      ref
          .read(trackProgressProvider.notifier)
          .set(trackId, phase, field0.fraction);
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
    case Event_Error(:final field0):
      // The notice itself is surfaced once, by `dispatchBusEvent`; this handler
      // only clears the per-track job state the failure ended.
      final trackId = field0.trackId;
      if (trackId != null) {
        ref.read(analyzingTrackIdsProvider.notifier).clearIf(trackId);
        ref
            .read(stemGeneratingTrackIdsProvider.notifier)
            .setGenerating(trackId, false);
        ref.read(trackProgressProvider.notifier).clearIf(trackId);
      } else {
        ref.read(analyzingTrackIdsProvider.notifier).clear();
      }
    case Event_HotCuesChanged(:final field0):
      ref
          .read(trackHotCuesProvider.notifier)
          .set(field0.trackId, field0.hotCues);
    case Event_LoopsChanged(:final field0):
      ref
          .read(trackSavedLoopsProvider.notifier)
          .set(field0.trackId, field0.loops);
    case Event_BeatGridChanged(:final field0):
      ref
          .read(trackBeatGridsProvider.notifier)
          .set(field0.trackId, beatGridData(field0.beatGrid));
    case Event_Navigate(:final field0):
      applyLibraryNavigateEvt(ref, field0.delta);
    case Event_Focus(:final field0):
      // Same boundary guard as navigate: a step with no ticks is not a step.
      final focusDelta = field0.delta;
      if (focusDelta != 0) {
        // A press of the library knob reveals the drawer; SHIFT + press
        // dismisses it, and owns the step — there is no cursor to move in a
        // surface that is closing. Docked mode just steps focus, as before.
        if (focusDelta < 0) {
          if (dismissLibraryDrawer(ref)) {
            return;
          }
        } else {
          revealLibraryDrawer(ref);
        }
        ref.read(libraryFocusZoneProvider.notifier).move(focusDelta);
      }
    case Event_Load(:final field0):
      ref.read(pendingFocusedDeckLoadProvider.notifier).request(field0.deck);
    case Event_HistorySessionUpdated():
      ref.read(historyRefreshTickProvider.notifier).bump();
    default:
      break;
  }
}

/// Apply a `LibraryNavigation::navigate` evt to whichever pane owns focus.
///
/// The rotary is one control for both panes: the track list moves its focused
/// row, the sidebar moves the focused row of the active tab. A zero delta (a
/// relative knob that reported no ticks) is rejected here, once, so the helpers
/// below never have to repeat the guard.
void applyLibraryNavigateEvt(Ref ref, int delta) {
  if (delta == 0) {
    return;
  }
  if (ref.read(libraryFocusZoneProvider) == LibraryFocusZone.sidebar) {
    navigateSidebarSelection(ref, delta);
    return;
  }
  ref.read(focusedTrackRowIndexProvider.notifier).navigate(delta);
}

/// Browse the sidebar: the focused row of the active tab follows [delta],
/// mirroring [focusedTrackRowIndexProvider] for the track list.
void navigateSidebarSelection(Ref ref, int delta) {
  switch (ref.read(librarySourceTabProvider)) {
    case LibrarySourceTab.collections:
      final collections = ref.read(collectionsProvider).asData?.value;
      if (collections == null || collections.isEmpty) {
        return;
      }
      // The base row comes from [activeCollectionIdProvider] itself, which
      // already resolves "nothing selected" to the first row — so the
      // `index < 0` arm only covers a read where the selection is genuinely
      // absent from this list (a refresh landing mid-step), and it lands on
      // the same row the provider would show.
      final active = ref.read(activeCollectionIdProvider);
      final index = collections.indexWhere((c) => c.id == active);
      ref
          .read(selectedCollectionIdProvider.notifier)
          .set(
            collections[navigateIndex(
                  index < 0 ? 0 : index,
                  collections.length,
                  delta,
                )]
                .id,
          );
    case LibrarySourceTab.history:
      final sessions = ref.read(historySessionsProvider).asData?.value;
      if (sessions == null || sessions.isEmpty) {
        return;
      }
      // Same as the collections case: the active session resolves from
      // [activeHistorySessionIdProvider], which already falls back to the
      // first row.
      final active = ref.read(activeHistorySessionIdProvider);
      final index = sessions.indexWhere((s) => s.id == active);
      ref
          .read(selectedHistorySessionIdProvider.notifier)
          .set(
            sessions[navigateIndex(
                  index < 0 ? 0 : index,
                  sessions.length,
                  delta,
                )]
                .id,
          );
    case LibrarySourceTab.drive:
      _navigateDriveTree(ref, delta);
  }
}

Future<void> analyzeTrackAction(
  ProviderContainer container,
  String trackId, {
  bool force = false,
}) async {
  container.read(analyzingTrackIdsProvider.notifier).add(trackId);
  container.read(appNoticeProvider.notifier).clear();
  try {
    final transport = await container.read(libraryTransportProvider.future);
    await transport.analyzeTrack(trackId: trackId, force: force);
  } catch (e) {
    container.read(analyzingTrackIdsProvider.notifier).clearIf(trackId);
    container.read(appNoticeProvider.notifier).setError('$e');
  }
}

/// Queue stem generation for one track.
///
/// Deliberately independent of [analyzeTrackAction]: a full Demucs pass is far
/// more expensive than analysis, so it is opt-in per track. Server-side this
/// no-ops when a valid stem cache already exists.
Future<void> generateStemsAction(WidgetRef ref, String trackId) async {
  ref
      .read(stemGeneratingTrackIdsProvider.notifier)
      .setGenerating(trackId, true);
  ref.read(appNoticeProvider.notifier).clear();
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    await transport.generateStems(trackId: trackId);
  } catch (e) {
    ref
        .read(stemGeneratingTrackIdsProvider.notifier)
        .setGenerating(trackId, false);
    ref.read(appNoticeProvider.notifier).setError('$e');
  }
}

Future<void> refreshTrackAction(WidgetRef ref, String trackId) async {
  ref.read(appNoticeProvider.notifier).clear();
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    await transport.refreshTrack(trackId: trackId);
  } catch (e) {
    ref.read(appNoticeProvider.notifier).setError('$e');
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

/// Whether [parent] stays inside [volume]'s mount point.
///
/// A volume's parent sits outside it (e.g. `/media`), so the tree must not
/// offer that row as the "up" step.
///
/// Path containment rather than a string prefix, so a sibling like
/// `/mnt/data-backup` is not treated as being inside `/mnt/data`, and the
/// volume root counts as inside its own tree.
bool driveParentInVolume(String? parent, FsVolumeInfo? volume) {
  if (parent == null || volume == null) {
    return false;
  }
  return _isWithinOrEqual(volume.path, parent);
}

/// Whether [path] is [root] or nested inside it.
///
/// [p.isWithin] is exclusive, and the root has to count as inside: the "up" row
/// of a volume's own root is the volume itself. Both callers of this need the
/// inclusive form, so the [p.isWithin] asymmetry lives here once.
bool _isWithinOrEqual(String root, String path) {
  if (root == path) {
    return true;
  }
  return p.isWithin(root, path);
}

/// Visible Drive tree rows in pane order: the "up" row while a folder is open
/// (index 0) and its parent is still inside the volume, then that folder's
/// subfolders; volumes at the root.
///
/// Takes the values it derives from instead of reading them, so a caller can
/// pass one snapshot and be sure the rows, the cursor, and the focused path all
/// describe the same listing.
List<String> _driveTreeRowsFrom({
  required String? currentPath,
  required FsDirectoryListing? listing,
  required List<FsVolumeInfo> volumes,
}) {
  if (currentPath == null) {
    return [for (final v in volumes) v.path];
  }
  if (listing == null) {
    return const [];
  }
  // The "up" row is always rendered by the pane (disabled when its parent
  // is outside the active volume), so keep it in the row list to keep the
  // provider's indices aligned with the widget's visual rows.
  return [
    listing.parent ?? listing.path,
    for (final d in listing.directories) d.path,
  ];
}

/// Drive rows for the current state, as one read of each source.
List<String> _driveTreeRows(Ref ref) => _driveTreeRowsFrom(
  currentPath: ref.read(driveCurrentPathProvider),
  listing: ref.read(driveListingProvider).asData?.value,
  volumes: ref.read(driveVolumesProvider).asData?.value ?? const [],
);

/// Focused Drive tree row, held by path.
///
/// A row's path is its identity, so a listing that changes under the cursor
/// cannot leave it pointing at a different row — the way the track list pins its
/// focused row by id. When the path is no longer on screen the cursor reads as
/// the first row, which is where an externally opened folder starts.
class DriveTreeCursor extends Notifier<String?> {
  @override
  String? build() => null;

  /// Park the cursor on [path] (a row press).
  void set(String? path) => state = path;

  /// Step the cursor by [delta] rows within [rows] and return the row it lands
  /// on, so the caller can apply that path without re-deriving the rows.
  String? step(List<String> rows, int delta) {
    if (rows.isEmpty) {
      state = null;
      return null;
    }
    final current = rows.indexOf(state ?? '');
    final next =
        rows[navigateIndex(current < 0 ? 0 : current, rows.length, delta)];
    state = next;
    return next;
  }
}

final driveTreeCursorProvider = NotifierProvider<DriveTreeCursor, String?>(
  DriveTreeCursor.new,
);

/// Path of the Drive tree row the cursor is on, `null` when no row can hold it.
final driveTreeFocusedPathProvider = Provider<String?>((ref) {
  final rows = _driveTreeRowsWatched(ref);
  if (rows.isEmpty) {
    return null;
  }
  final cursor = ref.watch(driveTreeCursorProvider);
  return cursor != null && rows.contains(cursor) ? cursor : rows.first;
});

/// [_driveTreeRowsFrom] fed by the values this provider watches, so a listing
/// that resolves — or a volume list that arrives — re-derives the rows and the
/// focused path stays on a row that exists.
List<String> _driveTreeRowsWatched(Ref ref) => _driveTreeRowsFrom(
  currentPath: ref.watch(driveCurrentPathProvider),
  listing: ref.watch(driveListingProvider).asData?.value,
  volumes: ref.watch(driveVolumesProvider).asData?.value ?? const [],
);

/// Browse the Drive tree by [delta] rows.
///
/// A tree row's selection *is* the folder it opens, so the cursor moves and the
/// row it lands on becomes the listing (the "up" row walks back out).
///
/// Callers reject a zero [delta]; a zero step would still be a no-op here, but
/// the guard lives at the evt boundary so it cannot be forgotten.
void _navigateDriveTree(Ref ref, int delta) {
  // One derivation: the rows are read once and the path it lands on comes back
  // from the step, so a listing resolving halfway through cannot apply the new
  // cursor to a different row set.
  final rows = _driveTreeRows(ref);
  if (rows.isEmpty) {
    return;
  }
  final path = ref.read(driveTreeCursorProvider.notifier).step(rows, delta);
  final listing = ref.read(driveListingProvider).asData?.value;
  // Mirror the pane's disabled "up" row: a parent outside the active volume is
  // rendered but not openable, so the cursor cannot apply it.
  final openable =
      path == null ||
      path != listing?.parent ||
      driveParentInVolume(listing?.parent, ref.read(driveActiveVolumeProvider));
  if (path != null && openable) {
    ref.read(driveCurrentPathProvider.notifier).set(path);
  }
}

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
      final filter = ref.watch(trackFilterProvider);
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
          final needle = normalizeTrackFilter(filter);
          return AsyncData([
            for (final t in tracks)
              if (trackMatchesNeedle(t, needle)) t,
          ]);
        },
      );
    });

/// Sort field for the library list toolbar.
enum LibrarySortField {
  title('Title'),
  artist('Artist'),
  bpm('BPM'),
  key('Key'),
  length('Length'),
  rating('Rating'),
  year('Year'),
  trackNumber('Track #'),
  discNumber('Disc #'),
  color('Colour');

  new(this.label);

  final String label;
}

class LibrarySort extends Notifier<(LibrarySortField, bool)> {
  @override
  (LibrarySortField, bool) build() => (LibrarySortField.title, true);

  /// Table-header style: a new field sorts ascending; the active field toggles.
  void select(LibrarySortField field) {
    if (state.$1 == field) {
      state = (field, !state.$2);
    } else {
      state = (field, true);
    }
  }
}

final librarySortProvider =
    NotifierProvider<LibrarySort, (LibrarySortField, bool)>(LibrarySort.new);

/// Sort [tracks] by [field]. Null metadata sorts last in either direction so
/// an unanalyzed track never displaces a scanned one at the top of the list.
List<LibraryTrackSummary> sortLibraryTracks(
  List<LibraryTrackSummary> tracks,
  LibrarySortField field, {
  required bool ascending,
}) {
  // Sort (originalIndex, track) pairs rather than bare tracks: `List.sort` is
  // only stable below its insertion-sort threshold, so a comparator that
  // returns 0 for equal keys lets a larger library shuffle tied rows.
  // Carrying the index makes the ordering total, which keeps provider order
  // for ties at any size.
  final indexed = [for (var i = 0; i < tracks.length; i++) (i, tracks[i])];
  indexed.sort((a, b) {
    final left = _sortKeyFor(a.$2, field);
    final right = _sortKeyFor(b.$2, field);
    final byRank = left.$1.compareTo(right.$1);
    if (byRank != 0) {
      // Rank is the missing-value marker, and it is never inverted: a track
      // with no BPM belongs at the bottom in both directions, not floating to
      // the top when the user flips the sort.
      return byRank;
    }
    final byValue = left.$2.compareTo(right.$2);
    if (byValue != 0) {
      return ascending ? byValue : -byValue;
    }
    return a.$1.compareTo(b.$1);
  });
  return [for (final (_, track) in indexed) track];
}

/// Rank prefix: a known value sorts before a missing one, in both directions.
const _kSortHasValue = 0;

/// Rank for a missing value. Every missing key shares this rank and an empty
/// value, so unknowns tie among themselves and settle at the bottom.
const _kSortMissingValue = 1;

/// Sort key for one field: a rank that separates a known value from a missing
/// one, and the value itself as a string so one comparison serves text and
/// numbers alike. A track with no BPM should never outrank a scanned one,
/// hence the rank in [_kSortHasValue] / [_kSortMissingValue].
(int rank, String value) _sortKeyFor(
  LibraryTrackSummary track,
  LibrarySortField field,
) => switch (field) {
  LibrarySortField.title => _textSortKey(trackTitleLabel(track)),
  LibrarySortField.artist => _textSortKey(track.artist ?? ''),
  LibrarySortField.key => _keySortKey(track.key),
  LibrarySortField.bpm => _numericSortKey(track.bpm),
  LibrarySortField.length => _numericSortKey(track.durationMs?.toDouble()),
  LibrarySortField.rating => _numericSortKey(track.rating?.toDouble()),
  LibrarySortField.year => _numericSortKey(track.year?.toDouble()),
  LibrarySortField.trackNumber => _numericSortKey(
    track.trackNumber?.toDouble(),
  ),
  LibrarySortField.discNumber => _numericSortKey(track.discNumber?.toDouble()),
  LibrarySortField.color => _numericSortKey(
    track.colorLabel == null
        ? null
        : colorLabelNames.indexOf(track.colorLabel!).toDouble(),
  ),
};

(int, String) _textSortKey(String? value) {
  final text = value?.trim().toLowerCase() ?? '';
  return text.isEmpty ? (_kSortMissingValue, '') : (_kSortHasValue, text);
}

/// Camelot wheel position, not the raw label: sorted as text, `10A` lands
/// before `2A`, which reads as no order at all. [camelotSlotForKey] normalises
/// either key format to `(1-12, A=minor / B=major)`, so the list follows the
/// wheel — `1A, 1B, 2A, … 12B` — keeping harmonically adjacent keys together.
(int, String) _keySortKey(String? key) {
  final slot = camelotSlotForKey(key ?? '');
  if (slot == null) {
    // An unparseable key joins the unknowns rather than sorting as text.
    return (_kSortMissingValue, '');
  }
  final (number, minor) = slot;
  return (
    _kSortHasValue,
    '${number.toString().padLeft(2, '0')}${minor ? '0' : '1'}',
  );
}

/// Fixed-width padded numeric key, so `128` does not sort before `90`. A
/// non-finite or negative value is treated as missing: a NaN from a failed
/// analysis must not poison the whole ordering. BPM and duration are never
/// negative, so the sign needs no case.
///
/// Scaled to milli-units before padding: rounding to whole units would map
/// 109.7 and 109.9 to the same key, turning two distinct BPMs into a tie that
/// falls back to provider order instead of the true value.
///
/// Values are clamped to [_kSortNumericMax] so a scaled key is always exactly
/// [_kSortNumericDigits] long. Past that the key grows a digit, `padLeft`
/// stops aligning, and the lexicographic comparison inverts: `'1000…'` sorts
/// *below* `'999…'`, which silently mis-orders a short track after a long one.
/// (`round()` also clamps to the int64 maximum rather than throwing, so an
/// absurd duration degrades quietly instead of failing loudly.)
(int, String) _numericSortKey(double? value) {
  if (value == null || !value.isFinite || value < 0) {
    return (_kSortMissingValue, '');
  }
  final clamped = value > _kSortNumericMax ? _kSortNumericMax : value;
  final scaled = (clamped * _kSortNumericScale).round();
  return (_kSortHasValue, scaled.toString().padLeft(_kSortNumericDigits, '0'));
}

/// Milli-units of precision for numeric sort keys: finer than any BPM or
/// duration the app displays, coarse enough that `(value * scale).round()` is
/// not tripped by floating-point representation.
const _kSortNumericScale = 1000;

/// Largest key magnitude, in the field's own units (BPM or milliseconds).
///
/// `_kSortNumericMax * _kSortNumericScale` is 10^15, which is 16 digits and
/// below 2^53, so it converts to an integer exactly. A real BPM or duration is
/// orders of magnitude smaller; anything above this is garbage metadata and
/// sorts as the largest value rather than corrupting the ordering.
const _kSortNumericMax = 1e12;

/// Width of a padded numeric key: the digits of `10^15`.
const _kSortNumericDigits = 16;

/// Right-pane rows: collection tracks or drive files, depending on the tab,
/// ordered by the toolbar sort. Sorting here rather than in the view keeps
/// list index, provider index and `focusedTrackRowIndexProvider` in agreement,
/// so MIDI "load focused row" targets the row the user is looking at.
final libraryTableTracksProvider =
    Provider<AsyncValue<List<LibraryTrackSummary>>>((ref) {
      // Explicit type, not inferred: the switch mixes three AsyncValue
      // sources, and without it the history arm's empty literal degrades the
      // whole expression to Object/dynamic.
      final AsyncValue<List<LibraryTrackSummary>> source = switch (ref.watch(
        librarySourceTabProvider,
      )) {
        LibrarySourceTab.collections => ref.watch(filteredTracksProvider),
        LibrarySourceTab.drive => ref.watch(driveTableTracksProvider),
        LibrarySourceTab.history => const AsyncData([]),
      };
      final (field, ascending) = ref.watch(librarySortProvider);
      return source.whenData(
        (tracks) => sortLibraryTracks(tracks, field, ascending: ascending),
      );
    });
