import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Reserved height below the pad cluster (sampler bank bar / empty spacer).
///
/// Keeps pad vertical position stable when switching to Sample mode.
const kPadModeBottomChromeHeight = 32.0;

const _kPadGap = 8.0;

/// 4-column, 8-pad grid shell (Tauri `PadGridContainer`).
///
/// Pads stay square; the 2×4 cluster is as large as the pane allows and
/// centered. The bottom bar is empty on most modes and holds sampler bank
/// chrome on Sample.
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
