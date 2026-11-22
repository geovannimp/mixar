import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/fx_bar.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_layout_config.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_layout_solver.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_panes.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_plan.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_slot.dart';
import 'package:gui_flutter/widgets/mixer/library_drawer.dart';
import 'package:panes/panes.dart';

/// A solved split and the layout it belongs to, waiting for the frame that
/// scheduled it. Kept together so a callback cannot pair a newer solution with
/// an older plan or config.
typedef _PendingSplit = ({
  MixerLayoutSolution solution,
  MixerPlan plan,
  MixerLayoutConfig config,
});

/// Mixer page: two fixed pane stacks whose slots hold the panels.
///
/// The stacks never change — band, deck row, bottom slot, FX bar, and inside
/// the deck row deck(s) | centre | deck(s). What changes with settings is what
/// each slot holds, decided by [mixerPlanFor], and how tall each slot is,
/// decided by [solveMixerLayout]. The body never scrolls: when the library
/// would get too little room it moves to the drawer instead, and the deck row
/// takes the space back.
///
/// Sizes are session-local (no [PaneController.save] / [PaneController.load]).
/// The drawer is its own route, so opening it is not a layout concern.
class MixerPage extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<MixerPage> createState() => _MixerPageState();
}

class _MixerPageState extends ConsumerState<MixerPage> {
  late final PaneController _main;
  late final PaneController _content;

  /// The body height and settings the controllers were last solved for.
  (double, MixerLayoutConfig)? _solvedFor;

  /// The split waiting to be applied, after the frame that solved it.
  _PendingSplit? _pending;

  /// Whether the body has built once with a positive height. The first solve
  /// seeds the controllers before their panes exist; later solves must wait
  /// for the frame.
  var _builtWithPositiveHeight = false;

  @override
  void initState() {
    super.initState();
    _main = PaneController(
      entries: [
        PaneEntry(
          id: MixerPaneIds.top,
          initialSize: PaneSize.fraction(0.22),
          minSize: PaneSize.pixel(MixerPaneMetrics.bandMin),
        ),
        PaneEntry(
          id: MixerPaneIds.content,
          initialSize: PaneSize.fraction(0.56),
          minSize: PaneSize.pixel(MixerPaneMetrics.contentFloorTwo),
        ),
        PaneEntry(
          id: MixerPaneIds.bottom,
          initialSize: PaneSize.fraction(0.22),
          minSize: PaneSize.pixel(MixerPaneMetrics.libraryMin),
        ),
        PaneEntry(
          id: MixerPaneIds.bar,
          initialSize: PaneSize.pixel(FxBar.height),
          minSize: PaneSize.pixel(FxBar.height),
          maxSize: PaneSize.pixel(FxBar.height),
        ),
      ],
    );
    _content = PaneController(
      entries: [
        PaneEntry(id: MixerPaneIds.left, initialSize: PaneSize.fraction(1)),
        PaneEntry(
          id: MixerPaneIds.center,
          initialSize: PaneSize.pixel(MixerPaneMetrics.laneWidthTwo),
          minSize: PaneSize.pixel(MixerPaneMetrics.laneWidthTwo),
          maxSize: PaneSize.pixel(MixerPaneMetrics.laneWidthTwo),
        ),
        PaneEntry(id: MixerPaneIds.right, initialSize: PaneSize.fraction(1)),
      ],
    );
  }

  @override
  void dispose() {
    _main.dispose();
    _content.dispose();
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
    final settings =
        ref.watch(appSettingsProvider).value ?? defaultAppSettings();
    // Value type with equality over every field, so the solve key below reads
    // "the same layout" rather than "the same object".
    final config = MixerLayoutConfig.fromSettings(settings);

    ref.listen(libraryDrawerOpenProvider, (_, open) {
      if (!open) {
        return;
      }
      // A drawer route can be on the stack under another one (the track edit
      // drawer); toggling the header there must not stack a second panel.
      if (ref.read(libraryDrawerRouteMountedProvider) != null) {
        return;
      }
      final drawerAvailable =
          settings.libraryDrawerEnabled ||
          ref.read(mixerLibraryForcedDrawerProvider);
      if (!drawerAvailable) {
        return;
      }
      _resetDrawerTuck();
      unawaited(_showDrawer(settings.libraryDrawerSide));
    });

    return LayoutBuilder(
      builder: (context, constraints) {
        final bandFraction =
            _main.getFractionalSize(MixerPaneIds.top) ??
            (MixerPaneMetrics.bandDefault /
                (constraints.maxHeight - (config.showFxBar ? FxBar.height : 0))
                    .clamp(1, double.infinity));
        final solution = solveMixerLayout(
          height: constraints.maxHeight,
          config: config,
          bandFraction: bandFraction,
        );
        final plan = mixerPlanFor(config, solution);

        // Nothing to split and no split yet: mounting the panes into a
        // zero-height body would make the first real frame paint the entry
        // split before the post-frame solve. Wait for the first positive
        // height instead.
        if (_solvedFor == null && constraints.maxHeight <= 0) {
          return const SizedBox.shrink();
        }

        final solvedFor = (constraints.maxHeight.roundToDouble(), config);
        final firstBuild = !_builtWithPositiveHeight;
        _builtWithPositiveHeight = true;
        // A zero-height body has nothing to split; applying it would only move
        // the controllers to a degenerate state before the window exists.
        if (constraints.maxHeight > 0 && _solvedFor != solvedFor) {
          _solvedFor = solvedFor;
          _pending = (solution: solution, plan: plan, config: config);
          if (firstBuild) {
            // Mounting: the panes below are built in this same pass, so nothing
            // is listening yet. Seeding now keeps the entry split from being
            // painted for one frame and snapping to the solved one — the jump
            // a page rebuilt after Settings used to show.
            _seed(solution, plan, config);
          }
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (!mounted) {
              return;
            }
            final pending = _pending;
            if (pending != null) {
              _apply(pending.solution, pending.plan, pending.config);
            }
          });
        }

        return MixerPanes(main: _main, content: _content, plan: plan);
      },
    );
  }

  /// Puts the solved split on the controllers: hidden slots go away, the rest
  /// take their fraction, and the deck row's floor follows the deck count.
  void _apply(
    MixerLayoutSolution solution,
    MixerPlan plan,
    MixerLayoutConfig config,
  ) {
    // The first solve of a mounting page already seeded this exact split before
    // its frame, so this write is a no-op there; later solves need it. One path
    // keeps both cases in step, and the drawer side effects below ride along.
    _seed(solution, plan, config);

    // The library forces the drawer when the body has no room to dock it; a
    // drawer that was opened for that reason closes again once it fits. Only a
    // drawer *this* flag forced is closed, and closing it flips nothing here —
    // `forced` depends on the split alone — so the two cannot ping-pong.
    final forced = config.preferDockedLibrary && !solution.libraryDocked;
    final wasForced = ref.read(mixerLibraryForcedDrawerProvider);
    ref.read(mixerLibraryForcedDrawerProvider.notifier).set(forced);
    if (wasForced && !forced && ref.read(libraryDrawerOpenProvider)) {
      ref.read(libraryDrawerOpenProvider.notifier).set(false);
    }
  }

  /// Writes a solved split onto the controllers: hidden slots go away, the rest
  /// take their fraction, and the deck row's floor follows the deck count.
  ///
  /// Runs before the first paint on a mounting page and after the frame for
  /// later solves.
  void _seed(
    MixerLayoutSolution solution,
    MixerPlan plan,
    MixerLayoutConfig config,
  ) {
    final contentEntry = _main.entries.firstWhere(
      (entry) => entry.id == MixerPaneIds.content,
    );
    _main.updatePane(
      contentEntry.copyWith(
        minSize: PaneSize.pixel(
          config.fourDecks
              ? MixerPaneMetrics.contentFloorFour
              : MixerPaneMetrics.contentFloorTwo,
        ),
      ),
    );
    _setPane(_main, MixerPaneIds.top, solution.topFraction);
    _setPane(_main, MixerPaneIds.content, solution.contentFraction);
    _setPane(_main, MixerPaneIds.bottom, solution.bottomFraction);
    _setVisible(_main, MixerPaneIds.bar, solution.barVisible);

    final row = plan.content;
    final center = row?.center;
    _setVisible(_content, MixerPaneIds.center, center != null);
    if (row != null && center != null) {
      final centerWidth = row.centerWidth;
      final centerEntry = _content.entries.firstWhere(
        (entry) => entry.id == MixerPaneIds.center,
      );
      _content
        ..updatePane(
          centerEntry.copyWith(
            initialSize: PaneSize.pixel(centerWidth),
            minSize: PaneSize.pixel(centerWidth),
            maxSize: PaneSize.pixel(centerWidth),
          ),
        )
        ..updateSize(MixerPaneIds.center, PaneSize.pixel(centerWidth));
    }
  }

  /// [fraction] `null` hides the slot; otherwise it shows at that fraction.
  void _setPane(PaneController controller, String id, double? fraction) {
    if (fraction == null) {
      controller.hide(id);
      return;
    }
    // One cascade, one notification per call; two statements on the same
    // receiver trip `cascade_invocations`.
    controller
      ..show(id)
      ..updateSize(id, PaneSize.fraction(fraction));
  }

  void _setVisible(PaneController controller, String id, bool visible) {
    if (visible) {
      controller.show(id);
    } else {
      controller.hide(id);
    }
  }
}
