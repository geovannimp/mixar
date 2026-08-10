import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/mixer_page.dart';
import 'package:gui_flutter/shell/app_header.dart';
import 'package:gui_flutter/shell/shell_tab.dart';

/// Top-level shell: header + Mixer / Settings body (no backend wiring).
class AppShell extends StatefulWidget {
  const AppShell({super.key});

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
  var _tab = ShellTab.mixer;

  @override
  Widget build(BuildContext context) {
    return FScaffold(
      childPad: false,
      header: AppHeader(
        tab: _tab,
        onTabChanged: (tab) => setState(() => _tab = tab),
      ),
      child: switch (_tab) {
        ShellTab.mixer => const MixerPage(),
        ShellTab.settings => const _SettingsPlaceholder(),
      },
    );
  }
}

class _SettingsPlaceholder extends StatelessWidget {
  const _SettingsPlaceholder();

  @override
  Widget build(BuildContext context) {
    return const Center(child: Text('Settings — placeholder'));
  }
}
