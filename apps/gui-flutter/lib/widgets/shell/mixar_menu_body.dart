import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_dialog_chrome.dart';

/// Column of [MixarMenuGroup]s with hairline separators between groups.
class MixarMenuBody extends StatelessWidget {
  const new({required this.groups, super.key});

  final List<Widget> groups;

  @override
  Widget build(BuildContext context) {
    if (groups.isEmpty) {
      return const SizedBox.shrink();
    }
    final dialogChrome = MixarMenuDialogChrome.isActive(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (var i = 0; i < groups.length; i++) ...[
          if (i > 0)
            dialogChrome
                // Card rows already own a 6px bottom gap; keep the same rhythm
                // between groups instead of a hairline + extra padding.
                ? const SizedBox.shrink()
                : const MDivider(padding: EdgeInsets.symmetric(vertical: 4)),
          groups[i],
        ],
      ],
    );
  }
}
