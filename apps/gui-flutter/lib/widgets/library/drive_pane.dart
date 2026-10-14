import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/collection_actions.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/widgets/library/create_collection_dialog.dart';
import 'package:gui_flutter/widgets/library/library_nav_row.dart';
import 'package:gui_flutter/widgets/library/library_pane_label.dart';
import 'package:gui_flutter/widgets/shell/m_loader.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Drive sidebar: volume list, then browse select + folder tree (Tauri drive pane).
class DrivePane extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<DrivePane> createState() => _DrivePaneState();
}

class _DrivePaneState extends ConsumerState<DrivePane> {
  var _pickerOpen = false;

  void _openPath(String path) {
    setState(() => _pickerOpen = false);
    ref.read(driveCurrentPathProvider.notifier).set(path);
  }

  Future<void> _createCollection(String folderPath) async {
    final result = await showCreateCollectionDialog(
      context,
      input: CreateCollectionInput(initialFolderPath: folderPath),
    );
    if (result == null || !mounted) {
      return;
    }
    final collection = await createCollection(ref, result);
    if (!mounted || collection == null) {
      return;
    }
    selectCreatedCollection(ref, collection);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final volumes = ref.watch(driveVolumesProvider);
    final currentPath = ref.watch(driveCurrentPathProvider);
    final listing = ref.watch(driveListingProvider);
    final selectedVolume = ref.watch(driveActiveVolumeProvider);
    // The tree row the browse rotary is on, marked only while the sidebar owns
    // focus (the same rule as the other sidebar tabs).
    final focusedPath =
        ref.watch(libraryFocusZoneProvider) == LibraryFocusZone.sidebar
        ? ref.watch(driveTreeFocusedPathProvider)
        : null;
    // Pressing a row parks focus on the sidebar and moves the cursor onto that
    // row, so the marker follows the pointer here as it does in the other panes.
    void pressPath(String path) {
      ref.read(libraryFocusZoneProvider.notifier).set(LibraryFocusZone.sidebar);
      ref.read(driveTreeCursorProvider.notifier).set(path);
      _openPath(path);
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
              if (currentPath == null)
                volumes.when(
                  loading: () => const Center(child: MLoader()),
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
                    onSelect: pressPath,
                    focusedPath: focusedPath,
                    emptyColor: colors.mutedForeground,
                    style: theme.typography.body.sm,
                  ),
                )
              else
                listing.when(
                  loading: () => const Center(child: MLoader()),
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
                    final parentOpenable = driveParentInVolume(
                      dir.parent,
                      selectedVolume,
                    );
                    return ListView(
                      children: [
                        LibraryNavRow(
                          title: currentName,
                          icon: LucideIcons.folder,
                          selected: true,
                          // The "up" row is the current folder, so it already
                          // carries the selection fill; the cursor adds the
                          // outline, exactly as it does in the other panes.
                          // A volume root has no parent row, and a `null`
                          // focused path must not match one.
                          focused:
                              dir.parent != null && focusedPath == dir.parent,
                          onPress: parentOpenable
                              // Same contract as a subfolder press: park focus
                              // on the sidebar and move the cursor onto the
                              // parent row, or the next step starts from a row
                              // the user never pressed.
                              ? () => pressPath(dir.parent!)
                              : null,
                          trailing: _CreateCollectionButton(
                            onPress: () => _createCollection(dir.path),
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
                              icon: LucideIcons.folder,
                              indented: true,
                              focused: focusedPath == d.path,
                              onPress: () => pressPath(d.path),
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
  const new({required this.volume, required this.open, required this.onToggle});

  final FsVolumeInfo? volume;
  final bool open;
  final VoidCallback onToggle;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    return MTappable(
      semanticsLabel: 'Select drive',
      selected: open,
      onPress: onToggle,
      builder: (context, state) {
        return DecoratedBox(
          decoration: BoxDecoration(
            borderRadius: theme.style.borderRadius.sm,
            border: Border.all(color: colors.border),
            color: state.hovered || open ? colors.card : colors.secondary,
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            child: Row(
              children: [
                Icon(
                  volume?.isRemovable == true
                      ? LucideIcons.usb
                      : LucideIcons.hardDrive,
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
                  open ? LucideIcons.chevronUp : LucideIcons.chevronDown,
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
  const new({
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
                        ? LucideIcons.usb
                        : LucideIcons.hardDrive,
                    selected: v.path == selectedPath,
                    onPress: () => onSelect(v.path),
                  ),
              ],
            ),
          ),
        ),
        Expanded(
          child: MTappable.child(
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
  const new({
    required this.volumes,
    required this.onSelect,
    required this.focusedPath,
    required this.emptyColor,
    required this.style,
  });

  final List<FsVolumeInfo> volumes;
  final ValueChanged<String> onSelect;

  /// Path of the row the browse rotary is on, when the sidebar owns focus.
  final String? focusedPath;
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
            icon: v.isRemovable ? LucideIcons.usb : LucideIcons.hardDrive,
            focused: v.path == focusedPath,
            onPress: () => onSelect(v.path),
          ),
      ],
    );
  }
}

class _CreateCollectionButton extends StatelessWidget {
  const new({required this.onPress});

  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return MTappable.child(
      semanticsLabel: 'Create collection',
      onPress: onPress,
      child: Padding(
        padding: const EdgeInsets.all(4),
        child: Icon(
          LucideIcons.folderPlus,
          size: 14,
          color: colors.mutedForeground,
        ),
      ),
    );
  }
}
