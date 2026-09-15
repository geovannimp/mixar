import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_section.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

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
    SettingsSection.audio => LucideIcons.volume2,
    SettingsSection.mixer => LucideIcons.slidersHorizontal,
    SettingsSection.waveform => LucideIcons.audioWaveform,
    SettingsSection.deck => LucideIcons.disc3,
    SettingsSection.ui => LucideIcons.panelTop,
    SettingsSection.library => LucideIcons.library,
    SettingsSection.session => LucideIcons.history,
    SettingsSection.controllers => LucideIcons.gamepad2,
  };
}
