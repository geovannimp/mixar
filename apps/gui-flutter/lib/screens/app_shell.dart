import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/models/shell_tab.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/deck_load.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/mixer_page.dart';
import 'package:gui_flutter/screens/settings_page.dart';
import 'package:gui_flutter/widgets/library/history_restore_bridge.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/app_notice_bridge.dart';
import 'package:gui_flutter/widgets/shell/controller_offer_bridge.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Top-level shell: header + Mixer / Settings body.
class AppShell extends ConsumerStatefulWidget {
  const new({required this.appTitle, super.key});

  final String appTitle;

  @override
  ConsumerState<AppShell> createState() => _AppShellState();
}

class _AppShellState extends ConsumerState<AppShell> {
  ShellTab _tab = ShellTab.mixer;
  SettingsDeepLink? _settingsDeepLink;
  var _settingsDeepLinkEpoch = 0;

  /// Leaving the mixer puts the library drawer away. It is a mixer surface that
  /// lives in a route above the whole shell, so otherwise it would sit on top
  /// of the settings page.
  void _closeLibraryDrawer() {
    ref.read(libraryDrawerOpenProvider.notifier).set(false);
  }

  @override
  Widget build(BuildContext context) {
    ref.watch(engineEventsBootstrapProvider);
    ref.watch(libraryEventsBootstrapProvider);
    ref.watch(focusedDeckLoadBootstrapProvider);
    ref.watch(historySettingsBootstrapProvider);
    ref.watch(librarySettingsBootstrapProvider);
    ref.listen<SettingsDeepLink?>(pendingSettingsDeepLinkProvider, (
      prev,
      next,
    ) {
      if (next == null) {
        return;
      }
      _closeLibraryDrawer();
      setState(() {
        _tab = ShellTab.settings;
        _settingsDeepLink = next;
        _settingsDeepLinkEpoch++;
      });
      // Sync clear: [next] is already copied into `_settingsDeepLink`. Avoids a
      // post-frame clear racing dispose and leaving a stale pending link.
      ref.read(pendingSettingsDeepLinkProvider.notifier).clear();
    });
    // Opaque fill: native window is transparent for rounded desktop chrome.
    return ColoredBox(
      color: context.theme.colors.background,
      child: Column(
        children: [
          AppHeader(
            appTitle: widget.appTitle,
            tab: _tab,
            onTabChanged: (tab) {
              if (tab != ShellTab.mixer) {
                _closeLibraryDrawer();
              }
              setState(() {
                _tab = tab;
                // Manual tab changes never carry a deep link; listen sets it.
                _settingsDeepLink = null;
              });
            },
          ),
          const AppNoticeBridge(),
          const ControllerOfferBridge(),
          const HistoryRestoreBridge(),
          Expanded(
            child: switch (_tab) {
              ShellTab.mixer => const MixerPage(),
              // Epoch key remounts SettingsPage for each deep link so scroll /
              // glow apply even when Settings is already open.
              ShellTab.settings => SettingsPage(
                key: ValueKey(_settingsDeepLinkEpoch),
                initialSection:
                    _settingsDeepLink?.section ?? SettingsSection.audio,
                scrollToPreviewBus:
                    _settingsDeepLink?.scrollToPreviewBus ?? false,
                onClose: () => setState(() {
                  _tab = ShellTab.mixer;
                  _settingsDeepLink = null;
                }),
              ),
            },
          ),
        ],
      ),
    );
  }
}
