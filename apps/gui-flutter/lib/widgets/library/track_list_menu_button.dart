import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_anchor.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// The 3-dot row menu button. The menu body is built lazily so its
/// `ref.watch`es only run while it is open. Public so a pane or test can mount
/// the menu on its own, not just inside a [TrackListView].
class TrackListMenuButton extends StatelessWidget {
  const new({
    required this.menuBuilder,
    this.enableSecondaryPress = true,
    super.key,
  });

  final Widget Function(
    BuildContext context,
    WidgetRef ref,
    VoidCallback dismiss,
  )
  menuBuilder;
  final bool enableSecondaryPress;

  @override
  Widget build(BuildContext context) {
    return MixarMenuAnchor(
      dialogTitle: 'Track actions',
      menuBuilder: (context, controller) => Consumer(
        builder: (context, ref, _) =>
            menuBuilder(context, ref, controller.hide),
      ),
      childBuilder: (context, controller) => AppButton.icon(
        variant: .ghost,
        size: .xs,
        semanticsLabel: 'Track actions',
        onPress: controller.toggle,
        onSecondaryPress: enableSecondaryPress ? controller.toggle : null,
        child: const Icon(LucideIcons.ellipsisVertical),
      ),
    );
  }
}
