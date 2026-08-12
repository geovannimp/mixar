import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Reserved height below the pad cluster (sampler bank bar / empty spacer).
///
/// Keeps pad vertical position stable when switching to Sample mode.
const kPadModeBottomChromeHeight = 32.0;

/// 4-column, 8-pad grid shell (Tauri `PadGridContainer`).
///
/// Pads are equal squares with uniform gaps, centered above a fixed bottom bar
/// that is empty on most modes and holds sampler bank chrome on Sample.
class PadGrid extends StatelessWidget {
  PadGrid({required this.children, this.bottomChrome, super.key}) {
    if (children.length != 8) {
      throw ArgumentError.value(
        children.length,
        'children.length',
        'PadGrid expects exactly 8 children',
      );
    }
  }

  final List<Widget> children;

  /// Optional content inside the shared bottom bar (e.g. sampler bank controls).
  final Widget? bottomChrome;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          child: Padding(
            padding: const EdgeInsets.all(8),
            child: LayoutBuilder(
              builder: (context, constraints) {
                const gap = 8.0;
                const cols = 4;
                const rows = 2;
                final maxSideW =
                    (constraints.maxWidth - gap * (cols - 1)) / cols;
                final maxSideH =
                    (constraints.maxHeight - gap * (rows - 1)) / rows;
                final side = math.min(maxSideW, maxSideH);
                if (side <= 0 || !side.isFinite) {
                  return const SizedBox.shrink();
                }

                return Center(
                  child: SizedBox(
                    width: side * cols + gap * (cols - 1),
                    height: side * rows + gap * (rows - 1),
                    child: GridView.count(
                      crossAxisCount: cols,
                      mainAxisSpacing: gap,
                      crossAxisSpacing: gap,
                      childAspectRatio: 1,
                      physics: const NeverScrollableScrollPhysics(),
                      padding: EdgeInsets.zero,
                      children: children,
                    ),
                  ),
                );
              },
            ),
          ),
        ),
        DecoratedBox(
          decoration: BoxDecoration(
            border: Border(top: BorderSide(color: theme.colors.border)),
          ),
          child: SizedBox(
            height: kPadModeBottomChromeHeight,
            width: double.infinity,
            child: bottomChrome,
          ),
        ),
      ],
    );
  }
}
