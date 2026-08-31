import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_section.dart';

class SettingsSidebar extends StatelessWidget {
  const SettingsSidebar({
    super.key,
    required this.active,
    required this.onSelect,
  });

  final SettingsSection active;
  final ValueChanged<SettingsSection> onSelect;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return SizedBox(
      width: 176,
      child: FSidebar(
        style: .delta(
          constraints: const BoxConstraints.tightFor(width: 176),
          decoration: DecorationDelta.boxDelta(
            border: Border(right: BorderSide(color: theme.colors.border)),
          ),
        ),
        children: [
          FSidebarGroup(
            children: [
              for (final section in kSettingsSections)
                FSidebarItem(
                  selected: section == active,
                  icon: Icon(_iconFor(section), size: 16),
                  label: Text(section.label),
                  onPress: () => onSelect(section),
                ),
            ],
          ),
        ],
      ),
    );
  }
}

IconData _iconFor(SettingsSection section) {
  return switch (section) {
    SettingsSection.audio => FLucideIcons.volume2,
    SettingsSection.mixer => FLucideIcons.slidersHorizontal,
    SettingsSection.waveform => FLucideIcons.audioWaveform,
    SettingsSection.deck => FLucideIcons.disc3,
    SettingsSection.ui => FLucideIcons.panelTop,
    SettingsSection.library => FLucideIcons.library,
    SettingsSection.session => FLucideIcons.history,
    SettingsSection.controllers => FLucideIcons.gamepad2,
  };
}
