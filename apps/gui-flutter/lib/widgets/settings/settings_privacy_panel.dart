import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';

class SettingsPrivacyPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        SettingsSectionHeader(
          title: l10n.settingsSectionPrivacy,
          description: l10n.settingsPrivacyDescription,
        ),
        SettingsSectionCard(
          title: l10n.settingsAnalyticsTitle,
          subtitle: l10n.settingsAnalyticsSubtitle,
          trailing: SettingsHeaderSwitch(
            value: draft.analyticsEnabled,
            semanticsLabel: l10n.settingsAnalyticsToggle,
            onChanged: (enabled) =>
                onChanged(copyAppSettings(draft, analyticsEnabled: enabled)),
          ),
        ),
      ],
    );
  }
}
