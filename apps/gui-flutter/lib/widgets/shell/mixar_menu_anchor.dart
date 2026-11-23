import 'dart:math' as math;

import 'package:anchor_ui/anchor_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_dialog_chrome.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_dialog_controller.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Fallback share of the window height for an anchored menu.
///
/// Only used before the trigger has been laid out; once it has, the panel is
/// bounded by the room beside it instead.
const double kMixarMenuMaxHeightFactor = 0.8;

/// Gap kept between an anchored menu and the window edge.
const double kMixarMenuEdgeMargin = 8;

/// Button-anchored action menu (⋯ menus).
///
/// Desktop / auto-on-desktop opens an anchored popover. Mobile opens the same
/// menu body in [showMixarDialog] (driven by Settings → UI → Select style).
///
/// The anchored panel never grows past [kMixarMenuMaxHeightFactor] of the
/// window; longer menus scroll inside the panel.
class MixarMenuAnchor extends StatefulWidget {
  const new({
    required this.childBuilder,
    required this.menuBuilder,
    this.controller,
    this.placement = Placement.bottomEnd,
    this.enabled = true,
    this.minWidth = 200,
    this.style,
    this.dialogTitle,
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

  /// Title shown above the menu body in the mobile dialog.
  final String? dialogTitle;

  @override
  State<MixarMenuAnchor> createState() => _MixarMenuAnchorState();
}

class _MixarMenuAnchorState extends State<MixarMenuAnchor> {
  MixarOverlayController? _ownedAnchor;
  MenuDialogController? _ownedDialog;
  late MixarOverlayController _anchorController;

  /// Measures the trigger when a panel opens: the room beside it, not the
  /// whole window, is what the panel may use.
  final _triggerKey = GlobalKey();

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

  MenuDialogController _dialogController() {
    return _ownedDialog ??= MenuDialogController(onOpen: _openMenuDialog);
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
        return MixarDialog(
          title: widget.dialogTitle,
          // Menu rows are full-bleed bands; each owns its horizontal inset.
          contentPadding: EdgeInsets.zero,
          child: DefaultTextStyle(
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.foreground,
            ),
            child: MixarMenuDialogChrome(
              child: widget.menuBuilder(dialogContext, controller),
            ),
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
          MixarOverlayDismissBackdrop(onDismiss: _anchorController.hide),
      overlayBuilder: (context) {
        // A menu taller than its side of the window scrolls. The cap is the
        // larger of the room above and below the trigger: Anchor's flip
        // middleware only moves the panel up when it cannot fit below, so a
        // cap of that size always fits the side the panel ends up on.
        return MixarMenuPanel(
          minWidth: widget.minWidth,
          child: ConstrainedBox(
            constraints: BoxConstraints(maxHeight: _menuMaxHeight(context)),
            child: SingleChildScrollView(
              child: widget.menuBuilder(context, _anchorController),
            ),
          ),
        );
      },
      child: KeyedSubtree(
        key: _triggerKey,
        child: widget.childBuilder(context, _anchorController),
      ),
    );
  }

  /// Room the panel may fill: the larger of the space above and below the
  /// trigger, minus the edge margin.
  double _menuMaxHeight(BuildContext context) {
    final media = MediaQuery.maybeOf(context);
    if (media == null) {
      return double.infinity;
    }
    final box = _triggerKey.currentContext?.findRenderObject() as RenderBox?;
    if (box == null || !box.hasSize) {
      return media.size.height * kMixarMenuMaxHeightFactor;
    }
    final top = box.localToGlobal(Offset.zero).dy;
    final bottom = top + box.size.height;
    final below = media.size.height - bottom - kMixarMenuEdgeMargin;
    final above = top - kMixarMenuEdgeMargin;
    return math.max(math.max(below, above), 0);
  }
}
