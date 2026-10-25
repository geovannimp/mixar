import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/deck_grid.dart';
import 'package:gui_flutter/widgets/mixer/fx_bar.dart';
import 'package:gui_flutter/widgets/mixer/library_drawer.dart';
import 'package:gui_flutter/widgets/mixer/library_panel.dart';
import 'package:gui_flutter/widgets/mixer/waveform_section.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:panes/panes.dart';

/// Mixer page with resizable regions ([MultiPane]).
///
/// Vertical: waveforms | (fixed decks + library). Decks are not a resizable
/// region — only the waveform/library split moves. Sizes are session-local
/// (no [PaneController.save] / [PaneController.load]).
///
/// When the library drawer setting is on, the docked library is replaced by the
/// [showLibraryDrawer] route and the deck row fills the pane; the drawer itself
/// lives outside this layout. The pane ids and controller stay the same, so
/// pane sizes survive the mode switch.
class MixerPage extends ConsumerStatefulWidget {
  const new({super.key});

  static const _waveformDefault = 160.0;
  static const _waveformMin = 110.0;

  @override
  ConsumerState<MixerPage> createState() => _MixerPageState();
}

class _MixerPageState extends ConsumerState<MixerPage> {
  late final PaneController _controller;
  final _compactScroll = ScrollController();

  @override
  void initState() {
    super.initState();
    _controller = PaneController(
      entries: [
        PaneEntry(
          id: 'waveforms',
          initialSize: PaneSize.pixel(MixerPage._waveformDefault),
          minSize: PaneSize.pixel(MixerPage._waveformMin),
        ),
        PaneEntry(id: 'decks_library', initialSize: PaneSize.fraction(1)),
      ],
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    _compactScroll.dispose();
    super.dispose();
  }

  /// Opens the drawer route, then re-syncs the open flag once it closes —
  /// whatever closed it (a press outside, a track drag, a system back) — so
  /// the next toggle press opens it in one go.
  ///
  /// The drag's tuck is *not* released here: `Route.popped` completes when the
  /// pop starts, so releasing it now would drag the panel back into view while
  /// the route still animates out — the route slides the window height, the
  /// tuck only the panel height, so mid-close the two leave the panel on
  /// screen. It is released when the drawer is next opened instead.
  Future<void> _showDrawer(LibraryDrawerSideSetting side) async {
    await showLibraryDrawer(context, side);
    // Completes when the pop *starts*, so no newer open can be waiting on this
    // continuation: the flag is already false by the time it runs.
    if (mounted) {
      ref.read(libraryDrawerOpenProvider.notifier).set(false);
    }
  }

  /// A fresh open always starts untucked: a drag leaves the panel tucked as it
  /// closes, and that must not carry over.
  void _resetDrawerTuck() {
    ref.read(libraryTrackDragActiveProvider.notifier).end();
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final settingsState = ref.watch(appSettingsProvider);
    final settings = settingsState.value ?? defaultAppSettings();
    final drawerMode = settings.libraryDrawerEnabled;
    final deckRowHeight = settings.numDecks == 4 ? 560.0 : 410.0;

    // The drawer is its own route, so opening it is not a layout concern: the
    // panes below only decide whether the docked library is there at all.
    ref.listen(libraryDrawerOpenProvider, (_, open) {
      if (open && drawerMode) {
        _resetDrawerTuck();
        unawaited(_showDrawer(settings.libraryDrawerSide));
      }
    });

    // Invisible chrome; keep a grab hit-area (was library divider:none).
    return PaneTheme(
      data: const PaneThemeData(
        resizerColor: Color(0x00000000),
        resizerHoverColor: Color(0x00000000),
        resizerFocusedColor: Color(0x00000000),
        resizerThickness: 0,
        resizerHitTestThickness: 8,
      ),
      child: MultiPane(
        direction: Axis.vertical,
        controller: _controller,
        paneBuilder: (context, id, _) => switch (id) {
          'waveforms' => const WaveformSection(),
          'decks_library' => ColoredBox(
            color: theme.colors.card,
            child: LayoutBuilder(
              builder: (context, constraints) {
                final minHeight = drawerMode
                    ? constraints.maxHeight
                    : deckRowHeight +
                          FxBar.height +
                          120 +
                          2 * theme.style.borderWidth;
                final height = constraints.maxHeight < minHeight
                    ? minHeight
                    : constraints.maxHeight;
                final scrollBehavior = ScrollConfiguration.of(context);
                return ScrollConfiguration(
                  // Pad presses must never double as scroll drags; the library
                  // is re-wrapped below so its lists keep default drag scrolling.
                  behavior: scrollBehavior.copyWith(dragDevices: const {}),
                  child: RawScrollbar(
                    controller: _compactScroll,
                    interactive: true,
                    thumbVisibility: true,
                    child: SingleChildScrollView(
                      controller: _compactScroll,
                      child: SizedBox(
                        height: height,
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            // In drawer mode the deck row fills the pane: the library moves
                            // to the drawer, so there is no split and no dead band below
                            // the decks.
                            if (drawerMode)
                              const Expanded(child: ClipRect(child: DeckGrid()))
                            else ...[
                              SizedBox(
                                height: deckRowHeight,
                                child: const ClipRect(child: DeckGrid()),
                              ),
                              // Split the library from the mixer above it in the same
                              // border colour as the rest of the library chrome.
                              MDivider(
                                padding: .zero,
                                color: theme.colors.border,
                              ),
                              Expanded(
                                child: ScrollConfiguration(
                                  behavior: scrollBehavior,
                                  child: const LibraryPanel(),
                                ),
                              ),
                            ],
                            // The FX bar rides at the pane's foot in both layouts.
                            MDivider(
                              padding: .zero,
                              color: theme.colors.border,
                            ),
                            const SizedBox(
                              height: FxBar.height,
                              child: FxBar(),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                );
              },
            ),
          ),
          _ => const SizedBox.shrink(),
        },
      ),
    );
  }
}
