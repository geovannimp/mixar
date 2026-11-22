import 'dart:async';

import 'package:anchor_ui/anchor_ui.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/shell_tab.dart';
import 'package:gui_flutter/core/utils/app_assets.dart';
import 'package:gui_flutter/providers/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_anchor.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/nav_list_item.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// The name a page carries in the header and in the pages menu.
String shellPageLabel(ShellTab tab) => switch (tab) {
  ShellTab.mixer => 'Booth',
  ShellTab.settings => 'Settings',
};

IconData _shellPageIcon(ShellTab tab) => switch (tab) {
  ShellTab.mixer => LucideIcons.play,
  ShellTab.settings => LucideIcons.settings,
};

/// The shell's title control: the wordmark and the open page's name, opening a
/// dropdown with one row per page.
///
/// It sits at the window's centre, so the pages live in one place instead of a
/// corner hamburger, and it reads as the app's own title rather than as the
/// current page's chrome.
class AppPagesMenu extends StatelessWidget {
  const new({
    required this.appTitle,
    required this.current,
    required this.onSelect,
    super.key,
  });

  final String appTitle;
  final ShellTab current;
  final Future<void> Function(ShellTab) onSelect;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final label = shellPageLabel(current);
    return MixarMenuAnchor(
      // A shell menu, not a page-local one: it drops under the wordmark on
      // every host, so a phone sees the list in the same place the desktop
      // does instead of in a separate dialog.
      style: SelectStyleSetting.desktop,
      placement: Placement.bottom,
      // Wide enough for the longest page name beside its icon and accent rule.
      minWidth: 200,
      menuBuilder: (context, controller) => _PagesMenuBody(
        current: current,
        controller: controller,
        onSelect: onSelect,
      ),
      childBuilder: (context, controller) => AppButton(
        variant: .ghost,
        size: .sm,
        mainAxisSize: MainAxisSize.min,
        semanticsLabel: 'Pages, $label',
        onPress: controller.toggle,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            ColorFiltered(
              colorFilter: ColorFilter.mode(
                theme.colors.foreground,
                BlendMode.srcIn,
              ),
              child: Image.asset(
                kMixarLogoAsset,
                height: 12,
                semanticLabel: appTitle,
              ),
            ),
            const SizedBox(width: 8),
            Text(label),
            // The control opens a menu: the chevron says so before a press.
            Icon(
              LucideIcons.chevronDown,
              size: 14,
              color: theme.colors.mutedForeground,
            ),
          ],
        ),
      ),
    );
  }
}

/// Rows for every page, the open one marked the way the settings sidebar marks
/// its section.
class _PagesMenuBody extends StatelessWidget {
  const new({
    required this.current,
    required this.controller,
    required this.onSelect,
  });

  final ShellTab current;
  final MixarOverlayController controller;
  final Future<void> Function(ShellTab) onSelect;

  /// Reports the pick, then closes.
  ///
  /// The order matters: the page change can raise its own modal (the settings
  /// unsaved-edits guard), and a menu left floating above it would sit on the
  /// dialog's barrier.
  void _pick(ShellTab tab) {
    controller.hide();
    unawaited(onSelect(tab));
  }

  @override
  Widget build(BuildContext context) {
    // Escape is the desktop way out of a menu; the outside press is handled by
    // the overlay's backdrop.
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): controller.hide,
      },
      child: Focus(
        autofocus: true,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final tab in ShellTab.values)
              NavListItem(
                icon: _shellPageIcon(tab),
                label: shellPageLabel(tab),
                selected: tab == current,
                onPress: () => _pick(tab),
              ),
          ],
        ),
      ),
    );
  }
}
