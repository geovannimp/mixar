import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/library_nav.dart';
import 'package:gui_flutter/library/providers.dart';

/// Drive sidebar: volume list, then browse select + folder tree (Tauri drive pane).
class DrivePane extends ConsumerWidget {
  const DrivePane({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final colors = theme.colors;
    final volumes = ref.watch(driveVolumesProvider);
    final currentPath = ref.watch(driveCurrentPathProvider);
    final listing = ref.watch(driveListingProvider);
    final selectedVolume = ref.watch(driveActiveVolumeProvider);

    void openPath(String path) {
      ref.read(driveCurrentPathProvider.notifier).set(path);
    }

    Future<void> createCollection(String folderPath) async {
      ref.read(libraryMessageProvider.notifier).clear();
      try {
        final transport = await ref.read(libraryTransportProvider.future);
        final result = await transport.addFolderCollection(
          folderPath: folderPath,
        );
        ref.invalidate(collectionsProvider);
        ref.invalidate(collectionTracksProvider);
        ref.read(selectedCollectionIdProvider.notifier).set(result.collection.id);
        ref
            .read(librarySourceTabProvider.notifier)
            .set(LibrarySourceTab.collections);
      } catch (e) {
        ref.read(libraryMessageProvider.notifier).setError('$e');
      }
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (currentPath != null) const _DriveBrowseHeader(),
        Expanded(
          child: currentPath == null
              ? volumes.when(
                  loading: () => const Center(child: FCircularProgress()),
                  error: (e, _) => Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 10),
                    child: Text(
                      'Volumes error: $e',
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
                          'No drives found.',
                          style: theme.typography.body.sm.copyWith(
                            color: colors.mutedForeground,
                          ),
                        ),
                      );
                    }
                    return ListView(
                      children: [
                        for (final v in items)
                          LibraryNavRow(
                            title: v.name,
                            subtitle: v.path,
                            icon: v.isRemovable
                                ? FLucideIcons.usb
                                : FLucideIcons.hardDrive,
                            onPress: () => openPath(v.path),
                          ),
                      ],
                    );
                  },
                )
              : listing.when(
                  loading: () => const Center(child: FCircularProgress()),
                  error: (e, _) => Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 10),
                    child: Text(
                      'Browse error: $e',
                      style: theme.typography.body.sm.copyWith(
                        color: colors.destructive,
                      ),
                    ),
                  ),
                  data: (dir) {
                    if (dir == null) {
                      return const SizedBox.shrink();
                    }
                    final currentName =
                        selectedVolume?.path == dir.path
                        ? (selectedVolume?.name ?? dir.path)
                        : dir.path.split(RegExp(r'[/\\]')).last;
                    return ListView(
                      children: [
                        LibraryNavRow(
                          title: currentName,
                          icon: FLucideIcons.folder,
                          selected: true,
                          trailing: _CreateCollectionButton(
                            onPress: () => createCollection(dir.path),
                          ),
                        ),
                        if (dir.directories.isEmpty)
                          Padding(
                            padding: const EdgeInsets.fromLTRB(24, 8, 10, 8),
                            child: Text(
                              'No subfolders here.',
                              style: theme.typography.body.sm.copyWith(
                                color: colors.mutedForeground,
                              ),
                            ),
                          )
                        else
                          for (final d in dir.directories)
                            LibraryNavRow(
                              title: d.name,
                              icon: FLucideIcons.folder,
                              indented: true,
                              onPress: () => openPath(d.path),
                              trailing: _CreateCollectionButton(
                                onPress: () => createCollection(d.path),
                              ),
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

/// Volume [FSelect](https://forui.dev/docs/widgets/form/select); isolated from
/// listing rebuilds so the popover does not remount mid-browse.
class _DriveBrowseHeader extends ConsumerWidget {
  const _DriveBrowseHeader();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final volumes = ref.watch(driveVolumesProvider).asData?.value ?? [];
    final selected = ref.watch(driveActiveVolumeProvider);
    return Padding(
      padding: const EdgeInsets.fromLTRB(10, 4, 4, 8),
      child: Row(
        children: [
          const LibraryPaneLabel('Browse'),
          const SizedBox(width: 8),
          Expanded(
            child: volumes.isEmpty
                ? const SizedBox.shrink()
                : FSelect<String>.rich(
                    size: .sm,
                    hint: 'Select drive',
                    contentCutout: false,
                    format: (path) {
                      for (final v in volumes) {
                        if (v.path == path) {
                          return v.name;
                        }
                      }
                      return path;
                    },
                    control: .lifted(
                      value: selected?.path,
                      onChange: (path) {
                        if (path == null) {
                          return;
                        }
                        ref.read(driveCurrentPathProvider.notifier).set(path);
                      },
                    ),
                    prefixBuilder: (context, style, variants) {
                      final removable = selected?.isRemovable == true;
                      return Padding(
                        padding: const EdgeInsets.only(left: 8),
                        child: Icon(
                          removable
                              ? FLucideIcons.usb
                              : FLucideIcons.hardDrive,
                          size: 14,
                          color: context.theme.colors.mutedForeground,
                        ),
                      );
                    },
                    children: [
                      for (final v in volumes)
                        FSelectItem(
                          value: v.path,
                          prefix: Icon(
                            v.isRemovable
                                ? FLucideIcons.usb
                                : FLucideIcons.hardDrive,
                            size: 16,
                          ),
                          title: Text(v.name),
                        ),
                    ],
                  ),
          ),
        ],
      ),
    );
  }
}

class _CreateCollectionButton extends StatelessWidget {
  const _CreateCollectionButton({required this.onPress});

  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return FTappable(
      semanticsLabel: 'Create collection',
      onPress: onPress,
      child: Padding(
        padding: const EdgeInsets.all(4),
        child: Icon(
          FLucideIcons.folderPlus,
          size: 14,
          color: colors.mutedForeground,
        ),
      ),
    );
  }
}
