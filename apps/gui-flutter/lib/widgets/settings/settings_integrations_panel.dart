import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/widgets/settings/rekordbox_import_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';

/// Settings → Integrations: read another DJ application's library into Mixar.
///
/// Each source is one card that detects its own database and runs the import.
/// Nothing here is a saved preference, so the panel takes no draft: the cards
/// work straight from the page.
class SettingsIntegrationsPanel extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SettingsSectionBody(
      title: l10n.settingsSectionIntegrations,
      description: l10n.settingsIntegrationsDescription,
      cards: const [MixxxImportPanel(), RekordboxImportPanel()],
    );
  }
}
