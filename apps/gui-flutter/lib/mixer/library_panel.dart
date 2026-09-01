import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/collections_pane.dart';
import 'package:gui_flutter/library/drive_pane.dart';
import 'package:gui_flutter/library/history_detail_pane.dart';
import 'package:gui_flutter/library/history_pane.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_table_pane.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';

/// Library panel: left [FTabs](https://forui.dev/docs/widgets/navigation/tabs)
/// (Collections / Drive / History); right pane follows the selected tab.
class LibraryPanel extends ConsumerWidget {
  const LibraryPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.watch(libraryEventsBootstrapProvider);
    ref.watch(historySettingsBootstrapProvider);
    final theme = context.theme;
    final message = ref.watch(libraryMessageProvider);
    final tab = ref.watch(librarySourceTabProvider);

    return Padding(
      padding: const EdgeInsets.all(8),
      child: Column(
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
              divider: .none,
              children: [
                .fixed(
                  minExtent: 240,
                  extent: 240,
                  builder: _fill,
                  child: FCard(
                    clipBehavior: Clip.antiAlias,
                    child: FTabs(
                      expands: true,
                      style: .delta(
                        spacing: 4,
                        indicatorSize: .tab,
                        minHeight: 28,
                        decoration: DecorationDelta.boxDelta(
                          borderRadius: BorderRadius.zero,
                        ),
                      ),
                      control: .lifted(
                        index: switch (tab) {
                          LibrarySourceTab.collections => 0,
                          LibrarySourceTab.drive => 1,
                          LibrarySourceTab.history => 2,
                        },
                        onChange: (index) {
                          ref.read(librarySourceTabProvider.notifier).set(
                            switch (index) {
                              1 => LibrarySourceTab.drive,
                              2 => LibrarySourceTab.history,
                              _ => LibrarySourceTab.collections,
                            },
                          );
                        },
                      ),
                      children: [
                        FTabEntry(
                          label: AppTooltip(
                            tip: 'Collections',
                            child: Semantics(
                              label: 'Collections',
                              child: Icon(FLucideIcons.library, size: 16),
                            ),
                          ),
                          child: const CollectionsPane(),
                        ),
                        FTabEntry(
                          label: AppTooltip(
                            tip: 'Drive',
                            child: Semantics(
                              label: 'Drive',
                              child: Icon(FLucideIcons.hardDrive, size: 16),
                            ),
                          ),
                          child: const DrivePane(),
                        ),
                        FTabEntry(
                          label: AppTooltip(
                            tip: 'History',
                            child: Semantics(
                              label: 'History',
                              child: Icon(FLucideIcons.history, size: 16),
                            ),
                          ),
                          child: const HistoryPane(),
                        ),
                      ],
                    ),
                  ),
                ),
                .flex(
                  flex: 3,
                  minFlex: 1,
                  builder: _fill,
                  child: tab == LibrarySourceTab.history
                      ? const HistoryDetailPane()
                      : const TrackTablePane(),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  static Widget _fill(BuildContext _, FResizableRegionData _, Widget? child) =>
      SizedBox.expand(child: child);
}
