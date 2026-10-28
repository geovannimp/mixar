import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/shell_tab.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/window_title_bar_controls.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:window_manager/window_manager.dart';

/// Height of the app header strip.
///
/// Its body below is the stage for full-window overlays: the mixer's library
/// drawer is inset by this, so it never covers the window controls.
const double kAppHeaderHeight = 40;

/// Brand | drag region | library drawer | settings | window controls (desktop).
class AppHeader extends ConsumerWidget {
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
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final desktop = isDesktopWindow;
    final settings = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s, orElse: defaultAppSettings);
    final drawerOpen = ref.watch(libraryDrawerOpenProvider);

    // Card, not the app canvas: the header sits directly on the decks/mixer
    // panel, so both read as one piece of chrome in every theme.
    return ColoredBox(
      color: theme.colors.card,
      child: SizedBox(
        height: kAppHeaderHeight,
        child: Row(
          children: [
            _maybeDrag(
              desktop: desktop,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: Center(
                  child: ColorFiltered(
                    colorFilter: ColorFilter.mode(
                      theme.colors.foreground,
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
            // Only in drawer mode, and only on the mixer: the docked library
            // mode has nothing to toggle. Hidden while settings load, since
            // `defaultAppSettings` has the drawer off.
            if (settings.libraryDrawerEnabled && tab == ShellTab.mixer)
              AppTooltip(
                tip: drawerOpen ? 'Close library' : 'Open library',
                child: AppButton.icon(
                  variant: .ghost,
                  size: .sm,
                  semanticsLabel: drawerOpen ? 'Close library' : 'Open library',
                  onPress: () =>
                      ref.read(libraryDrawerOpenProvider.notifier).toggle(),
                  child: const Icon(LucideIcons.libraryBig),
                ),
              ),
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
