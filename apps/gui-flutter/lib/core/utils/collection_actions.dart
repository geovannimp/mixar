import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/android_media_access.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/create_collection_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';

/// Reload the sidebar counts and the visible collection rows after a
/// membership change.
void refreshCollectionViews(WidgetRef ref) {
  ref
    ..invalidate(collectionsProvider)
    ..invalidate(collectionTracksProvider);
}

/// Whether a folder scan matched anything at all. Drives both the notice and
/// whether the new collection is worth selecting.
bool folderImportFoundTracks(AddFolderCollectionResult result) =>
    result.added + result.updated + result.skipped > 0;

/// What to tell the user after a folder import: `null` when everything came in
/// cleanly, otherwise the copy and whether it reads as an error. Unreadable
/// files are named even when the rest of the import worked, so a permission or
/// format problem is never swallowed; a folder whose files all matched
/// something already scanned is not called empty.
({String message, bool isError})? folderImportNotice(
  AddFolderCollectionResult result,
) {
  final name = result.collection.name;
  if (result.failed > 0) {
    final files = result.failed == 1 ? 'file' : 'files';
    return (
      message: 'Could not read ${result.failed} $files in “$name”.',
      isError: true,
    );
  }
  if (folderImportFoundTracks(result)) {
    return null;
  }
  return (message: 'No audio files found in “$name”.', isError: false);
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
    // Folder only: whether the scan found anything. Playlist creation always
    // yields a selectable collection, even when empty.
    var imported = true;
    switch (request.type) {
      case CreateCollectionType.folder:
        // The host scan reads paths with `std::fs`; without the Android media
        // permission it sees no files, so stop with a reason instead of
        // creating a collection that can only list nothing.
        if (!await ensureAudioReadAccess()) {
          ref.read(appNoticeProvider.notifier).setError(audioReadDeniedMessage);
          return null;
        }
        final result = await transport.addFolderCollection(
          folderPath: request.folderPath!,
          scanFolderTree: request.scanSubfolders,
          name: name.isEmpty ? null : name,
        );
        collection = result.collection;
        imported = folderImportFoundTracks(result);
        final notice = folderImportNotice(result);
        if (notice != null) {
          final notices = ref.read(appNoticeProvider.notifier);
          if (notice.isError) {
            notices.setError(notice.message);
          } else {
            notices.setNotice(notice.message);
          }
        }
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
    refreshCollectionViews(ref);
    // A folder that brought nothing in stays in the sidebar, but selecting it
    // would show an empty list as if the import had worked; the notice above
    // says why, and the caller stays where it was.
    return imported ? collection : null;
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

/// Add one track to a playlist collection.
///
/// Sortable playlists append a new entry (the same track may appear twice);
/// other playlists keep a single row per track. Returns `false` when the add
/// failed, after surfacing the error.
Future<bool> addTrackToCollection(
  BuildContext context,
  WidgetRef ref,
  String collectionId,
  String trackId,
) async {
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    await transport.addCollectionEntry(
      collectionId: collectionId,
      trackId: trackId,
    );
  } catch (e) {
    if (context.mounted) {
      showMixarToast(
        context: context,
        variant: MixarToastVariant.destructive,
        title: Text('$e'),
      );
    }
    return false;
  }
  // `ref` dies with the widget; the awaited call can outlive the pane.
  if (!context.mounted) {
    return true;
  }
  refreshCollectionViews(ref);
  return true;
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
  refreshCollectionViews(ref);
  if (deleted) {
    ref
        .read(appNoticeProvider.notifier)
        .setNotice('Deleted “${collection.name}”.');
  }
}
