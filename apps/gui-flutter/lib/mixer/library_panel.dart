import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/collections_pane.dart';
import 'package:gui_flutter/library/drive_pane.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_table_pane.dart';

/// Library panel: left [FTabs](https://forui.dev/docs/widgets/navigation/tabs)
/// (Collections / Drive); right pane follows the selected tab.
class LibraryPanel extends ConsumerWidget {
  const LibraryPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.watch(libraryEventsBootstrapProvider);
    final theme = context.theme;
    final message = ref.watch(libraryMessageProvider);
    final drive =
        ref.watch(librarySourceTabProvider) == LibrarySourceTab.drive;

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
                  if (message != null)
                    Padding(
                      padding: const EdgeInsets.fromLTRB(12, 12, 12, 0),
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
                          minExtent: 140,
                          extent: 220,
                          builder: _fill,
                          child: Padding(
                            padding: const EdgeInsets.fromLTRB(8, 8, 0, 8),
                            child: FTabs(
                              // Equal-width tabs (Account/Password style); scrollable shrinks to labels.
                              expands: true,
                              onPress: (index) {
                                ref.read(librarySourceTabProvider.notifier).set(
                                  index == 1
                                      ? LibrarySourceTab.drive
                                      : LibrarySourceTab.collections,
                                );
                              },
                              children: const [
                                FTabEntry(
                                  label: Text('Collections'),
                                  child: CollectionsPane(),
                                ),
                                FTabEntry(
                                  label: Text('Drive'),
                                  child: DrivePane(),
                                ),
                              ],
                            ),
                          ),
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
