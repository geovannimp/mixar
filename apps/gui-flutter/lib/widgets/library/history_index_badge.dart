import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/library/history_row_layout.dart';

/// Leading `#` position, right-aligned so the numbers line up down the list.
class IndexBadge extends StatelessWidget {
  const new({required this.index});

  final int index;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return SizedBox(
      width: kHistoryIndexWidth,
      child: Text(
        '${index + 1}',
        textAlign: TextAlign.right,
        maxLines: 1,
        style: theme.typography.body.xs.copyWith(
          color: theme.colors.mutedForeground,
          fontFeatures: const [FontFeature.tabularFigures()],
        ),
      ),
    );
  }
}
