import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

/// Opens the shared app-support `library.db` (same folder as Tauri when app IDs match).
final libraryTransportProvider = FutureProvider<LibraryTransport>((ref) async {
  final support = await getApplicationSupportDirectory();
  final dbPath = p.join(support.path, 'library.db');
  return LibraryTransport.open(dbPath: dbPath);
});

final collectionsProvider =
    FutureProvider<List<LibraryCollectionSummary>>((ref) async {
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

final collectionTracksProvider =
    FutureProvider<List<LibraryTrackSummary>>((ref) async {
      final id = ref.watch(activeCollectionIdProvider);
      if (id == null) {
        return const [];
      }
      final transport = await ref.watch(libraryTransportProvider.future);
      return transport.listCollectionTracks(collectionId: id);
    });

class TrackFilter extends Notifier<String> {
  @override
  String build() => '';

  void set(String value) => state = value;
}

final trackFilterProvider =
    NotifierProvider<TrackFilter, String>(TrackFilter.new);

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

class AnalyzingTrackId extends Notifier<String?> {
  Timer? _stuckClear;

  @override
  String? build() {
    ref.onDispose(() => _stuckClear?.cancel());
    return null;
  }

  void set(String? id) {
    _stuckClear?.cancel();
    state = id;
    if (id == null) {
      return;
    }
    // ponytail: clear stuck spinner if evt never arrives. Upgrade: correlate cmd/evt ids.
    _stuckClear = Timer(const Duration(seconds: 60), () {
      if (state == id) {
        state = null;
      }
    });
  }

  void clearIf(String? trackId) {
    if (trackId != null && state == trackId) {
      _stuckClear?.cancel();
      state = null;
    }
  }

  void clear() {
    _stuckClear?.cancel();
    state = null;
  }
}

final analyzingTrackIdProvider =
    NotifierProvider<AnalyzingTrackId, String?>(AnalyzingTrackId.new);

class LibraryMessage extends Notifier<String?> {
  @override
  String? build() => null;

  void clear() => state = null;

  void setError(String message) => state = message;

  void setNotice(String? message) => state = message;
}

final libraryMessageProvider =
    NotifierProvider<LibraryMessage, String?>(LibraryMessage.new);

class LibraryAnalysisEpoch extends Notifier<int> {
  @override
  int build() => 0;

  void bump() => state++;
}

final libraryAnalysisEpochProvider =
    NotifierProvider<LibraryAnalysisEpoch, int>(LibraryAnalysisEpoch.new);

void _handleLibraryEvt(Ref ref, LibraryEvt evt) {
  switch (evt.kind) {
    case LibraryEvtKind.trackUpdated:
    case LibraryEvtKind.trackAnalyzed:
      ref.invalidate(collectionTracksProvider);
      ref.invalidate(collectionsProvider);
      if (evt.kind == LibraryEvtKind.trackAnalyzed) {
        ref
            .read(analyzingTrackIdProvider.notifier)
            .clearIf(evt.trackId ?? evt.track?.id);
        ref.read(libraryAnalysisEpochProvider.notifier).bump();
      }
    case LibraryEvtKind.error:
      ref.read(libraryMessageProvider.notifier).setError(evt.message ?? 'Error');
      if (evt.trackId != null) {
        ref.read(analyzingTrackIdProvider.notifier).clearIf(evt.trackId);
      } else {
        ref.read(analyzingTrackIdProvider.notifier).clear();
      }
    case LibraryEvtKind.notice:
      ref.read(libraryMessageProvider.notifier).setNotice(evt.message);
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
  ref.read(analyzingTrackIdProvider.notifier).set(trackId);
  ref.read(libraryMessageProvider.notifier).clear();
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    await transport.analyzeTrack(trackId: trackId, force: false);
  } catch (e) {
    ref.read(analyzingTrackIdProvider.notifier).clearIf(trackId);
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

enum LibrarySourceTab { collections, drive }

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

final driveCurrentPathProvider =
    NotifierProvider<DriveCurrentPath, String?>(DriveCurrentPath.new);

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
  final sorted = [...volumes]..sort((a, b) => b.path.length.compareTo(a.path.length));
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
        error: (e, st) => AsyncError(e, st),
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
      }
    });
