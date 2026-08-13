import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/collections_pane.dart';
import 'package:gui_flutter/library/drive_pane.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_table_pane.dart';

/// Library panel: collections/drive tabs + track table or drive files.
class LibraryPanel extends ConsumerWidget {
  const LibraryPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.watch(libraryEventsBootstrapProvider);
    final theme = context.theme;
    final sourceTab = ref.watch(librarySourceTabProvider);
    final message = ref.watch(libraryMessageProvider);
    final drive = sourceTab == LibrarySourceTab.drive;

    return Padding(
      padding: const EdgeInsets.all(8),
      child: FCard(
        clipBehavior: Clip.antiAlias,
        child: kIsWeb
            ? Center(
                child: Text(
                  'Library browse is desktop-only',
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                ),
              )
            : Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Padding(
                    padding: const EdgeInsets.fromLTRB(12, 12, 12, 0),
                    child: Row(
                      children: [
                        _SourceTabButton(
                          label: 'Collections',
                          selected: !drive,
                          onPress: () => ref
                              .read(librarySourceTabProvider.notifier)
                              .set(LibrarySourceTab.collections),
                        ),
                        const SizedBox(width: 8),
                        _SourceTabButton(
                          label: 'Drive',
                          selected: drive,
                          onPress: () => ref
                              .read(librarySourceTabProvider.notifier)
                              .set(LibrarySourceTab.drive),
                        ),
                      ],
                    ),
                  ),
                  if (message != null)
                    Padding(
                      padding: const EdgeInsets.fromLTRB(12, 8, 12, 0),
                      child: Text(
                        message,
                        style: theme.typography.body.sm.copyWith(
                          color: theme.colors.destructive,
                        ),
                      ),
                    ),
                  Expanded(
                    child: FResizable(
                      axis: .horizontal,
                      divider: .dividerWithThumb,
                      children: [
                        .fixed(
                          minExtent: 100,
                          extent: 200,
                          builder: _fill,
                          child: drive
                              ? const DrivePane()
                              : const CollectionsPane(),
                        ),
                        .flex(
                          flex: 3,
                          minFlex: 1,
                          builder: _fill,
                          child: drive
                              ? const DriveFilesPane()
                              : const TrackTablePane(),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
      ),
    );
  }

  static Widget _fill(BuildContext _, FResizableRegionData _, Widget? child) =>
      child!;
}

class _SourceTabButton extends StatelessWidget {
  const _SourceTabButton({
    required this.label,
    required this.selected,
    required this.onPress,
  });

  final String label;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    return FButton(
      size: .sm,
      variant: selected ? .secondary : .outline,
      onPress: onPress,
      child: Text(label),
    );
  }
}
