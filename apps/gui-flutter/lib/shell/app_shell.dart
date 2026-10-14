import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/history_restore_bridge.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/deck_load.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/mixer_page.dart';
import 'package:gui_flutter/settings/settings_page.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/settings/settings_section.dart';
import 'package:gui_flutter/shell/app_header.dart';
import 'package:gui_flutter/shell/app_notice_bridge.dart';
import 'package:gui_flutter/shell/controller_offer_bridge.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/shell_tab.dart';

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
      setState(() {
        _tab = ShellTab.settings;
        _settingsDeepLink = next;
        _settingsDeepLinkEpoch++;
      });
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
            onTabChanged: (tab) => setState(() {
              _tab = tab;
              if (tab == ShellTab.settings) {
                _settingsDeepLink = null;
              }
            }),
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
