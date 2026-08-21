import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/library_nav.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/settings/settings_providers.dart';

/// Collections sidebar: header + full-width rows (Tauri collection list).
class CollectionsPane extends ConsumerStatefulWidget {
  const CollectionsPane({super.key});

  @override
  ConsumerState<CollectionsPane> createState() => _CollectionsPaneState();
}

class _CollectionsPaneState extends ConsumerState<CollectionsPane> {
  var _adding = false;

  Future<void> _addFolder() async {
    final path = await FilePicker.platform.getDirectoryPath();
    if (path == null || !mounted) {
      return;
    }
    setState(() => _adding = true);
    ref.read(libraryMessageProvider.notifier).clear();
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      final settings = await ref.read(appSettingsProvider.future);
      if (!mounted) {
        return;
      }
      final result = await transport.addFolderCollection(
        folderPath: path,
        scanFolderTree: settings.scanFolderTree,
      );
      if (!mounted) {
        return;
      }
      ref.invalidate(collectionsProvider);
      ref.invalidate(collectionTracksProvider);
      ref.read(selectedCollectionIdProvider.notifier).set(result.collection.id);
    } catch (e) {
      if (!mounted) {
        return;
      }
      ref.read(libraryMessageProvider.notifier).setError('$e');
    } finally {
      if (mounted) {
        setState(() => _adding = false);
      }
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

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(10, 4, 4, 8),
          child: Row(
            children: [
              const Expanded(child: LibraryPaneLabel('Collections')),
              FTappable(
                onPress: _adding ? null : _addFolder,
                semanticsLabel: 'Add folder collection',
                builder: (context, variants, _) {
                  final hovered = variants.contains(FTappableVariant.hovered);
                  return Container(
                    width: 24,
                    height: 24,
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      borderRadius: theme.style.borderRadius.sm,
                      border: Border.all(
                        color: colors.primary.withValues(alpha: 0.35),
                      ),
                      color: colors.primary.withValues(
                        alpha: hovered ? 0.20 : 0.12,
                      ),
                    ),
                    child: _adding
                        ? const FCircularProgress(size: .sm)
                        : Text(
                            '+',
                            style: theme.typography.body.sm.copyWith(
                              color: colors.primary,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                  );
                },
              ),
            ],
          ),
        ),
        Expanded(
          child: collections.when(
            loading: () => const Center(child: FCircularProgress()),
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
                    'No collections yet. Add a folder to scan audio files.',
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
                      icon: FLucideIcons.folder,
                      selected: c.id == selectedId,
                      onPress: () => ref
                          .read(selectedCollectionIdProvider.notifier)
                          .set(c.id),
                      trailing: c.kind == 'folder' && c.path != null
                          ? FTappable(
                              semanticsLabel: 'Browse ${c.name} in Drive',
                              onPress: () => _browseInDrive(c.path!),
                              child: Padding(
                                padding: const EdgeInsets.all(4),
                                child: Icon(
                                  FLucideIcons.folderOpen,
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
