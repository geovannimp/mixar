import 'package:flutter/widgets.dart';

/// 4-column, 8-pad grid shell (Tauri `PadGridContainer`).
class PadGrid extends StatelessWidget {
  PadGrid({required this.children, super.key}) {
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
      padding: const EdgeInsets.all(8),
      child: LayoutBuilder(
        builder: (context, constraints) {
          const gap = 6.0;
          final cellW = (constraints.maxWidth - gap * 3) / 4;
          final cellH = (constraints.maxHeight - gap) / 2;
          return Column(
            children: [
              for (var row = 0; row < 2; row++) ...[
                if (row > 0) const SizedBox(height: gap),
                Expanded(
                  child: Row(
                    children: [
                      for (var col = 0; col < 4; col++) ...[
                        if (col > 0) const SizedBox(width: gap),
                        Expanded(
                          child: SizedBox(
                            width: cellW,
                            height: cellH,
                            child: children[row * 4 + col],
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
              ],
            ],
          );
        },
      ),
    );
  }
}
