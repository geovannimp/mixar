import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/create_collection_dialog.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

Future<LibraryCollectionSummary?> createCollection(
  WidgetRef ref,
  CreateCollectionResult request, {
  String? historySessionId,
}) async {
  ref.read(libraryMessageProvider.notifier).clear();
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    final name = request.name.trim();
    final LibraryCollectionSummary collection;
    switch (request.type) {
      case CreateCollectionType.folder:
        final result = await transport.addFolderCollection(
          folderPath: request.folderPath!,
          scanFolderTree: request.scanSubfolders,
          name: name.isEmpty ? null : name,
        );
        collection = result.collection;
      case CreateCollectionType.playlist:
        collection = historySessionId == null
            ? await transport.addPlaylistCollection(
                name: name,
                sortable: request.sortable,
              )
            : await transport.saveHistoryAsPlaylist(
                sessionId: historySessionId,
                name: name,
                sortable: request.sortable,
              );
    }
    ref.invalidate(collectionsProvider);
    ref.invalidate(collectionTracksProvider);
    return collection;
  } catch (e) {
    ref.read(libraryMessageProvider.notifier).setError('$e');
    return null;
  }
}

void selectCreatedCollection(
  WidgetRef ref,
  LibraryCollectionSummary collection,
) {
  ref.read(selectedCollectionIdProvider.notifier).set(collection.id);
  ref.read(librarySourceTabProvider.notifier).set(LibrarySourceTab.collections);
}

/// Queue collection-wide analysis onto the library cmd bus (one `AnalyzeTrack`
/// per target, plus `GenerateStems` when requested). Marks the queued ids
/// analyzing/generating up front so rows show progress pills immediately; the
/// worker's per-track events clear them as jobs finish.
Future<void> analyzeCollectionAction(
  WidgetRef ref,
  String collectionId, {
  required bool force,
  required bool generateStems,
}) async {
  ref.read(libraryMessageProvider.notifier).clear();
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    final result = await transport.analyzeCollection(
      collectionId: collectionId,
      force: force,
      generateStems: generateStems,
    );
    final analyzing = ref.read(analyzingTrackIdsProvider.notifier);
    result.queuedTrackIds.forEach(analyzing.add);
    final stems = ref.read(stemGeneratingTrackIdsProvider.notifier);
    for (final id in result.stemTrackIds) {
      stems.setGenerating(id, true);
    }
    final queued = result.queuedTrackIds.length;
    final stemCount = result.stemTrackIds.length;
    ref
        .read(libraryMessageProvider.notifier)
        .setNotice(
          queued == 0 && stemCount == 0
              ? 'Everything is already analyzed.'
              : 'Queued $queued for analysis'
                    '${generateStems ? ' + $stemCount for stems' : ''}.',
        );
  } catch (e) {
    ref.read(libraryMessageProvider.notifier).setError('$e');
  }
}
