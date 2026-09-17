import 'package:flutter/widgets.dart';
import 'package:gui_flutter/settings/settings_section.dart';
import 'package:gui_flutter/shell/m_tappable.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

class SettingsSidebar extends StatelessWidget {
  const new({required this.active, required this.onSelect, super.key});

  final SettingsSection active;
  final ValueChanged<SettingsSection> onSelect;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border(right: BorderSide(color: theme.colors.border)),
      ),
      child: SizedBox(
        width: 176,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final section in kSettingsSections)
              _SettingsNavItem(
                label: section.label,
                icon: _iconFor(section),
                selected: section == active,
                onPress: () => onSelect(section),
              ),
          ],
        ),
      ),
    );
  }
}

class _SettingsNavItem extends StatelessWidget {
  const new({
    required this.label,
    required this.icon,
    required this.selected,
    required this.onPress,
  });

  final String label;
  final IconData icon;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;

    return MTappable(
      selected: selected,
      semanticsLabel: label,
      onPress: onPress,
      builder: (context, state) {
        final fill = selected
            ? colors.primary.withValues(alpha: 0.10)
            : state.hovered
            ? colors.foreground.withValues(alpha: 0.05)
            : null;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: fill,
            border: Border(
              left: BorderSide(
                width: 2,
                color: selected ? colors.primary : const Color(0x00000000),
              ),
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(10, 8, 4, 8),
            child: Row(
              children: [
                Icon(icon, size: 16, color: colors.mutedForeground),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.typography.body.sm.copyWith(
                      color: colors.foreground,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
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
