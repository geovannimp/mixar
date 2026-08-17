import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/mixer_page.dart';
import 'package:gui_flutter/shell/app_header.dart';
import 'package:gui_flutter/shell/controller_offer_bridge.dart';
import 'package:gui_flutter/shell/settings_page.dart';
import 'package:gui_flutter/shell/shell_tab.dart';

/// Top-level shell: header + Mixer / Settings body.
class AppShell extends ConsumerStatefulWidget {
  const AppShell({required this.appTitle, super.key});

  final String appTitle;

  @override
  ConsumerState<AppShell> createState() => _AppShellState();
}

class _AppShellState extends ConsumerState<AppShell> {
  var _tab = ShellTab.mixer;

  @override
  Widget build(BuildContext context) {
    ref.watch(engineEventsBootstrapProvider);
    return FScaffold(
      childPad: false,
      header: AppHeader(
        appTitle: widget.appTitle,
        tab: _tab,
        onTabChanged: (tab) => setState(() => _tab = tab),
      ),
      child: Column(
        children: [
          const ControllerOfferBridge(),
          Expanded(
            child: switch (_tab) {
              ShellTab.mixer => const MixerPage(),
              ShellTab.settings => const SettingsPage(),
            },
          ),
        ],
      ),
    );
  }
}
