import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/collections_pane.dart';
import 'package:gui_flutter/library/drive_pane.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_table_pane.dart';

/// Library panel: [FTabs](https://forui.dev/docs/widgets/navigation/tabs) for
/// collections vs drive, plus track/drive file panes.
class LibraryPanel extends ConsumerWidget {
  const LibraryPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.watch(libraryEventsBootstrapProvider);
    final theme = context.theme;
    final message = ref.watch(libraryMessageProvider);

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
                    child: Padding(
                      padding: const EdgeInsets.all(12),
                      child: FTabs(
                        expands: true,
                        scrollable: true,
                        children: [
                          FTabEntry(
                            label: const Text('Collections'),
                            child: FResizable(
                              axis: .horizontal,
                              divider: .dividerWithThumb,
                              children: [
                                .fixed(
                                  minExtent: 100,
                                  extent: 200,
                                  builder: _fill,
                                  child: const CollectionsPane(),
                                ),
                                .flex(
                                  flex: 3,
                                  minFlex: 1,
                                  builder: _fill,
                                  child: const TrackTablePane(),
                                ),
                              ],
                            ),
                          ),
                          FTabEntry(
                            label: const Text('Drive'),
                            child: FResizable(
                              axis: .horizontal,
                              divider: .dividerWithThumb,
                              children: [
                                .fixed(
                                  minExtent: 100,
                                  extent: 200,
                                  builder: _fill,
                                  child: const DrivePane(),
                                ),
                                .flex(
                                  flex: 3,
                                  minFlex: 1,
                                  builder: _fill,
                                  child: const DriveFilesPane(),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
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
