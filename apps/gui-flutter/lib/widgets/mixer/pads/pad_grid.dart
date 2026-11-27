import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/scroll_physics.dart';
import 'package:gui_flutter/core/utils/silky_scroll_glide.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:silky_scroll/silky_scroll.dart';

const _kPadMinSide = 24.0;

/// Widest a pad may get relative to its height.
const _kPadMaxAspect = 16 / 10;

/// Seams of the tiled pad table, in block coordinates.
///
/// The grid paints the table itself, above the pads: left and right frames, the
/// three column seams, and the row seam. Only the top edge is missing — the
/// panel's header divider draws it — and the bottom is left to the surface
/// below (the deck's transport divider).
///
/// Lines snap to device pixels: a 290 px pane gives 72.5 px pads, and an
/// unsnapped 1 px seam would blur across two pixels and read as a double line.
@visibleForTesting
List<Rect> padTableLines(Size size, double width, double devicePixelRatio) {
  double snap(double value) =>
      (value * devicePixelRatio).round() / devicePixelRatio;
  final lines = <Rect>[
    Rect.fromLTWH(0, 0, width, size.height),
    Rect.fromLTWH(size.width - width, 0, width, size.height),
  ];
  for (var column = 1; column < 4; column++) {
    lines.add(
      Rect.fromLTWH(snap(size.width * column / 4), 0, width, size.height),
    );
  }
  lines.add(Rect.fromLTWH(0, snap(size.height / 2), size.width, width));
  return lines;
}

/// Paints [padTableLines] above the pads.
class PadTablePainter extends CustomPainter {
  const PadTablePainter({
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
/// Pads tile the pane edge to edge, table-style: no gaps and no outer inset.
/// A pad's width caps at 16:10, so a pane wider than that ratio centres a
/// narrower block instead of stretching pads; short panes scroll without
/// crushing controls.
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
              final double padHeight = math.max(
                _kPadMinSide,
                constraints.maxHeight / 2,
              );
              final double widthFit = constraints.maxWidth / 4;
              final double padWidth = math.max(
                0,
                math.min(widthFit, _kPadMaxAspect * padHeight),
              );
              final size = Size(4 * padWidth, 2 * padHeight);
              return SilkySingleChildScrollView(
                key: _silkyKey,
                controller: _controller,
                physics: mixarScrollPhysics(context),
                decayLogFactor: kMixarSilkyDecayLogFactor,
                blockWebOverscrollBehaviorX:
                    kMixarSilkyBlockWebOverscrollBehaviorX,
                child: ConstrainedBox(
                  constraints: BoxConstraints(minHeight: constraints.maxHeight),
                  child: Center(
                    child: SizedBox.fromSize(
                      size: size,
                      child: Stack(
                        fit: StackFit.expand,
                        children: [
                          Column(
                            children: [
                              _padRow(
                                widget.children.sublist(0, 4),
                                padWidth,
                                padHeight,
                              ),
                              _padRow(
                                widget.children.sublist(4),
                                padWidth,
                                padHeight,
                              ),
                            ],
                          ),
                          // The seams sit above the pads, so no pad fill can
                          // cover the frame and every line is one hairline.
                          IgnorePointer(
                            child: CustomPaint(
                              painter: PadTablePainter(
                                color: theme.colors.border,
                                width: theme.style.borderWidth,
                                devicePixelRatio: MediaQuery.devicePixelRatioOf(
                                  context,
                                ),
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

Widget _padRow(List<Widget> pads, double padWidth, double padHeight) {
  return SizedBox(
    height: padHeight,
    child: Row(
      children: [
        for (final pad in pads)
          SizedBox(width: padWidth, height: padHeight, child: pad),
      ],
    ),
  );
}
