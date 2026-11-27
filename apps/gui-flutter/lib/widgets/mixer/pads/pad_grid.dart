import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/scroll_physics.dart';
import 'package:gui_flutter/core/utils/silky_scroll_glide.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:silky_scroll/silky_scroll.dart';

const _kPadMinSide = 24.0;

/// Evenly spaced cell boundaries for [extent], snapped to device pixels.
///
/// Boundaries, not widths: neighbouring cells may differ by one device pixel,
/// so a 290 px block of four pads gets 73/72/73/72 instead of 72.5 each. That
/// keeps every fill edge on a pixel, which is what makes a 1 px seam stay 1 px.
@visibleForTesting
List<double> padTableSeams(double extent, int cells, double devicePixelRatio) {
  final snapped = (extent * devicePixelRatio).floor() / devicePixelRatio;
  return [
    for (var i = 0; i <= cells; i++)
      (snapped * i / cells * devicePixelRatio).round() / devicePixelRatio,
  ];
}

/// Seams of the tiled pad table, in block coordinates.
///
/// The grid paints the table's interior above the pads: the three column seams
/// and the row seam. It draws no outer frame — the top is the panel's header
/// divider, the bottom the deck's transport divider, and the sides are the
/// hairline the deck chrome already draws against the pane (the tempo column's
/// border or the tab rail's).
@visibleForTesting
List<Rect> padTableLines(Size size, double width, double devicePixelRatio) {
  final columns = padTableSeams(size.width, 4, devicePixelRatio);
  final rows = padTableSeams(size.height, 2, devicePixelRatio);
  return <Rect>[
    for (var column = 1; column < 4; column++)
      Rect.fromLTWH(columns[column], 0, width, size.height),
    Rect.fromLTWH(0, rows[1], size.width, width),
  ];
}

/// Paints [padTableLines] above the pads.
class PadTablePainter extends CustomPainter {
  const new({
    required this.color,
    required this.width,
    required this.devicePixelRatio,
  });

  final Color color;
  final double width;
  final double devicePixelRatio;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..color = color;
    for (final line in padTableLines(size, width, devicePixelRatio)) {
      canvas.drawRect(line, paint);
    }
  }

  @override
  bool shouldRepaint(PadTablePainter oldDelegate) =>
      oldDelegate.color != color ||
      oldDelegate.width != width ||
      oldDelegate.devicePixelRatio != devicePixelRatio;
}

/// 4-column, 8-pad grid shell (Tauri `PadGridContainer`).
///
/// Pads tile the pane edge to edge, table-style: no gaps, no outer inset and
/// no aspect cap — each pad takes a quarter of the width and half the height,
/// whatever shape that gives. Short panes scroll without crushing controls.
class PadGrid extends StatefulWidget {
  new({required this.children, super.key}) {
    if (children.length != 8) {
      throw ArgumentError.value(
        children.length,
        'children.length',
        'PadGrid expects exactly 8 children',
      );
    }
  }

  final List<Widget> children;

  @override
  State<PadGrid> createState() => _PadGridState();
}

class _PadGridState extends State<PadGrid> {
  final _controller = ScrollController();

  /// Lets a thumb drag stop an in-flight wheel glide.
  final _silkyKey = GlobalKey();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    // Pads fire on pointer-down, so only wheel/thumb scrolling is safe.
    return SilkyGlideHandoff(
      scrollKey: _silkyKey,
      child: ScrollConfiguration(
        behavior: ScrollConfiguration.of(context)
            .copyWith(dragDevices: const {}),
        child: RawScrollbar(
          controller: _controller,
          thumbVisibility: true,
          interactive: true,
          child: LayoutBuilder(
            builder: (context, constraints) {
              final devicePixelRatio = MediaQuery.devicePixelRatioOf(context);
              double snapDown(double value) =>
                  (value * devicePixelRatio).floor() / devicePixelRatio;
              final double padHeight = math.max(
                _kPadMinSide,
                constraints.maxHeight / 2,
              );
              // Snap the block itself, not just the lines: a fractional block
              // would leave a pad fill bleeding past its frame by a fraction
              // of a pixel, which reads as a second border along the edge.
              final double height = snapDown(2 * padHeight);
              final double width = snapDown(constraints.maxWidth);
              if (width <= 0) {
                return const SizedBox.shrink();
              }
              final columns = padTableSeams(width, 4, devicePixelRatio);
              final rows = padTableSeams(height, 2, devicePixelRatio);
              return SilkySingleChildScrollView(
                key: _silkyKey,
                controller: _controller,
                physics: mixarScrollPhysics(context),
                decayLogFactor: kMixarSilkyDecayLogFactor,
                blockWebOverscrollBehaviorX:
                    kMixarSilkyBlockWebOverscrollBehaviorX,
                child: ConstrainedBox(
                  constraints: BoxConstraints(minHeight: constraints.maxHeight),
                  child: Align(
                    alignment: Alignment.topLeft,
                    child: SizedBox(
                      width: width,
                      height: height,
                      child: Stack(
                        fit: StackFit.expand,
                        children: [
                          Column(
                            children: [
                              _padRow(
                                widget.children.sublist(0, 4),
                                columns,
                                rows[0],
                                rows[1],
                              ),
                              _padRow(
                                widget.children.sublist(4),
                                columns,
                                rows[1],
                                rows[2],
                              ),
                            ],
                          ),
                          // The seams sit above the pads, so no pad fill can
                          // cover one and every line stays one hairline.
                          IgnorePointer(
                            child: CustomPaint(
                              painter: PadTablePainter(
                                color: theme.colors.border,
                                width: theme.style.borderWidth,
                                devicePixelRatio: devicePixelRatio,
                              ),
                            ),
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
      ),
    );
  }
}

Widget _padRow(
  List<Widget> pads,
  List<double> columns,
  double top,
  double bottom,
) {
  final height = bottom - top;
  return SizedBox(
    height: height,
    child: Row(
      children: [
        for (var i = 0; i < pads.length; i++)
          SizedBox(
            width: columns[i + 1] - columns[i],
            height: height,
            child: pads[i],
          ),
      ],
    ),
  );
}
