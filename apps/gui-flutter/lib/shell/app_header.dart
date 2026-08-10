import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/shell/shell_tab.dart';
import 'package:gui_flutter/shell/window_title_bar_controls.dart';
import 'package:window_manager/window_manager.dart';

/// Brand | tabs | drag region | status | window controls (desktop).
class AppHeader extends StatelessWidget {
  const AppHeader({
    required this.tab,
    required this.onTabChanged,
    super.key,
  });

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
                  child: Text(
                    'RUST DJ',
                    style: theme.typography.body.sm.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
              ),
            ),
            FButton(
              variant: tab == ShellTab.mixer ? .secondary : .ghost,
              size: .sm,
              mainAxisSize: .min,
              onPress: () => onTabChanged(ShellTab.mixer),
              child: const Text('Mixer'),
            ),
            const SizedBox(width: 4),
            FButton(
              variant: tab == ShellTab.settings ? .secondary : .ghost,
              size: .sm,
              mainAxisSize: .min,
              onPress: () => onTabChanged(ShellTab.settings),
              child: const Text('Settings'),
            ),
            Expanded(
              child: _maybeDrag(
                desktop: desktop,
                child: const SizedBox.expand(),
              ),
            ),
            _maybeDrag(
              desktop: desktop,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8),
                child: Center(
                  child: Text(
                    'Engine idle',
                    style: theme.typography.body.xs.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                ),
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
