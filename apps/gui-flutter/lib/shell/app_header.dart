import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/shell/headphone_monitor_controls.dart';
import 'package:gui_flutter/shell/shell_tab.dart';
import 'package:gui_flutter/shell/window_title_bar_controls.dart';
import 'package:window_manager/window_manager.dart';

/// Brand | drag region | status | settings | window controls (desktop).
class AppHeader extends ConsumerWidget {
  const AppHeader({
    required this.appTitle,
    required this.tab,
    required this.onTabChanged,
    super.key,
  });

  final String appTitle;
  final ShellTab tab;
  final ValueChanged<ShellTab> onTabChanged;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final desktop = isDesktopWindow;
    final starting = ref.watch(engineTransportProvider).isLoading;
    final running = ref.watch(engineRunningProvider);
    final engineLabel = starting
        ? 'Engine starting…'
        : running
        ? 'Engine running'
        : 'Engine idle';

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
                      filterQuality: FilterQuality.medium,
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
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 8),
              child: HeadphoneMonitorControls(),
            ),
            _maybeDrag(
              desktop: desktop,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8),
                child: Center(
                  child: Text(
                    engineLabel,
                    style: theme.typography.body.xs.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                ),
              ),
            ),
            FButton.icon(
              variant: .ghost,
              size: .sm,
              semanticsLabel: 'Settings',
              onPress: () => onTabChanged(ShellTab.settings),
              child: const Icon(FLucideIcons.settings),
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
