import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Drive volume + directory navigation (left pane).
class DrivePane extends ConsumerWidget {
  const DrivePane({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final colors = theme.colors;
    final volumes = ref.watch(driveVolumesProvider);
    final currentPath = ref.watch(driveCurrentPathProvider);
    final listing = ref.watch(driveListingProvider);

    final highlight = Color.alphaBlend(
      colors.foreground.withValues(alpha: 0.1),
      colors.secondary,
    );
    final itemStyle = FItemStyleDelta.delta(
      backgroundColor: .delta([
        .base(colors.background.withValues(alpha: 0.00)),
      ]),
      padding: .value(EdgeInsets.zero),
      contentDecoration: .delta([
        .base(.shapeDelta(color: colors.secondary)),
        .match({.hovered}, .shapeDelta(color: highlight)),
        .match({.pressed}, .shapeDelta(color: highlight)),
      ]),
    );

    void openPath(String path) {
      ref.read(driveCurrentPathProvider.notifier).set(path);
    }

    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Browse', style: theme.typography.display.sm),
          const SizedBox(height: 12),
          Expanded(
            child: currentPath == null
                ? volumes.when(
                    loading: () => const Center(child: FCircularProgress()),
                    error: (e, _) => Text(
                      'Volumes error: $e',
                      style: theme.typography.body.sm.copyWith(
                        color: colors.destructive,
                      ),
                    ),
                    data: (items) {
                      if (items.isEmpty) {
                        return Text(
                          'No volumes found',
                          style: theme.typography.body.sm.copyWith(
                            color: colors.mutedForeground,
                          ),
                        );
                      }
                      return FItemGroup(
                        children: [
                          for (final v in items)
                            FItem(
                              title: Text(v.name),
                              subtitle: Text(v.path),
                              style: itemStyle,
                              onPress: () => openPath(v.path),
                            ),
                        ],
                      );
                    },
                  )
                : listing.when(
                    loading: () => const Center(child: FCircularProgress()),
                    error: (e, _) => Text(
                      'Browse error: $e',
                      style: theme.typography.body.sm.copyWith(
                        color: colors.destructive,
                      ),
                    ),
                    data: (dir) {
                      if (dir == null) {
                        return const SizedBox.shrink();
                      }
                      return FItemGroup(
                        children: [
                          if (dir.parent != null)
                            FItem(
                              title: const Text('..'),
                              subtitle: const Text('Parent folder'),
                              style: itemStyle,
                              onPress: () => openPath(dir.parent!),
                            ),
                          for (final d in dir.directories)
                            FItem(
                              title: Text(d.name),
                              style: itemStyle,
                              onPress: () => openPath(d.path),
                            ),
                        ],
                      );
                    },
                  ),
          ),
        ],
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

    final highlight = Color.alphaBlend(
      colors.foreground.withValues(alpha: 0.1),
      colors.secondary,
    );
    final itemStyle = FItemStyleDelta.delta(
      backgroundColor: .delta([
        .base(colors.background.withValues(alpha: 0.00)),
      ]),
      padding: .value(EdgeInsets.zero),
      contentDecoration: .delta([
        .base(.shapeDelta(color: colors.secondary)),
        .match({.hovered}, .shapeDelta(color: highlight)),
      ]),
    );

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
        style: theme.typography.body.sm.copyWith(
          color: colors.destructive,
        ),
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
        return resolved.when(
          loading: () => const Center(child: FCircularProgress()),
          error: (e, _) => Text(
            'Resolve error: $e',
            style: theme.typography.body.sm.copyWith(
              color: colors.destructive,
            ),
          ),
          data: (byPath) => FItemGroup(
            children: [
              for (final f in files)
                FItem(
                  title: Text(_driveFileTitle(f, byPath)),
                  subtitle: Text(_driveFileSubtitle(f, byPath)),
                  style: itemStyle,
                ),
            ],
          ),
        );
      },
    );
  }

  String _driveFileTitle(FsEntry file, Map<String, LibraryTrackSummary> byPath) {
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
