import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';

/// Slides a wide waveform strip behind a fixed, centred playhead.
///
/// The strip used to be repositioned with a `Positioned` rebuilt on every
/// display tick, which dirtied its parent's layout each time. The same geometry
/// is applied here by a [Flow] delegate during paint: a tick only marks the
/// viewport for repaint, so neither the strip's widget tree nor its layout is
/// touched while it moves.
///
/// [Flow] keeps the transform it painted each child with, so hit tests are
/// inverted through it and pointer positions still land on the translated
/// strip.
class WaveformScrollViewport extends StatelessWidget {
  const new({
    required this.playhead,
    required this.displayMs,
    required this.originMs,
    required this.pxPerMs,
    required this.stripSize,
    required this.dpr,
    required this.child,
    super.key,
  });

  /// Ticks that move the strip. Painting listens to it; nothing rebuilds.
  final Listenable playhead;

  /// Signed source position that belongs under the playhead, read at paint.
  final double Function() displayMs;

  /// Source time of the strip's first painted pixel.
  final int originMs;

  /// Horizontal density of the strip.
  final double pxPerMs;

  /// Size the strip lays out at; it may be wider than the viewport.
  final Size stripSize;

  /// Device pixel ratio the translate is snapped to.
  final double dpr;

  /// The strip, painted under the playhead.
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Flow(
      // The lane's ClipRect owns clipping. The strip paints a beat grid that
      // starts before its audio window and reaches past its far edge; clipping
      // here would cut it at the strip instead of the lane.
      clipBehavior: Clip.none,
      delegate: _WaveformScrollDelegate(
        playhead: playhead,
        displayMs: displayMs,
        originMs: originMs,
        pxPerMs: pxPerMs,
        stripSize: stripSize,
        dpr: dpr,
      ),
      children: [child],
    );
  }
}

class _WaveformScrollDelegate extends FlowDelegate {
  new({
    required this.playhead,
    required this.displayMs,
    required this.originMs,
    required this.pxPerMs,
    required this.stripSize,
    required this.dpr,
  }) : super(repaint: playhead);

  final Listenable playhead;
  final double Function() displayMs;
  final int originMs;
  final double pxPerMs;
  final Size stripSize;
  final double dpr;

  @override
  BoxConstraints getConstraintsForChild(int i, BoxConstraints constraints) =>
      BoxConstraints.tight(stripSize);

  @override
  void paintChildren(FlowPaintingContext context) {
    final x = snapPx(
      stripTranslateX(
        positionMs: displayMs(),
        originMs: originMs,
        viewportWidth: context.size.width,
        pxPerMs: pxPerMs,
      ),
      dpr,
    );
    context.paintChild(0, transform: Matrix4.translationValues(x, 0, 0));
  }

  @override
  bool shouldRelayout(covariant _WaveformScrollDelegate oldDelegate) =>
      stripSize != oldDelegate.stripSize;

  @override
  bool shouldRepaint(covariant _WaveformScrollDelegate oldDelegate) =>
      playhead != oldDelegate.playhead ||
      displayMs != oldDelegate.displayMs ||
      originMs != oldDelegate.originMs ||
      pxPerMs != oldDelegate.pxPerMs ||
      stripSize != oldDelegate.stripSize ||
      dpr != oldDelegate.dpr;
}
