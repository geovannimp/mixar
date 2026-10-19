import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/library_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Overlay library drawer: slides in from [side], resizable on its inner edge.
///
/// Overlay only: the mixer never reflows, and stays visible and interactive
/// around the drawer. While a track drag is in flight the drawer slides off its
/// edge — and stays mounted, only translated, so the drag source survives — so
/// a deck drop zone is never covered at drop time.
class LibraryDrawer extends ConsumerStatefulWidget {
  const new({required this.side, super.key});

  final LibraryDrawerSideSetting side;

  @override
  ConsumerState<LibraryDrawer> createState() => _LibraryDrawerState();
}

class _LibraryDrawerState extends ConsumerState<LibraryDrawer>
    with SingleTickerProviderStateMixin {
  /// Same duration family as `MultiPane`'s default size animation.
  static const _slideDuration = Duration(milliseconds: 250);

  /// Thickness of the inner-edge resize grip, mirroring the invisible 8px
  /// grips the `panes` package paints (`resizerHitTestThickness: 8`).
  static const _gripThickness = 8.0;

  /// Slide progress along the docked edge: 0 fully off, 1 fully open.
  late final AnimationController _slide;

  @override
  void initState() {
    super.initState();
    _slide = AnimationController(
      duration: _slideDuration,
      vsync: this,
      value: ref.read(libraryDrawerVisibleProvider) ? 1 : 0,
    );
  }

  @override
  void dispose() {
    _slide.dispose();
    super.dispose();
  }

  bool get _isHorizontal =>
      widget.side == LibraryDrawerSideSetting.left ||
      widget.side == LibraryDrawerSideSetting.right;

  /// The unit vector the panel is translated along to hide it off its edge.
  Offset get _hiddenOffset => switch (widget.side) {
    LibraryDrawerSideSetting.left => const Offset(-1, 0),
    LibraryDrawerSideSetting.right => const Offset(1, 0),
    LibraryDrawerSideSetting.top => const Offset(0, -1),
    LibraryDrawerSideSetting.bottom => const Offset(0, 1),
  };

  /// Where the resize grip sits inside the panel: its inner edge.
  Alignment get _innerAlignment => switch (widget.side) {
    LibraryDrawerSideSetting.left => Alignment.centerRight,
    LibraryDrawerSideSetting.right => Alignment.centerLeft,
    LibraryDrawerSideSetting.top => Alignment.bottomCenter,
    LibraryDrawerSideSetting.bottom => Alignment.topCenter,
  };

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

  /// Pinches [value] between the drawer minimum and [extent], so an extent
  /// below the minimum cannot invert the range.
  static double _clamp(double value, double extent) {
    final upper = math.max(kLibraryDrawerMinExtent, extent);
    return math.min(math.max(value, kLibraryDrawerMinExtent), upper);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    // `open` is the user's intent; `visible` is what actually renders, so a
    // track drag can hide the drawer without closing it. The animation follows
    // the latter.
    final open = ref.watch(libraryDrawerOpenProvider);
    final visible = ref.watch(libraryDrawerVisibleProvider);
    ref.listen(libraryDrawerVisibleProvider, (_, next) {
      if (next) {
        _slide.forward();
      } else {
        _slide.reverse();
      }
    });
    final size = ref.watch(libraryDrawerSizeProvider);
    final horizontal = _isHorizontal;

    return LayoutBuilder(
      builder: (context, constraints) {
        final extent = horizontal
            ? constraints.maxWidth
            : constraints.maxHeight;
        // Clamp the rendered extent only: the stored value comes back when the
        // window grows, and a shrunk window never overflows.
        final clamped = _clamp(horizontal ? size.width : size.height, extent);
        // Canonical const, so the ticking animation rebuilds only the
        // translation and positioning around it, not the panel.
        const panel = LibraryPanel();
        final framed = DecoratedBox(
          decoration: BoxDecoration(border: _innerBorder(theme)),
          child: panel,
        );
        return AnimatedBuilder(
          animation: _slide,
          child: framed,
          builder: (context, child) {
            // Closed by the user — not merely hidden by a drag: unmount the
            // library subtree so nothing off-screen can hold focus. While a
            // drag hides it the subtree stays mounted and is only translated.
            if (!open && _slide.value == 0) {
              return const SizedBox.shrink();
            }
            final hidden = _hiddenOffset;
            final progress = _slide.value;
            final translated = FractionalTranslation(
              translation: Offset(
                hidden.dx * (1 - progress),
                hidden.dy * (1 - progress),
              ),
              child: child,
            );
            return Stack(
              children: [
                _pinned(extent: clamped, child: translated),
                if (visible && progress == 1)
                  _pinned(
                    extent: clamped,
                    child: Align(
                      alignment: _innerAlignment,
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
            );
          },
        );
      },
    );
  }

  /// Positions [child] against the edge the drawer docks to, at [extent] on the
  /// drawer's own axis and spanning the other axis.
  Widget _pinned({required double extent, required Widget child}) {
    return switch (widget.side) {
      LibraryDrawerSideSetting.left => Positioned(
        left: 0,
        top: 0,
        bottom: 0,
        width: extent,
        child: child,
      ),
      LibraryDrawerSideSetting.right => Positioned(
        right: 0,
        top: 0,
        bottom: 0,
        width: extent,
        child: child,
      ),
      LibraryDrawerSideSetting.top => Positioned(
        left: 0,
        right: 0,
        top: 0,
        height: extent,
        child: child,
      ),
      LibraryDrawerSideSetting.bottom => Positioned(
        left: 0,
        right: 0,
        bottom: 0,
        height: extent,
        child: child,
      ),
    };
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
