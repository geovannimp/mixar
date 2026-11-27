import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Bordered panel chrome (Mixar card colors; square corner language).
class MCard extends StatelessWidget {
  const new({
    required this.child,
    this.clipBehavior = Clip.none,
    this.borderColor,
    this.outlineInset = EdgeInsets.zero,
    this.sideBorders = true,
    this.topBorder = true,
    super.key,
  });

  final Widget child;
  final Clip clipBehavior;

  /// When set, replaces the default hairline color and closes all four edges
  /// (e.g. the deck's drop-hover outline), regardless of [sideBorders].
  final Color? borderColor;

  /// Insets the closed outline [borderColor] draws, away from the card's
  /// edges. The resting hairlines stay flush. Use it where the card runs into
  /// chrome painted over its edge — the window frame along a deck's outer
  /// edge, say — so the outline is not cropped there.
  ///
  /// Every side is clamped to `[0, borderWidth]`. The overlay paints over the
  /// card's content, so an inset may sit on the content's first hairline;
  /// negative sides resolve to the card edge.
  final EdgeInsetsGeometry outlineInset;

  /// Whether the hairline also runs down the left and right edges. Off, the
  /// card keeps only its top and bottom rules so its content can sit flush
  /// against the surface around it.
  final bool sideBorders;

  /// Whether the card carries its top hairline. Ignored while [sideBorders]
  /// is on, which paints all four.
  final bool topBorder;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final radius = theme.style.borderRadius.lg;
    final borderW = theme.style.borderWidth;
    final content = clipBehavior == Clip.none
        ? child
        : ClipRRect(
            borderRadius: radius,
            clipBehavior: clipBehavior,
            child: child,
          );
    final color = borderColor ?? theme.colors.border;
    // Interaction chrome (an overridden color) must read as a closed outline;
    // the resting card may sit flush with only its horizontal rules. The
    // overridden outline moves to the inset foreground overlay below; painting
    // it here as well would double it.
    final Border? border;
    if (borderColor != null) {
      border = null;
    } else if (sideBorders) {
      border = Border.all(color: color, width: borderW);
    } else {
      border = Border(
        top: topBorder
            ? BorderSide(color: color, width: borderW)
            : BorderSide.none,
        bottom: BorderSide(color: color, width: borderW),
      );
    }
    final card = DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.card,
        border: border,
        borderRadius: radius,
      ),
      // Inset so edge-flush fills (cover art, waveforms) cannot paint over the
      // hairline — DecoratedBox draws the stroke behind its child. Only the
      // sides that carry a resting hairline are reserved, so content sits
      // flush where there is none (a card with `sideBorders`/`topBorder` off).
      // The hover outline is a foreground overlay: it must not reserve any
      // room, or a drag entering the card would shift and squeeze its content.
      child: Padding(
        padding: sideBorders
            ? EdgeInsets.all(borderW)
            : EdgeInsets.only(top: topBorder ? borderW : 0, bottom: borderW),
        child: content,
      ),
    );
    // The Stack is always the root so toggling the interaction outline does
    // not remount the card body (and lose e.g. a scroll position). Passthrough
    // keeps the parent's constraints on the card: a shrink-wrapping card must
    // keep its own size, not just let the overlay span a larger box.
    final inset = borderColor == null
        ? EdgeInsets.zero
        : _clampOutlineInset(
            outlineInset.resolve(Directionality.of(context)),
            borderW,
          );
    return Stack(
      fit: StackFit.passthrough,
      clipBehavior: Clip.none,
      children: [
        card,
        if (borderColor != null)
          Positioned.fill(
            child: IgnorePointer(
              child: Padding(
                padding: inset,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    border: Border.all(color: color, width: borderW),
                    borderRadius: _deflateBorderRadius(radius, inset),
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}

/// The inset stays within the card hairline ([max]), so the interaction
/// outline's stroke can sit over the content's first hairline but never past
/// the card edge.
EdgeInsets _clampOutlineInset(EdgeInsets inset, double max) =>
    EdgeInsets.fromLTRB(
      _clampInset(inset.left, max),
      _clampInset(inset.top, max),
      _clampInset(inset.right, max),
      _clampInset(inset.bottom, max),
    );

/// Keeps one side of the outline inset inside the card's hairline ring:
/// negative values collapse to the edge and values past [max] stop at it.
double _clampInset(double value, double max) => value < 0
    ? 0
    : value > max
    ? max
    : value;

/// Keeps the outline's corners concentric with the card's while it is inset
/// by a different amount on each side.
BorderRadius _deflateBorderRadius(BorderRadius radius, EdgeInsets inset) =>
    BorderRadius.only(
      topLeft: _deflateRadius(radius.topLeft, inset.left, inset.top),
      topRight: _deflateRadius(radius.topRight, inset.right, inset.top),
      bottomLeft: _deflateRadius(radius.bottomLeft, inset.left, inset.bottom),
      bottomRight: _deflateRadius(
        radius.bottomRight,
        inset.right,
        inset.bottom,
      ),
    );

/// One corner of the outline: each radius axis gives up its adjacent inset,
/// so the corner stays concentric with the card's on every side.
Radius _deflateRadius(Radius radius, double dx, double dy) => Radius.elliptical(
  _deflateLength(radius.x, dx),
  _deflateLength(radius.y, dy),
);

/// A radius axis can shrink to zero but never past it, even when the inset
/// meets or exceeds the radius.
double _deflateLength(double value, double inset) =>
    value > inset ? value - inset : 0;
