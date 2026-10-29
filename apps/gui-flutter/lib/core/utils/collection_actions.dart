import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/smart_playlist_rules.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/create_collection_drawer.dart';
import 'package:gui_flutter/widgets/library/smart_playlist_rules_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';

/// Drop the cached collection list and open track list after a write that can
/// change either, so every action refreshes the same two providers.
void invalidateCollectionLists(WidgetRef ref) {
  ref
    ..invalidate(collectionsProvider)
    ..invalidate(collectionTracksProvider);
}

Future<LibraryCollectionSummary?> createCollection(
  WidgetRef ref,
  CreateCollectionResult request, {
  String? historySessionId,
}) async {
  ref.read(appNoticeProvider.notifier).clear();
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
      case CreateCollectionType.smartPlaylist:
        collection = await transport.createSmartPlaylist(
          name: name,
          rulesJson: request.rules!.toJsonString(),
        );
    }
    invalidateCollectionLists(ref);
    return collection;
  } catch (e) {
    ref.read(appNoticeProvider.notifier).setError('$e');
    return null;
  }
}

void selectCreatedCollection(
  WidgetRef ref,
  LibraryCollectionSummary collection,
) {
  ref.read(selectedCollectionIdProvider.notifier).set(collection.id);
  ref.read(librarySourceTabProvider.notifier).set(LibrarySourceTab.collections);
  // The caller named a collection, so show its tracks. On a narrow surface that
  // is the content page; in the split the id change already swaps the list.
  ref.read(libraryNarrowPageProvider.notifier).set(LibraryNarrowPage.content);
}

/// Edit a smart playlist's name and rules, then persist them.
Future<void> editSmartPlaylistRulesAction(
  BuildContext context,
  WidgetRef ref,
  LibraryCollectionSummary collection,
) async {
  ref.read(appNoticeProvider.notifier).clear();
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    final rules = SmartPlaylistRules.parse(
      await transport.smartPlaylistRules(collectionId: collection.id),
    );
    if (!context.mounted) {
      return;
    }
    final result = await showSmartPlaylistRulesDialog(
      context,
      name: collection.name,
      initial: rules,
    );
    if (result == null || !context.mounted) {
      return;
    }
    await transport.updateSmartPlaylist(
      collectionId: collection.id,
      name: result.name,
      rulesJson: result.rules.toJsonString(),
    );
    // `ref` dies with the widget; the awaited write can outlive it.
    if (!context.mounted) {
      return;
    }
    invalidateCollectionLists(ref);
    ref.read(appNoticeProvider.notifier).setNotice('Rules saved.');
  } on Object catch (e) {
    if (context.mounted) {
      ref.read(appNoticeProvider.notifier).setError('$e');
    }
  }
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
  ref.read(appNoticeProvider.notifier).clear();
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
        .read(appNoticeProvider.notifier)
        .setNotice(
          queued == 0 && stemCount == 0
              ? 'Everything is already analyzed.'
              : 'Queued $queued for analysis'
                    '${generateStems ? ' + $stemCount for stems' : ''}.',
        );
  } catch (e) {
    ref.read(appNoticeProvider.notifier).setError('$e');
  }
}

/// Confirm then delete a collection. Tracks only referenced by it go too.
Future<void> deleteCollectionAction(
  BuildContext context,
  WidgetRef ref,
  LibraryCollectionSummary collection,
) async {
  final confirmed = await showMixarConfirm<bool>(
    context: context,
    title: 'Delete this collection?',
    body:
        'Removes “${collection.name}” from your library. \n'
        '\n'
        '• Tracks only used by this collection are removed too.\n'
        '• Original audio files on disk are preserved.\n',
    actions: const [
      MixarDialogAction(
        label: 'Cancel',
        value: false,
        variant: MixarButtonVariant.outline,
      ),
      MixarDialogAction(
        label: 'Delete',
        value: true,
        variant: MixarButtonVariant.destructive,
      ),
    ],
  );
  if (confirmed != true || !context.mounted) {
    return;
  }
  ref.read(appNoticeProvider.notifier).clear();
  var deleted = false;
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    await transport.deleteCollection(collectionId: collection.id);
    deleted = true;
  } catch (e) {
    // The collection row may already be gone when track cleanup fails, so the
    // list must refresh either way or the UI keeps offering a stale row.
    if (context.mounted) {
      ref.read(appNoticeProvider.notifier).setError('$e');
    }
  }
  // `ref` dies with the widget; the awaited call can outlive the topbar.
  if (!context.mounted) {
    return;
  }
  if (ref.read(selectedCollectionIdProvider) == collection.id) {
    ref.read(selectedCollectionIdProvider.notifier).set(null);
  }
  invalidateCollectionLists(ref);
  if (deleted) {
    ref
        .read(appNoticeProvider.notifier)
        .setNotice('Deleted “${collection.name}”.');
  }
}
