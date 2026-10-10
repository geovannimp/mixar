import 'package:anchor_ui/anchor_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

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
          _MixarOverlayDismissBackdrop(onDismiss: _controller.hide),
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
///
/// Desktop / auto-on-desktop opens an anchored popover. Mobile opens the same
/// menu body in [showMixarDialog] (driven by Settings → UI → Select style).
class MixarMenuAnchor extends StatefulWidget {
  const new({
    required this.childBuilder,
    required this.menuBuilder,
    this.controller,
    this.placement = Placement.bottomEnd,
    this.enabled = true,
    this.minWidth = 200,
    this.style,
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

  /// Force presentation; `null` reads the saved select-style setting.
  final SelectStyleSetting? style;

  @override
  State<MixarMenuAnchor> createState() => _MixarMenuAnchorState();
}

class _MixarMenuAnchorState extends State<MixarMenuAnchor> {
  MixarOverlayController? _ownedAnchor;
  _MenuDialogController? _ownedDialog;
  late MixarOverlayController _anchorController;

  @override
  void initState() {
    super.initState();
    _bindAnchorController();
  }

  @override
  void didUpdateWidget(MixarMenuAnchor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      _bindAnchorController();
    }
  }

  void _bindAnchorController() {
    if (widget.controller != null) {
      _ownedAnchor?.dispose();
      _ownedAnchor = null;
      _anchorController = widget.controller!;
    } else {
      _ownedAnchor ??= MixarOverlayController();
      _anchorController = _ownedAnchor!;
    }
  }

  _MenuDialogController _dialogController() {
    return _ownedDialog ??= _MenuDialogController(
      onOpen: _openMenuDialog,
    );
  }

  Future<void> _openMenuDialog() async {
    final hostContext = context;
    if (!hostContext.mounted || !widget.enabled) return;
    final controller = _dialogController();
    await showMixarDialog<void>(
      context: hostContext,
      builder: (dialogContext) {
        controller.attachNavigator(Navigator.of(dialogContext));
        final theme = dialogContext.theme;
        return Padding(
          padding: const EdgeInsets.all(16),
          child: DefaultTextStyle(
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.foreground,
            ),
            child: widget.menuBuilder(dialogContext, controller),
          ),
        );
      },
    );
  }

  @override
  void dispose() {
    _ownedAnchor?.dispose();
    _ownedDialog?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final override = widget.style;
    if (override != null) {
      return _buildForStyle(effectiveSelectStyle(override));
    }
    return Consumer(
      builder: (context, ref, _) {
        final setting = ref.watch(selectStyleSettingProvider);
        return _buildForStyle(effectiveSelectStyle(setting));
      },
    );
  }

  Widget _buildForStyle(SelectStyleSetting style) {
    if (style == SelectStyleSetting.mobile) {
      final controller = _dialogController();
      return widget.childBuilder(context, controller);
    }
    return Anchor(
      controller: _anchorController.anchor,
      triggerMode: const AnchorTriggerMode.manual(),
      placement: widget.placement,
      enabled: widget.enabled,
      backdropBuilder: (context) =>
          _MixarOverlayDismissBackdrop(onDismiss: _anchorController.hide),
      overlayBuilder: (context) {
        return MixarMenuPanel(
          minWidth: widget.minWidth,
          child: widget.menuBuilder(context, _anchorController),
        );
      },
      child: widget.childBuilder(context, _anchorController),
    );
  }
}

/// [MixarOverlayController] that opens a dialog instead of an Anchor overlay.
class _MenuDialogController extends MixarOverlayController {
  _MenuDialogController({required this.onOpen});

  final Future<void> Function() onOpen;
  NavigatorState? _dialogNavigator;
  var _open = false;

  @override
  bool get isShowing => _open;

  void attachNavigator(NavigatorState navigator) {
    _dialogNavigator = navigator;
  }

  @override
  void show() {
    if (_open) return;
    _open = true;
    notifyListeners();
    onOpen().whenComplete(_handleClosed);
  }

  @override
  void hide() {
    if (!_open) return;
    final nav = _dialogNavigator;
    if (nav != null && nav.canPop()) {
      nav.pop();
      return;
    }
    _handleClosed();
  }

  @override
  void toggle() => _open ? hide() : show();

  void _handleClosed() {
    _dialogNavigator = null;
    if (!_open) return;
    _open = false;
    notifyListeners();
  }
}

/// Primary-button outside tap dismisses (manual [Anchor] has no TapRegion dismiss).
class _MixarOverlayDismissBackdrop extends StatelessWidget {
  const new({required this.onDismiss});

  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    return Listener(
      onPointerDown: (event) {
        if (event.buttons == 1) {
          onDismiss();
        }
      },
      behavior: HitTestBehavior.opaque,
      child: const SizedBox.expand(),
    );
  }
}
