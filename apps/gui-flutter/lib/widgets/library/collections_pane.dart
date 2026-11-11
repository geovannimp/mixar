import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/collection_actions.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/create_collection_dialog.dart';
import 'package:gui_flutter/widgets/library/library_layout_scope.dart';
import 'package:gui_flutter/widgets/library/library_nav_row.dart';
import 'package:gui_flutter/widgets/library/library_pane_label.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/m_loader.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Collections sidebar: header + full-width rows (Tauri collection list).
class CollectionsPane extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<CollectionsPane> createState() => _CollectionsPaneState();
}

class _CollectionsPaneState extends ConsumerState<CollectionsPane> {
  var _adding = false;

  Future<void> _addCollection() async {
    final result = await showCreateCollectionDialog(context);
    if (result == null || !mounted) {
      return;
    }
    setState(() => _adding = true);
    final collection = await createCollection(ref, result);
    if (!mounted) {
      return;
    }
    if (collection != null) {
      selectCreatedCollection(ref, collection);
    }
    if (mounted) {
      setState(() => _adding = false);
    }
  }

  void _browseInDrive(String path) {
    ref.read(driveCurrentPathProvider.notifier).set(path);
    ref.read(librarySourceTabProvider.notifier).set(LibrarySourceTab.drive);
    // The menu names a folder, so the intent is the same in either layout:
    // show the folder's contents. On a narrow surface that is the content page.
    ref.read(libraryNarrowPageProvider.notifier).set(LibraryNarrowPage.content);
  }

  /// Drop target for playlist rows only: a folder takes its membership from
  /// the path it was created for, so the core rejects an entry add there.
  static const _playlistKind = 'playlist';
  static const _folderKind = 'folder';

  Widget _row(
    LibraryCollectionSummary c, {
    required String? selectedId,
    required bool sidebarFocused,
    required bool dropActive,
  }) {
    final colors = context.theme.colors;
    return LibraryNavRow(
      title: c.name,
      subtitle: '${c.trackCount} tracks',
      icon: c.kind == _playlistKind
          ? LucideIcons.listMusic
          : LucideIcons.folder,
      selected: c.id == selectedId,
      focused: sidebarFocused && c.id == selectedId,
      dropActive: dropActive,
      onPress: () {
        ref.read(selectedCollectionIdProvider.notifier).set(c.id);
        // Focus follows the pointer, the way a row press in the
        // track list does. On a narrow surface the press is a
        // page change, so focus goes to the track list page
        // instead of parking on the sidebar.
        if (LibraryLayoutScope.maybeOf(context)?.narrow ?? false) {
          ref
              .read(libraryFocusZoneProvider.notifier)
              .set(LibraryFocusZone.trackList);
          ref
              .read(libraryNarrowPageProvider.notifier)
              .set(LibraryNarrowPage.content);
        } else {
          ref
              .read(libraryFocusZoneProvider.notifier)
              .set(LibraryFocusZone.sidebar);
        }
      },
      trailing: c.kind == _folderKind && c.path != null
          ? MTappable.child(
              semanticsLabel: 'Browse ${c.name} in Drive',
              onPress: () => _browseInDrive(c.path!),
              child: Padding(
                padding: const EdgeInsets.all(4),
                child: Icon(
                  LucideIcons.folderOpen,
                  size: 14,
                  color: colors.mutedForeground,
                ),
              ),
            )
          : null,
    );
  }

  /// Add a dropped library track to [collection] and report the outcome. The
  /// core decides duplicates: an append on a sortable playlist, an upsert on a
  /// plain one.
  Future<void> _dropOnPlaylist(
    LibraryCollectionSummary collection,
    TrackDragPayload payload,
  ) async {
    final trackId = payload.trackId;
    if (trackId == null || trackId.isEmpty) {
      return;
    }
    final added = await addTrackToCollection(
      context,
      ref,
      collection.id,
      trackId,
    );
    if (!mounted || !added) {
      return;
    }
    final title = trackDisplayTitle(title: payload.title, path: payload.path);
    showMixarToast(
      context: context,
      title: Text('Added “$title” to “${collection.name}”'),
      variant: MixarToastVariant.success,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final collections = ref.watch(collectionsProvider);
    final selectedId = ref.watch(activeCollectionIdProvider);
    // The selected row doubles as the cursor while the sidebar owns focus.
    final sidebarFocused =
        ref.watch(libraryFocusZoneProvider) == LibraryFocusZone.sidebar;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(10, 4, 4, 8),
          child: Row(
            children: [
              const Expanded(child: LibraryPaneLabel('Collections')),
              AppTooltip(
                tip: 'Create collection',
                child: AppButton.icon(
                  semanticsLabel: 'Create collection',
                  onPress: _adding ? null : _addCollection,
                  variant: .ghost,
                  size: .xs,
                  child: _adding
                      ? const MLoader(size: MLoaderSize.sm)
                      : const Icon(LucideIcons.plus, size: 14),
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: collections.when(
            loading: () => const Center(child: MLoader()),
            error: (e, _) => Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10),
              child: Text(
                'Library error: $e',
                style: theme.typography.body.sm.copyWith(
                  color: colors.destructive,
                ),
              ),
            ),
            data: (items) {
              if (items.isEmpty) {
                return Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  child: Text(
                    'No collections yet. Create a folder or playlist collection.',
                    style: theme.typography.body.sm.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
                );
              }
              return ListView(
                children: [
                  for (final c in items)
                    if (c.kind == _playlistKind)
                      DragTarget<TrackDragPayload>(
                        onWillAcceptWithDetails: (details) =>
                            trackLoadKind(details.data) ==
                            TrackLoadKind.library,
                        onAcceptWithDetails: (details) =>
                            unawaited(_dropOnPlaylist(c, details.data)),
                        builder: (_, candidate, _) => _row(
                          c,
                          selectedId: selectedId,
                          sidebarFocused: sidebarFocused,
                          dropActive: candidate.isNotEmpty,
                        ),
                      )
                    else
                      _row(
                        c,
                        selectedId: selectedId,
                        sidebarFocused: sidebarFocused,
                        dropActive: false,
                      ),
                ],
              );
            },
          ),
        ),
      ],
    );
  }
}
