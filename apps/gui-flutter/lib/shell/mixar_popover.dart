import 'package:anchor_ui/anchor_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_overlay_controller.dart';

/// Anchored content popover (e.g. deck gain details).
///
/// Uses [Anchor] with manual trigger so the child owns press handling.
class MixarPopover extends StatefulWidget {
  const MixarPopover({
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

/// Button-anchored action menu (⋯ menus).
class MixarMenuAnchor extends StatefulWidget {
  const MixarMenuAnchor({
    required this.childBuilder,
    required this.menuBuilder,
    this.controller,
    this.placement = Placement.bottomEnd,
    this.enabled = true,
    this.minWidth = 200,
    super.key,
  });

  final MixarOverlayController? controller;
  final Widget Function(BuildContext context, MixarOverlayController controller)
  childBuilder;
  final Widget Function(BuildContext context, MixarOverlayController controller)
  menuBuilder;
  final Placement placement;
  final bool enabled;
  final double minWidth;

  @override
  State<MixarMenuAnchor> createState() => _MixarMenuAnchorState();
}

class _MixarMenuAnchorState extends State<MixarMenuAnchor> {
  MixarOverlayController? _owned;
  late MixarOverlayController _controller;

  @override
  void initState() {
    super.initState();
    _bindController();
  }

  @override
  void didUpdateWidget(MixarMenuAnchor oldWidget) {
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
    return Anchor(
      controller: _controller.anchor,
      triggerMode: const AnchorTriggerMode.manual(),
      placement: widget.placement,
      enabled: widget.enabled,
      overlayBuilder: (context) {
        return MixarMenuPanel(
          minWidth: widget.minWidth,
          child: widget.menuBuilder(context, _controller),
        );
      },
      child: widget.childBuilder(context, _controller),
    );
  }
}
