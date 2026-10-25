import 'dart:math' as math;

import 'package:flutter/widgets.dart';

const _kPadGap = 8.0;
const _kPadMinSide = 24.0;

/// 4-column, 8-pad grid shell (Tauri `PadGridContainer`).
///
/// Pads stay square and centered; short panes scroll without crushing controls.
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

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // Pads fire on pointer-down, so only wheel/thumb scrolling is safe.
    return ScrollConfiguration(
      behavior: ScrollConfiguration.of(context).copyWith(dragDevices: const {}),
      child: RawScrollbar(
        controller: _controller,
        thumbVisibility: true,
        interactive: true,
        child: Padding(
          padding: const EdgeInsets.all(_kPadGap),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final double side = math.max(
                0,
                math.min(
                  (constraints.maxWidth - 3 * _kPadGap) / 4,
                  math.max(
                    _kPadMinSide,
                    (constraints.maxHeight - _kPadGap) / 2,
                  ),
                ),
              );
              return SingleChildScrollView(
                controller: _controller,
                child: ConstrainedBox(
                  constraints: BoxConstraints(minHeight: constraints.maxHeight),
                  child: Center(
                    child: SizedBox(
                      width: 4 * side + 3 * _kPadGap,
                      height: 2 * side + _kPadGap,
                      child: Column(
                        children: [
                          _padRow(widget.children.sublist(0, 4), side),
                          const SizedBox(height: _kPadGap),
                          _padRow(widget.children.sublist(4), side),
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

Widget _padRow(List<Widget> pads, double side) {
  return SizedBox(
    height: side,
    child: Row(
      children: [
        for (var i = 0; i < pads.length; i++) ...[
          if (i > 0) const SizedBox(width: _kPadGap),
          SizedBox(width: side, height: side, child: pads[i]),
        ],
      ],
    ),
  );
}
