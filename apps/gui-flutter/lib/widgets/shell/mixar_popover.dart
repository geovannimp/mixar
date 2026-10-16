import 'package:anchor_ui/anchor_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_panel.dart';
import 'package:gui_flutter/providers/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_dialog_controller.dart';

/// Anchored content popover (e.g. deck gain details).
///
/// Uses [Anchor] with manual trigger so the child owns press handling.
class MixarPopover extends StatefulWidget {
  const new({
    required this.childBuilder,
    required this.overlayBuilder,
    this.controller,
    this.placement = Placement.bottomStart,
    this.enabled = true,
    this.minWidth = 180,
    super.key,
  });

  final MixarOverlayController? controller;
  final Widget Function(BuildContext context, MixarOverlayController controller)
  childBuilder;
  final WidgetBuilder overlayBuilder;
  final Placement placement;
  final bool enabled;
  final double minWidth;

  @override
  State<MixarPopover> createState() => _MixarPopoverState();
}

class _MixarPopoverState extends State<MixarPopover> {
  MixarOverlayController? _owned;
  late MixarOverlayController _controller;

  @override
  void initState() {
    super.initState();
    _bindController();
  }

  @override
  void didUpdateWidget(MixarPopover oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      _bindController();
    }
  }

  void _bindController() {
    if (widget.controller != null) {
      _owned?.dispose();
      _owned = null;
      _controller = widget.controller!;
    } else {
      _owned ??= MixarOverlayController();
      _controller = _owned!;
    }
  }

  @override
  void dispose() {
    _owned?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Anchor(
      controller: _controller.anchor,
      triggerMode: const AnchorTriggerMode.manual(),
      placement: widget.placement,
      enabled: widget.enabled,
      // Manual mode skips Anchor's TapRegion outside-dismiss; match
      // AnchorContextMenu with a translucent full-screen backdrop.
      backdropBuilder: (context) =>
          MixarOverlayDismissBackdrop(onDismiss: _controller.hide),
      overlayBuilder: (context) {
        return MixarMenuPanel(
          minWidth: widget.minWidth,
          child: DefaultTextStyle(
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.foreground,
            ),
            child: widget.overlayBuilder(context),
          ),
        );
      },
      child: widget.childBuilder(context, _controller),
    );
  }
}
