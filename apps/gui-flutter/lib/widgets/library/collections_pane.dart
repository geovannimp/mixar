import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/collection_actions.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/widgets/library/create_collection_dialog.dart';
import 'package:gui_flutter/widgets/library/library_nav_row.dart';
import 'package:gui_flutter/widgets/library/library_pane_label.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/m_loader.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
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
                    LibraryNavRow(
                      title: c.name,
                      subtitle: '${c.trackCount} tracks',
                      icon: LucideIcons.folder,
                      selected: c.id == selectedId,
                      focused: sidebarFocused && c.id == selectedId,
                      onPress: () {
                        // Focus follows the pointer, the way a row press in the
                        // track list does.
                        ref
                            .read(libraryFocusZoneProvider.notifier)
                            .set(LibraryFocusZone.sidebar);
                        ref
                            .read(selectedCollectionIdProvider.notifier)
                            .set(c.id);
                      },
                      trailing: c.kind == 'folder' && c.path != null
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
