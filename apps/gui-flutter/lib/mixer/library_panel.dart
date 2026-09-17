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
import 'package:gui_flutter/shell/m_card.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:panes/panes.dart';

/// Library panel: left [FTabs](https://forui.dev/docs/widgets/navigation/tabs)
/// (Collections / Drive / History); right pane follows the selected tab.
///
/// Horizontal split sizes are session-local (no [PaneController.save] /
/// [PaneController.load]).
class LibraryPanel extends ConsumerStatefulWidget {
  const LibraryPanel({super.key});

  @override
  ConsumerState<LibraryPanel> createState() => _LibraryPanelState();
}

class _LibraryPanelState extends ConsumerState<LibraryPanel> {
  late final PaneController _controller;

  @override
  void initState() {
    super.initState();
    _controller = PaneController(
      entries: [
        PaneEntry(
          id: 'sidebar',
          initialSize: PaneSize.pixel(240),
          minSize: PaneSize.pixel(240),
        ),
        PaneEntry(id: 'content', initialSize: PaneSize.fraction(1.0)),
      ],
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
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
            // Invisible chrome; keep a grab hit-area (was FResizable divider:none).
            child: PaneTheme(
              data: const PaneThemeData(
                resizerColor: Color(0x00000000),
                resizerHoverColor: Color(0x00000000),
                resizerFocusedColor: Color(0x00000000),
                resizerThickness: 0,
                resizerHitTestThickness: 8,
              ),
              child: MultiPane(
                direction: Axis.horizontal,
                controller: _controller,
                paneBuilder: (context, id, _) => switch (id) {
                  'sidebar' => MCard(
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
                              child: Icon(LucideIcons.library, size: 16),
                            ),
                          ),
                          child: const CollectionsPane(),
                        ),
                        FTabEntry(
                          label: AppTooltip(
                            tip: 'Drive',
                            child: Semantics(
                              label: 'Drive',
                              child: Icon(LucideIcons.hardDrive, size: 16),
                            ),
                          ),
                          child: const DrivePane(),
                        ),
                        FTabEntry(
                          label: AppTooltip(
                            tip: 'History',
                            child: Semantics(
                              label: 'History',
                              child: Icon(LucideIcons.history, size: 16),
                            ),
                          ),
                          child: const HistoryPane(),
                        ),
                      ],
                    ),
                  ),
                  'content' =>
                    tab == LibrarySourceTab.history
                        ? const HistoryDetailPane()
                        : const TrackTablePane(),
                  _ => const SizedBox.shrink(),
                },
              ),
            ),
          ),
        ],
      ),
    );
  }
}
