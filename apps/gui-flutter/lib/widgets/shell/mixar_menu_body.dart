import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Column of `MixarMenuGroup`s with hairline separators between groups.
class MixarMenuBody extends StatelessWidget {
  const new({required this.groups, super.key});

  final List<Widget> groups;

  @override
  Widget build(BuildContext context) {
    if (groups.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = context.theme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (var i = 0; i < groups.length; i++) ...[
          if (i > 0)
            // Rows are full-bleed bands; the group rule spans the panel like
            // the row dividers.
            MDivider(padding: EdgeInsets.zero, color: theme.colors.border),
          groups[i],
        ],
      ],
    );
  }
}
