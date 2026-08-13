import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/library_nav.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

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
        if (currentPath != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(10, 4, 4, 8),
            child: Row(
              children: [
                const LibraryPaneLabel('Browse'),
                const SizedBox(width: 8),
                Expanded(
                  child: _VolumeChip(
                    volume: selectedVolume,
                    path: currentPath,
                    onPress: () =>
                        ref.read(driveCurrentPathProvider.notifier).set(null),
                  ),
                ),
              ],
            ),
          ),
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

/// Tapping returns to the volume list (no Forui select overlay — that froze GTK).
class _VolumeChip extends StatelessWidget {
  const _VolumeChip({
    required this.volume,
    required this.path,
    required this.onPress,
  });

  final FsVolumeInfo? volume;
  final String path;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final label = volume?.name ?? path;
    return FTappable(
      semanticsLabel: 'Choose drive',
      onPress: onPress,
      builder: (context, variants, _) {
        final hovered = variants.contains(FTappableVariant.hovered);
        return DecoratedBox(
          decoration: BoxDecoration(
            borderRadius: theme.style.borderRadius.sm,
            border: Border.all(color: colors.border),
            color: hovered
                ? colors.foreground.withValues(alpha: 0.05)
                : colors.secondary,
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            child: Row(
              children: [
                Icon(
                  volume?.isRemovable == true
                      ? FLucideIcons.usb
                      : FLucideIcons.hardDrive,
                  size: 14,
                  color: colors.mutedForeground,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.typography.body.sm.copyWith(
                      color: colors.foreground,
                    ),
                  ),
                ),
                Icon(
                  FLucideIcons.chevronsUpDown,
                  size: 14,
                  color: colors.mutedForeground,
                ),
              ],
            ),
          ),
        );
      },
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

/// Audio files in the current drive directory (right pane).
class DriveFilesPane extends ConsumerWidget {
  const DriveFilesPane({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final colors = theme.colors;
    final currentPath = ref.watch(driveCurrentPathProvider);
    final listing = ref.watch(driveListingProvider);
    final resolved = ref.watch(driveResolvedByPathProvider);

    if (currentPath == null) {
      return Center(
        child: Text(
          'Select a drive or folder to browse audio files',
          style: theme.typography.body.sm.copyWith(
            color: colors.mutedForeground,
          ),
        ),
      );
    }

    return listing.when(
      loading: () => const Center(child: FCircularProgress()),
      error: (e, _) => Text(
        'Browse error: $e',
        style: theme.typography.body.sm.copyWith(color: colors.destructive),
      ),
      data: (dir) {
        if (dir == null) {
          return const SizedBox.shrink();
        }
        final files = dir.audioFiles;
        if (files.isEmpty) {
          return Center(
            child: Text(
              'No audio files in this folder',
              style: theme.typography.body.sm.copyWith(
                color: colors.mutedForeground,
              ),
            ),
          );
        }
        final byPath =
            resolved.asData?.value ?? const <String, LibraryTrackSummary>{};
        return ListView(
          padding: const EdgeInsets.symmetric(vertical: 8),
          children: [
            for (final f in files)
              LibraryNavRow(
                title: _driveFileTitle(f, byPath),
                subtitle: _driveFileSubtitle(f, byPath),
              ),
          ],
        );
      },
    );
  }

  String _driveFileTitle(
    FsEntry file,
    Map<String, LibraryTrackSummary> byPath,
  ) {
    final track = byPath[file.path];
    if (track != null) {
      return trackTitleLabel(track);
    }
    return file.name;
  }

  String _driveFileSubtitle(
    FsEntry file,
    Map<String, LibraryTrackSummary> byPath,
  ) {
    final track = byPath[file.path];
    if (track?.artist?.isNotEmpty ?? false) {
      return track!.artist!;
    }
    return file.path;
  }
}
