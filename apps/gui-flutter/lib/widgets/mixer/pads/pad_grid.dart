import 'dart:math' as math;

import 'package:flutter/widgets.dart';

const _kPadGap = 8.0;

/// 4-column, 8-pad grid shell (Tauri `PadGridContainer`).
///
/// Pads stay square; the 2×4 cluster is as large as the pane allows and
/// centered.
class PadGrid extends StatelessWidget {
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
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(_kPadGap),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final side = math.max(
            0.0,
            math.min(
              (constraints.maxWidth - 3 * _kPadGap) / 4,
              (constraints.maxHeight - _kPadGap) / 2,
            ),
          );
          return Center(
            child: SizedBox(
              width: 4 * side + 3 * _kPadGap,
              height: 2 * side + _kPadGap,
              child: Column(
                children: [
                  _padRow(children.sublist(0, 4), side),
                  const SizedBox(height: _kPadGap),
                  _padRow(children.sublist(4), side),
                ],
              ),
            ),
          );
        },
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
