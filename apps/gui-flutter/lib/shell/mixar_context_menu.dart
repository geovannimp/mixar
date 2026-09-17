import 'package:anchor_ui/anchor_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';

/// Handle for showing/hiding a [MixarContextMenu] from its builders.
class MixarContextMenuHandle {
  MixarContextMenuHandle._(this._controller);

  final AnchorContextMenuController _controller;

  bool get isShowing => _controller.isShowing;

  void showAt(Offset globalPosition) => _controller.show(globalPosition);

  void hide() => _controller.hide();
}

/// Right-click / long-press menu at a cursor position.
class MixarContextMenu extends StatelessWidget {
  const MixarContextMenu({
    required this.childBuilder,
    required this.menuBuilder,
    this.enabled = true,
    this.minWidth = 200,
    super.key,
  });

  final Widget Function(BuildContext context, MixarContextMenuHandle handle)
  childBuilder;
  final Widget Function(BuildContext context, MixarContextMenuHandle handle)
  menuBuilder;
  final bool enabled;
  final double minWidth;

  @override
  Widget build(BuildContext context) {
    return AnchorContextMenu(
      enabled: enabled,
      placement: Placement.bottomStart,
      menuBuilder: (context) {
        final handle = MixarContextMenuHandle._(
          AnchorContextMenuController.of(context),
        );
        return MixarMenuPanel(
          minWidth: minWidth,
          child: menuBuilder(context, handle),
        );
      },
      childBuilder: (context) {
        final handle = MixarContextMenuHandle._(
          AnchorContextMenuController.of(context),
        );
        return childBuilder(context, handle);
      },
    );
  }
}
