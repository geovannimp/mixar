import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/shell_tab.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/widgets/mixer/master_meter_header.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/window_title_bar_controls.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:window_manager/window_manager.dart';

/// Brand | drag region | master meter | settings | window controls (desktop).
class AppHeader extends StatelessWidget {
  const new({
    required this.appTitle,
    required this.tab,
    required this.onTabChanged,
    super.key,
  });

  final String appTitle;
  final ShellTab tab;
  final ValueChanged<ShellTab> onTabChanged;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final desktop = isDesktopWindow;

    return ColoredBox(
      color: theme.colors.background,
      child: SizedBox(
        height: 40,
        child: Row(
          children: [
            _maybeDrag(
              desktop: desktop,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: Center(
                  child: ColorFiltered(
                    colorFilter: const ColorFilter.mode(
                      Color(0xFFFFFFFF),
                      BlendMode.srcIn,
                    ),
                    child: Image.asset(
                      'assets/mixar-logo.png',
                      height: 12,
                      semanticLabel: appTitle,
                    ),
                  ),
                ),
              ),
            ),
            Expanded(
              child: _maybeDrag(
                desktop: desktop,
                child: const SizedBox.expand(),
              ),
            ),
            const MasterMeterHeader(),
            AppTooltip(
              tip: 'Settings',
              child: AppButton.icon(
                variant: .ghost,
                size: .sm,
                semanticsLabel: 'Settings',
                onPress: () => onTabChanged(ShellTab.settings),
                child: const Icon(LucideIcons.settings),
              ),
            ),
            if (desktop) const WindowTitleBarControls(),
          ],
        ),
      ),
    );
  }

  Widget _maybeDrag({required bool desktop, required Widget child}) {
    if (!desktop) {
      return child;
    }
    return DragToMoveArea(child: child);
  }
}
