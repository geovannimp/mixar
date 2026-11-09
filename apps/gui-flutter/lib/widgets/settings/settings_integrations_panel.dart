import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/curve_import_export_card.dart';
import 'package:gui_flutter/widgets/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/widgets/settings/rekordbox_import_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';

/// Settings → Integrations: read another DJ application's library into Mixar
/// and migrate its curve settings.
class SettingsIntegrationsPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SettingsSectionBody(
      title: l10n.settingsSectionIntegrations,
      description: l10n.settingsIntegrationsDescription,
      cards: [
        const MixxxImportPanel(),
        const RekordboxImportPanel(),
        CurveImportExportCard(draft: draft, onChanged: onChanged),
      ],
    );
  }
}
