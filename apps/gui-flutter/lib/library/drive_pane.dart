import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/library_nav.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';

/// Drive sidebar: volume list, then browse select + folder tree (Tauri drive pane).
class DrivePane extends ConsumerStatefulWidget {
  const DrivePane({super.key});

  @override
  ConsumerState<DrivePane> createState() => _DrivePaneState();
}

class _DrivePaneState extends ConsumerState<DrivePane> {
  var _pickerOpen = false;

  void _openPath(String path) {
    setState(() => _pickerOpen = false);
    ref.read(driveCurrentPathProvider.notifier).set(path);
  }

  void _openParent(FsDirectoryListing dir, FsVolumeInfo? volume) {
    final parent = dir.parent;
    if (parent == null || !_parentInVolume(parent, volume)) {
      return;
    }
    _openPath(parent);
  }

  static bool _parentInVolume(String parent, FsVolumeInfo? volume) {
    if (volume == null) {
      return false;
    }
    if (parent == volume.path) {
      return true;
    }
    if (volume.path == '/') {
      return parent.startsWith('/');
    }
    return parent.startsWith('${volume.path}/');
  }

  Future<void> _createCollection(String folderPath) async {
    ref.read(libraryMessageProvider.notifier).clear();
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      final settings = await ref.read(appSettingsProvider.future);
      final result = await transport.addFolderCollection(
        folderPath: folderPath,
        scanFolderTree: settings.scanFolderTree,
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

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final volumes = ref.watch(driveVolumesProvider);
    final currentPath = ref.watch(driveCurrentPathProvider);
    final listing = ref.watch(driveListingProvider);
    final selectedVolume = ref.watch(driveActiveVolumeProvider);

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
                  child: _VolumeSelectButton(
                    volume: selectedVolume,
                    open: _pickerOpen,
                    onToggle: () => setState(() => _pickerOpen = !_pickerOpen),
                  ),
                ),
              ],
            ),
          ),
        Expanded(
          child: Stack(
            children: [
              currentPath == null
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
                      data: (items) => _VolumeList(
                        volumes: items,
                        onSelect: _openPath,
                        emptyColor: colors.mutedForeground,
                        style: theme.typography.body.sm,
                      ),
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
                        final currentName = selectedVolume?.path == dir.path
                            ? (selectedVolume?.name ?? dir.path)
                            : dir.path.split(RegExp(r'[/\\]')).last;
                        return ListView(
                          children: [
                            LibraryNavRow(
                              title: currentName,
                              icon: FLucideIcons.folder,
                              selected: true,
                              onPress:
                                  dir.parent != null &&
                                      _parentInVolume(
                                        dir.parent!,
                                        selectedVolume,
                                      )
                                  ? () => _openParent(dir, selectedVolume)
                                  : null,
                              trailing: _CreateCollectionButton(
                                onPress: () => _createCollection(dir.path),
                              ),
                            ),
                            if (dir.directories.isEmpty)
                              Padding(
                                padding: const EdgeInsets.fromLTRB(
                                  24,
                                  8,
                                  10,
                                  8,
                                ),
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
                                  onPress: () => _openPath(d.path),
                                  trailing: _CreateCollectionButton(
                                    onPress: () => _createCollection(d.path),
                                  ),
                                ),
                          ],
                        );
                      },
                    ),
              if (_pickerOpen && currentPath != null)
                volumes.when(
                  loading: () => const SizedBox.shrink(),
                  error: (_, _) => const SizedBox.shrink(),
                  data: (items) => _VolumeDropdown(
                    volumes: items,
                    selectedPath: selectedVolume?.path,
                    onSelect: _openPath,
                    onDismiss: () => setState(() => _pickerOpen = false),
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }
}

/// In-tree dropdown (no Overlay/FPortal — those freeze GTK on Linux).
class _VolumeSelectButton extends StatelessWidget {
  const _VolumeSelectButton({
    required this.volume,
    required this.open,
    required this.onToggle,
  });

  final FsVolumeInfo? volume;
  final bool open;
  final VoidCallback onToggle;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    return FTappable(
      semanticsLabel: 'Select drive',
      selected: open,
      onPress: onToggle,
      builder: (context, variants, _) {
        final hovered = variants.contains(FTappableVariant.hovered);
        return DecoratedBox(
          decoration: BoxDecoration(
            borderRadius: theme.style.borderRadius.sm,
            border: Border.all(color: colors.border),
            color: hovered || open
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
                    volume?.name ?? 'Select drive',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.typography.body.sm.copyWith(
                      color: colors.foreground,
                    ),
                  ),
                ),
                Icon(
                  open ? FLucideIcons.chevronUp : FLucideIcons.chevronDown,
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

class _VolumeDropdown extends StatelessWidget {
  const _VolumeDropdown({
    required this.volumes,
    required this.selectedPath,
    required this.onSelect,
    required this.onDismiss,
  });

  final List<FsVolumeInfo> volumes;
  final String? selectedPath;
  final ValueChanged<String> onSelect;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        DecoratedBox(
          decoration: BoxDecoration(
            color: colors.background,
            borderRadius: theme.style.borderRadius.md,
            border: Border.all(color: colors.border),
          ),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 220),
            child: ListView(
              shrinkWrap: true,
              children: [
                for (final v in volumes)
                  LibraryNavRow(
                    title: v.name,
                    subtitle: v.path,
                    icon: v.isRemovable
                        ? FLucideIcons.usb
                        : FLucideIcons.hardDrive,
                    selected: v.path == selectedPath,
                    onPress: () => onSelect(v.path),
                  ),
              ],
            ),
          ),
        ),
        Expanded(
          child: FTappable(
            semanticsLabel: 'Dismiss drive picker',
            onPress: onDismiss,
            child: const ColoredBox(color: Color(0x00000000)),
          ),
        ),
      ],
    );
  }
}

class _VolumeList extends StatelessWidget {
  const _VolumeList({
    required this.volumes,
    required this.onSelect,
    required this.emptyColor,
    required this.style,
  });

  final List<FsVolumeInfo> volumes;
  final ValueChanged<String> onSelect;
  final Color emptyColor;
  final TextStyle style;

  @override
  Widget build(BuildContext context) {
    if (volumes.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(horizontal: 10),
        child: Text(
          'No drives found.',
          style: style.copyWith(color: emptyColor),
        ),
      );
    }
    return ListView(
      children: [
        for (final v in volumes)
          LibraryNavRow(
            title: v.name,
            subtitle: v.path,
            icon: v.isRemovable ? FLucideIcons.usb : FLucideIcons.hardDrive,
            onPress: () => onSelect(v.path),
          ),
      ],
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
