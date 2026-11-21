import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_label.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_segments.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class MixarMenuGroup extends StatelessWidget {
  const new({required this.children, super.key});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (var i = 0; i < children.length; i++) ...[
          if (i > 0 && _isDividerPair(children[i - 1], children[i]))
            MDivider(padding: EdgeInsets.zero, color: theme.colors.border),
          children[i],
        ],
      ],
    );
  }
}

/// In the mobile dialog, adjacent rows are separated by a hairline. Section
/// labels and segmented controls already delimit themselves, so no rule is
/// added next to them.
bool _isDividerPair(Widget previous, Widget next) =>
    previous is! MixarMenuLabel &&
    next is! MixarMenuLabel &&
    previous is! MixarMenuSegments &&
    next is! MixarMenuSegments &&
    // A nested group manages its own rows and rules.
    previous is! MixarMenuGroup &&
    next is! MixarMenuGroup;
