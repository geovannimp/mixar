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
