import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/scroll_physics.dart';
import 'package:gui_flutter/core/utils/silky_scroll_glide.dart';
import 'package:silky_scroll/silky_scroll.dart';

const _kPadMinSide = 24.0;

/// Widest a pad may get relative to its height.
const _kPadMaxAspect = 16 / 10;

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
                    child: SizedBox(
                      width: 4 * padWidth,
                      height: 2 * padHeight,
                      child: Column(
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
