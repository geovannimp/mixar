import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/widgets/library/collections_pane.dart';
import 'package:gui_flutter/widgets/library/drive_pane.dart';
import 'package:gui_flutter/widgets/library/history_detail_pane.dart';
import 'package:gui_flutter/widgets/library/history_pane.dart';
import 'package:gui_flutter/widgets/library/library_layout_scope.dart';
import 'package:gui_flutter/widgets/library/track_list_pane.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/m_tabs.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:panes/panes.dart';

/// Library panel: left [MTabs] (Collections / Drive / History); right pane
/// follows the selected tab.
///
/// Its own width decides the layout: at or above [kLibraryWideMinWidth] the two
/// panes sit side by side; below it the surface pages between the sidebar list
/// and the content pane (see [LibraryNarrowPage]). The surface's width, not the
/// window's, so the same rule covers the docked panel and the drawer.
///
/// Horizontal split sizes are session-local (no [PaneController.save] /
/// [PaneController.load]).
class LibraryPanel extends ConsumerStatefulWidget {
  const new({super.key});

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
        PaneEntry(id: 'content', initialSize: PaneSize.fraction(1)),
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
    final theme = context.theme;

    // Invisible chrome; keep a grab hit-area (was FResizable divider:none).
    return PaneTheme(
      data: const PaneThemeData(
        resizerColor: Color(0x00000000),
        resizerHoverColor: Color(0x00000000),
        resizerFocusedColor: Color(0x00000000),
        resizerThickness: 0,
        resizerHitTestThickness: 8,
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final narrow = constraints.maxWidth < kLibraryWideMinWidth;
          return LibraryLayoutScope(
            narrow: narrow,
            child: narrow
                ? _NarrowLibrary(
                    sidebar: _librarySidebar(),
                    content: _contentPane(),
                  )
                // The controller stays owned by this State, so dragged split
                // sizes survive a narrow/wide round trip.
                : MultiPane(
                    direction: Axis.horizontal,
                    controller: _controller,
                    paneBuilder: (context, id, _) => switch (id) {
                      // Flush sidebar: no margin or rounded chrome, only a
                      // right border separating it from the track table. The
                      // border is a foreground decoration so it stays
                      // continuous across the tab bar, and split-only chrome —
                      // the narrow pages draw none. The sidebar and content
                      // share the `secondary` chrome fill, which sits against
                      // the list's `card` surface.
                      'sidebar' => DecoratedBox(
                        position: DecorationPosition.foreground,
                        decoration: BoxDecoration(
                          border: Border(
                            right: BorderSide(
                              color: theme.colors.border,
                              width: theme.style.borderWidth,
                            ),
                          ),
                        ),
                        child: _librarySidebar(),
                      ),
                      'content' => _contentPane(),
                      _ => const SizedBox.shrink(),
                    },
                  ),
          );
        },
      ),
    );
  }

  /// Sidebar pane body: the Collections / Drive / History tabs.
  Widget _librarySidebar() {
    final theme = context.theme;
    final tab = ref.watch(librarySourceTabProvider);
    return ColoredBox(
      color: theme.colors.secondary,
      child: MTabs(
        expands: true,
        spacing: 4,
        barBottomBorder: BorderSide(
          color: theme.colors.border,
          width: theme.style.borderWidth,
        ),
        index: switch (tab) {
          LibrarySourceTab.collections => 0,
          LibrarySourceTab.drive => 1,
          LibrarySourceTab.history => 2,
        },
        onChange: (index) {
          ref.read(librarySourceTabProvider.notifier).set(switch (index) {
            1 => LibrarySourceTab.drive,
            2 => LibrarySourceTab.history,
            _ => LibrarySourceTab.collections,
          });
        },
        children: [
          MTabEntry(
            label: AppTooltip(
              tip: 'Collections',
              child: Semantics(
                label: 'Collections',
                child: const Icon(LucideIcons.library, size: 16),
              ),
            ),
            child: const CollectionsPane(),
          ),
          MTabEntry(
            label: AppTooltip(
              tip: 'Drive',
              child: Semantics(
                label: 'Drive',
                child: const Icon(LucideIcons.hardDrive, size: 16),
              ),
            ),
            child: const DrivePane(),
          ),
          MTabEntry(
            label: AppTooltip(
              tip: 'History',
              child: Semantics(
                label: 'History',
                child: const Icon(LucideIcons.history, size: 16),
              ),
            ),
            child: const HistoryPane(),
          ),
        ],
      ),
    );
  }

  /// Content pane body: the track list, or the history detail when the History
  /// tab is active.
  Widget _contentPane() {
    final theme = context.theme;
    final tab = ref.watch(librarySourceTabProvider);
    return ColoredBox(
      color: theme.colors.secondary,
      child: tab == LibrarySourceTab.history
          ? const HistoryDetailPane()
          : const TrackListPane(),
    );
  }
}

/// Narrow library surface: one page at a time — the sidebar list, or the
/// content pane a row press navigates to.
class _NarrowLibrary extends ConsumerStatefulWidget {
  const new({required this.sidebar, required this.content});

  final Widget sidebar;
  final Widget content;

  @override
  ConsumerState<_NarrowLibrary> createState() => _NarrowLibraryState();
}

class _NarrowLibraryState extends ConsumerState<_NarrowLibrary> {
  /// Keeps a controller / MIDI focus step in sync with the page: on a narrow
  /// surface a pane *is* a page, so stepping focus also pages. Closed in
  /// [dispose] so the provider does not keep this state alive after it is gone.
  ProviderSubscription<LibraryFocusZone>? _zoneListen;

  @override
  void initState() {
    super.initState();
    _zoneListen = ref.listenManual(libraryFocusZoneProvider, (_, zone) {
      ref
          .read(libraryNarrowPageProvider.notifier)
          .set(
            zone == LibraryFocusZone.sidebar
                ? LibraryNarrowPage.list
                : LibraryNarrowPage.content,
          );
    });
  }

  @override
  void dispose() {
    _zoneListen?.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ref.watch(libraryNarrowPageProvider) == LibraryNarrowPage.list
        ? widget.sidebar
        : widget.content;
  }
}
