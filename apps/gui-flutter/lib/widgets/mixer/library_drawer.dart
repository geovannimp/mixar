import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/library_panel.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// How long the drawer takes to slide in, slide out, and tuck away for a drag.
const _slideDuration = Duration(milliseconds: 250);

/// Backdrop behind the panel: the same 50% black the track edit drawer's
/// default dialog barrier paints, so both drawers read alike.
const _scrimColor = Color(0x80000000);

/// Shows the library drawer from [side] as an edge panel over the mixer.
///
/// A route, so the panel overlays the mixer without reflowing it. Deliberately
/// not `showGeneralDialog`: that route always installs a modal barrier, and the
/// barrier absorbs the very hit tests deck drop targets are resolved through
/// (a `DragTarget` is found by hit testing the pointer position), so no track
/// could ever be dropped while the drawer was open. Flutter's own
/// `BottomSheetRoute` suppresses the barrier the same way.
///
/// A track drag tucks the panel off its edge — keeping it mounted, because the
/// drag source (a row inside it) has to outlive the native drag — so the deck
/// it is dragged to is never covered at drop time. The drag's end closes the
/// drawer, and nothing reopens it: the header toggle is the way back. A deck
/// load that is not a library drag — double-click, the deck picker, or a
/// controller load of the focused row — pops the drawer as the load starts.
Future<void> showLibraryDrawer(
  BuildContext context,
  LibraryDrawerSideSetting side,
) async {
  await Navigator.of(
    context,
    rootNavigator: true,
  ).push<void>(_LibraryDrawerRoute(side: side));
}

/// The drawer's route: a dialog route with no barrier.
class _LibraryDrawerRoute extends RawDialogRoute<void> {
  new({required LibraryDrawerSideSetting side})
    : super(
        pageBuilder: (_, _, _) => _LibraryDrawerPanel(side: side),
        barrierDismissible: false,
        barrierLabel: 'Library',
        barrierColor: null,
        transitionDuration: _slideDuration,
        // The panel slides itself, so the backdrop can stay put: the route
        // installs no page transition.
        transitionBuilder: (context, _, _, child) => child,
      );

  /// Empty, not a `ModalBarrier`: while this route is up the mixer stays
  /// interactive, which is what lets a track be dragged out of the drawer and
  /// dropped on a deck. Nothing is dismissed by a tap — the header toggle owns
  /// closing.
  @override
  Widget buildModalBarrier() => const SizedBox.shrink();
}

/// The unit vector the panel slides along from off-window.
Offset _hiddenOffset(LibraryDrawerSideSetting side) => switch (side) {
  LibraryDrawerSideSetting.left => const Offset(-1, 0),
  LibraryDrawerSideSetting.right => const Offset(1, 0),
  LibraryDrawerSideSetting.top => const Offset(0, -1),
  LibraryDrawerSideSetting.bottom => const Offset(0, 1),
};

/// Where the panel sits: flush to the edge it docks to.
Alignment _edgeAlignment(LibraryDrawerSideSetting side) => switch (side) {
  LibraryDrawerSideSetting.left => Alignment.centerLeft,
  LibraryDrawerSideSetting.right => Alignment.centerRight,
  LibraryDrawerSideSetting.top => Alignment.topCenter,
  LibraryDrawerSideSetting.bottom => Alignment.bottomCenter,
};

/// Where the resize grip sits inside the panel: its inner edge.
Alignment _innerEdgeAlignment(LibraryDrawerSideSetting side) => switch (side) {
  LibraryDrawerSideSetting.left => Alignment.centerRight,
  LibraryDrawerSideSetting.right => Alignment.centerLeft,
  LibraryDrawerSideSetting.top => Alignment.bottomCenter,
  LibraryDrawerSideSetting.bottom => Alignment.topCenter,
};

/// Pinches [value] between the drawer minimum and [extent], so an extent below
/// the minimum cannot invert the range.
double _clamp(double value, double extent) {
  final upper = math.max(kLibraryDrawerMinExtent, extent);
  return math.min(math.max(value, kLibraryDrawerMinExtent), upper);
}

/// The drawer's panel: the library surface, sized along [side] and resizable on
/// its inner edge.
class _LibraryDrawerPanel extends ConsumerStatefulWidget {
  const new({required this.side});

  final LibraryDrawerSideSetting side;

  @override
  ConsumerState<_LibraryDrawerPanel> createState() =>
      _LibraryDrawerPanelState();
}

class _LibraryDrawerPanelState extends ConsumerState<_LibraryDrawerPanel> {
  /// Thickness of the inner-edge grip, mirroring the invisible 8px grips the
  /// `panes` package paints (`resizerHitTestThickness: 8`).
  static const _gripThickness = 8.0;

  bool get _isHorizontal =>
      widget.side == LibraryDrawerSideSetting.left ||
      widget.side == LibraryDrawerSideSetting.right;

  /// Border on the edge facing the mixer; no scrim and no shadow, so the mixer
  /// stays visible around the drawer.
  Border _innerBorder(MixarThemeData theme) {
    final side = BorderSide(
      color: theme.colors.border,
      width: theme.style.borderWidth,
    );
    return switch (widget.side) {
      LibraryDrawerSideSetting.left => Border(right: side),
      LibraryDrawerSideSetting.right => Border(left: side),
      LibraryDrawerSideSetting.top => Border(bottom: side),
      LibraryDrawerSideSetting.bottom => Border(top: side),
    };
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    // Closed by request: the header toggle, or the end of a track drag.
    // `isCurrent` keeps a second pop from taking the route underneath while
    // this one animates out.
    ref.listen(libraryDrawerOpenProvider, (_, open) {
      if (open) {
        return;
      }
      final route = ModalRoute.of(context);
      if (route != null && route.isCurrent) {
        Navigator.of(context).pop();
      }
    });

    final size = ref.watch(libraryDrawerSizeProvider);
    final tucked = ref.watch(libraryTrackDragActiveProvider);
    // Drives the panel's slide and the backdrop's fade.
    final routeAnimation =
        ModalRoute.of(context)?.animation ?? kAlwaysCompleteAnimation;
    final horizontal = _isHorizontal;

    // Inset by the header: the drawer's stage is the body below it, so a panel
    // on any edge — right included — can never cover the window controls.
    return Padding(
      padding: const EdgeInsets.only(top: kAppHeaderHeight),
      // Clipped to that stage. A top or bottom panel starts one full extent
      // outside it and has to travel through the padding, so without the clip
      // it slides across the titlebar on the way in and out.
      child: ClipRect(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final axisExtent = horizontal
                ? constraints.maxWidth
                : constraints.maxHeight;
            // Clamp the rendered extent only: the stored value comes back when
            // the window grows, and a shrunk window never overflows.
            final clamped = _clamp(
              horizontal ? size.width : size.height,
              axisExtent,
            );
            // Where the panel sits in this box, for the dismissal press below.
            // Edge-aligned, so it is plain geometry — no `GlobalKey` lookup.
            final panelRect = switch (widget.side) {
              LibraryDrawerSideSetting.left => Rect.fromLTWH(
                0,
                0,
                clamped,
                constraints.maxHeight,
              ),
              LibraryDrawerSideSetting.right => Rect.fromLTWH(
                constraints.maxWidth - clamped,
                0,
                clamped,
                constraints.maxHeight,
              ),
              LibraryDrawerSideSetting.top => Rect.fromLTWH(
                0,
                0,
                constraints.maxWidth,
                clamped,
              ),
              LibraryDrawerSideSetting.bottom => Rect.fromLTWH(
                0,
                constraints.maxHeight - clamped,
                constraints.maxWidth,
                clamped,
              ),
            };
            return Stack(
              children: [
                // Backdrop over the mixer, like the track edit drawer's. Purely
                // visual: `IgnorePointer` keeps it out of the hit test, which is
                // what keeps deck drop targets reachable, and it fades while
                // a drag tucks the panel aside so the deck stays readable.
                Positioned.fill(
                  child: IgnorePointer(
                    child: AnimatedOpacity(
                      opacity: tucked ? 0 : 1,
                      duration: _slideDuration,
                      curve: Curves.easeOutCubic,
                      child: FadeTransition(
                        opacity: routeAnimation,
                        child: const ColoredBox(color: _scrimColor),
                      ),
                    ),
                  ),
                ),
                SlideTransition(
                  position:
                      Tween<Offset>(
                        begin: _hiddenOffset(widget.side),
                        end: Offset.zero,
                      ).animate(
                        CurvedAnimation(
                          parent: routeAnimation,
                          curve: Curves.easeOutCubic,
                        ),
                      ),
                  child: Align(
                    alignment: _edgeAlignment(widget.side),
                    child: SizedBox(
                      // Spans the window across its axis, resized along it.
                      width: horizontal ? clamped : constraints.maxWidth,
                      height: horizontal ? constraints.maxHeight : clamped,
                      child: AnimatedSlide(
                        // A drag tucks the panel off its edge. It stays mounted —
                        // never unmounted, never popped — because the row being
                        // dragged lives inside it and the native drag dies with
                        // its source.
                        offset: tucked
                            ? _hiddenOffset(widget.side)
                            : Offset.zero,
                        duration: _slideDuration,
                        curve: Curves.easeOutCubic,
                        child: Stack(
                          children: [
                            DecoratedBox(
                              // Foreground, like the split pane rule: the panel
                              // paints its own opaque surface, which would cover
                              // an under-painted border.
                              position: DecorationPosition.foreground,
                              decoration: BoxDecoration(
                                border: _innerBorder(theme),
                              ),
                              child: const LibraryPanel(),
                            ),
                            Positioned.fill(
                              child: Align(
                                alignment: _innerEdgeAlignment(widget.side),
                                child: MouseRegion(
                                  cursor: horizontal
                                      ? SystemMouseCursors.resizeColumn
                                      : SystemMouseCursors.resizeRow,
                                  child: GestureDetector(
                                    behavior: HitTestBehavior.translucent,
                                    onPanUpdate: (details) =>
                                        _resize(details.delta, constraints),
                                    child: SizedBox(
                                      width: horizontal
                                          ? _gripThickness
                                          : double.infinity,
                                      height: horizontal
                                          ? double.infinity
                                          : _gripThickness,
                                    ),
                                  ),
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
                // A press outside the panel closes the drawer. A `Listener`, not
                // a barrier, and translucent: the press still reaches the mixer,
                // and drops are unaffected. An opaque barrier would absorb the
                // very hit tests deck drop targets resolve through
                // (`GestureBinding.hitTest`), blocking drag-to-deck entirely. It
                // covers the body only, so the header controls are untouched.
                Positioned.fill(
                  child: Listener(
                    behavior: HitTestBehavior.translucent,
                    onPointerDown: (event) {
                      if (!panelRect.contains(event.localPosition)) {
                        ref.read(libraryDrawerOpenProvider.notifier).set(false);
                      }
                    },
                    child: const SizedBox.expand(),
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }

  /// Drag on the inner edge. The delta widens the drawer for the left/top
  /// edges and shrinks it for right/bottom, then clamps to the available space.
  void _resize(Offset delta, BoxConstraints constraints) {
    final notifier = ref.read(libraryDrawerSizeProvider.notifier);
    if (_isHorizontal) {
      final growsRight = widget.side == LibraryDrawerSideSetting.left;
      final next =
          ref.read(libraryDrawerSizeProvider).width +
          (growsRight ? delta.dx : -delta.dx);
      notifier.setWidth(_clamp(next, constraints.maxWidth));
    } else {
      final growsDown = widget.side == LibraryDrawerSideSetting.top;
      final next =
          ref.read(libraryDrawerSizeProvider).height +
          (growsDown ? delta.dy : -delta.dy);
      notifier.setHeight(_clamp(next, constraints.maxHeight));
    }
  }
}
