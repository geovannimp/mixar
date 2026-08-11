import 'package:flutter_riverpod/flutter_riverpod.dart';
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
final selectedCollectionIdProvider = StateProvider<String?>((ref) => null);

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

final trackFilterProvider = StateProvider<String>((ref) => '');

String trackTitleLabel(LibraryTrackSummary t) =>
    (t.title?.isNotEmpty ?? false) ? t.title! : t.displayName;

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
