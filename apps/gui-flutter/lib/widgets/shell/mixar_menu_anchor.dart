import 'package:anchor_ui/anchor_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_dialog_chrome.dart';
import 'package:gui_flutter/providers/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_dialog_controller.dart';

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
        final title = widget.dialogTitle;
        return SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: DefaultTextStyle(
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.foreground,
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                if (title != null) ...[
                  Text(
                    title,
                    style: theme.typography.body.md.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 16),
                ],
                MixarMenuDialogChrome(
                  child: widget.menuBuilder(dialogContext, controller),
                ),
              ],
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
        return MixarMenuPanel(
          minWidth: widget.minWidth,
          child: widget.menuBuilder(context, _anchorController),
        );
      },
      child: widget.childBuilder(context, _anchorController),
    );
  }
}
