import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/shell_tab.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_pages_menu.dart';
import 'package:gui_flutter/widgets/shell/macos_title_bar_chrome.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/window_title_bar_controls.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:window_manager/window_manager.dart';

/// Height of the app header strip.
///
/// Its body below is the stage for full-window overlays: the mixer's library
/// drawer is inset by this, so it never covers the window controls.
const double kAppHeaderHeight = 40;

/// Pages title (centre) | drag region | library drawer | window controls.
class AppHeader extends ConsumerWidget {
  const new({
    required this.appTitle,
    required this.tab,
    required this.onTabChanged,
    super.key,
  });

  final String appTitle;
  final ShellTab tab;
  final Future<void> Function(ShellTab) onTabChanged;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final desktop = isDesktopWindow;
    final settings = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s, orElse: defaultAppSettings);
    // The mixer forces the drawer when the body leaves too little room to dock
    // the library, so the toggle has to appear even with docked mode set.
    final drawerAvailable =
        settings.libraryDrawerEnabled ||
        ref.watch(mixerLibraryForcedDrawerProvider);

    // Card, not the app canvas: the header sits directly on the decks/mixer
    // panel, so both read as one piece of chrome in every theme.
    //
    // The bottom hairline is painted in the foreground so it lands on top of
    // the card fill. It lives here, not on the waveform pane, so Settings gets
    // the same separation under the tab bar.
    return DecoratedBox(
      position: DecorationPosition.foreground,
      decoration: BoxDecoration(
        border: Border(
          bottom: BorderSide(
            color: theme.colors.border,
            width: theme.style.borderWidth,
          ),
        ),
      ),
      child: ColoredBox(
        color: theme.colors.card,
        child: MacOSTitleBarChrome(
          child: SizedBox(
            height: kAppHeaderHeight,
            // A stack, not a row: the title is pinned to the window's centre.
            // In a row it would drift every time the trailing chrome changed,
            // and the Library toggle only exists on the mixer.
            child: Stack(
              fit: StackFit.expand,
              children: [
                // The chrome the buttons leave free moves the window. It sits
                // under the controls, so a press on one never drags.
                _maybeDrag(desktop: desktop, child: const SizedBox.expand()),
                // The wordmark names the app and the page beside it opens the
                // pages menu; macOS keeps its traffic lights in the far corner
                // and draws them over this strip.
                Center(
                  child: SizedBox(
                    height: kAppHeaderHeight,
                    child: AppPagesMenu(
                      appTitle: appTitle,
                      current: tab,
                      onSelect: onTabChanged,
                    ),
                  ),
                ),
                // The way back from every page but the booth sits where the
                // old hamburger did; it runs the same page switch, so a dirty
                // Settings draft still asks before it closes.
                if (tab != ShellTab.mixer)
                  Positioned(
                    left: usesMacOSTitleBarLayout
                        ? kMacOSWindowControlsInset
                        : 0,
                    top: 0,
                    bottom: 0,
                    child: SizedBox(
                      height: kAppHeaderHeight,
                      child: AppButton(
                        variant: .ghost,
                        size: .sm,
                        mainAxisSize: .min,
                        semanticsLabel: 'Back to Booth',
                        onPress: () => unawaited(onTabChanged(ShellTab.mixer)),
                        child: const Row(
                          mainAxisSize: .min,
                          children: [
                            Icon(LucideIcons.arrowLeft),
                            SizedBox(width: 6),
                            Text('Booth'),
                          ],
                        ),
                      ),
                    ),
                  ),
                Positioned(
                  top: 0,
                  bottom: 0,
                  right: 0,
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      // Only when the library lives in a drawer, and only on
                      // the mixer: the docked mode has nothing to toggle.
                      if (drawerAvailable && tab == ShellTab.mixer)
                        SizedBox(
                          height: kAppHeaderHeight,
                          child: AppButton(
                            variant: .ghost,
                            size: .sm,
                            mainAxisSize: .min,
                            onPress: () => ref
                                .read(libraryDrawerOpenProvider.notifier)
                                .toggle(),
                            child: const Row(
                              mainAxisSize: .min,
                              children: [
                                Icon(LucideIcons.libraryBig),
                                SizedBox(width: 6),
                                Text('Library'),
                              ],
                            ),
                          ),
                        ),
                      if (desktop && !usesMacOSTitleBarLayout)
                        const WindowTitleBarControls(),
                    ],
                  ),
                ),
              ],
            ),
          ),
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
