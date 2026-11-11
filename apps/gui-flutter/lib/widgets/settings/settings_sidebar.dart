import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/nav_list_item.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// The settings nav column: one row per section the header's search matches.
class SettingsSidebar extends StatelessWidget {
  const new({
    required this.active,
    required this.query,
    required this.onSelect,
    super.key,
  });

  final SettingsSection active;

  /// Live header search text; empty lists every section.
  final String query;

  final ValueChanged<SettingsSection> onSelect;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final matches = matchingSettingsSections(query, l10n);
    return DecoratedBox(
      // Foreground: otherwise a selected / hovered nav item's fill covers the
      // divider along the sidebar's right edge.
      position: DecorationPosition.foreground,
      decoration: BoxDecoration(
        border: Border(right: BorderSide(color: theme.colors.border)),
      ),
      child: SizedBox(
        width: 176,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (matches.isEmpty)
                Padding(
                  padding: const EdgeInsets.all(12),
                  child: Text(
                    l10n.settingsSearchNoResults,
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                )
              else
                for (final section in matches)
                  NavListItem(
                    label: section.label(l10n),
                    icon: _iconFor(section),
                    selected: section == active,
                    onPress: () => onSelect(section),
                  ),
            ],
          ),
        ),
      ),
    );
  }
}

IconData _iconFor(SettingsSection section) {
  return switch (section) {
    SettingsSection.audio => LucideIcons.volume2,
    SettingsSection.mixer => LucideIcons.slidersHorizontal,
    SettingsSection.curves => LucideIcons.spline,
    SettingsSection.waveform => LucideIcons.audioWaveform,
    SettingsSection.deck => LucideIcons.disc3,
    SettingsSection.ui => LucideIcons.panelTop,
    SettingsSection.layout => LucideIcons.layoutDashboard,
    SettingsSection.library => LucideIcons.library,
    SettingsSection.integrations => LucideIcons.plug,
    SettingsSection.storage => LucideIcons.hardDrive,
    SettingsSection.session => LucideIcons.history,
    SettingsSection.privacy => LucideIcons.shield,
    SettingsSection.controllers => LucideIcons.gamepad2,
  };
}
