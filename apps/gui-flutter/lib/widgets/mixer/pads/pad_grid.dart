import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/scroll_physics.dart';
import 'package:gui_flutter/core/utils/silky_scroll_glide.dart';
import 'package:silky_scroll/silky_scroll.dart';

const _kPadGap = 8.0;
const _kPadMinSide = 24.0;

/// Widest a pad may get relative to its height.
const _kPadMaxAspect = 16 / 10;

/// 4-column, 8-pad grid shell (Tauri `PadGridContainer`).
///
/// The grid fills the pane: rows take its height, columns its width. A pad's
/// width caps at 16:10, so a wide pane centres a narrower block instead of
/// stretching pads; short panes scroll without crushing controls.
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
          child: Padding(
            padding: const EdgeInsets.all(_kPadGap),
            child: LayoutBuilder(
              builder: (context, constraints) {
                const gap = _kPadGap;
                final double padHeight = math.max(
                  _kPadMinSide,
                  (constraints.maxHeight - gap) / 2,
                );
                final double widthFit = (constraints.maxWidth - 3 * gap) / 4;
                final double padWidth = math.max(
                  0,
                  math.min(widthFit, _kPadMaxAspect * padHeight),
                );
                // No width for even one pad: painting the rows would overflow
                // the pane's three gaps, so the grid stays empty instead.
                final bool fitsColumns = widthFit > 0;
                return SilkySingleChildScrollView(
                  key: _silkyKey,
                  controller: _controller,
                  physics: mixarScrollPhysics(context),
                  decayLogFactor: kMixarSilkyDecayLogFactor,
                  blockWebOverscrollBehaviorX:
                      kMixarSilkyBlockWebOverscrollBehaviorX,
                  child: ConstrainedBox(
                    constraints: BoxConstraints(
                      minHeight: constraints.maxHeight,
                    ),
                    child: Center(
                      child: fitsColumns
                          ? SizedBox(
                              width: 4 * padWidth + 3 * gap,
                              height: 2 * padHeight + gap,
                              child: Column(
                                children: [
                                  _padRow(
                                    widget.children.sublist(0, 4),
                                    padWidth,
                                    padHeight,
                                  ),
                                  const SizedBox(height: gap),
                                  _padRow(
                                    widget.children.sublist(4),
                                    padWidth,
                                    padHeight,
                                  ),
                                ],
                              ),
                            )
                          : const SizedBox.shrink(),
                    ),
                  ),
                );
              },
            ),
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
        for (var i = 0; i < pads.length; i++) ...[
          if (i > 0) const SizedBox(width: _kPadGap),
          SizedBox(width: padWidth, height: padHeight, child: pads[i]),
        ],
      ],
    ),
  );
}
