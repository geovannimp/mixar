import 'package:gui_flutter/l10n/app_localizations.dart';

enum SettingsSection {
  ui,
  layout,
  audio,
  mixer,
  curves,
  waveform,
  deck,
  library,
  integrations,
  storage,
  session,
  controllers,
  privacy,
}

extension SettingsSectionLabel on SettingsSection {
  String label(AppLocalizations l10n) => switch (this) {
    SettingsSection.ui => l10n.settingsSectionUi,
    SettingsSection.layout => l10n.settingsSectionLayout,
    SettingsSection.audio => l10n.settingsSectionAudio,
    SettingsSection.mixer => l10n.settingsSectionMixer,
    SettingsSection.curves => l10n.settingsSectionCurves,
    SettingsSection.waveform => l10n.settingsSectionWaveform,
    SettingsSection.deck => l10n.settingsSectionDeck,
    SettingsSection.library => l10n.settingsSectionLibrary,
    SettingsSection.integrations => l10n.settingsSectionIntegrations,
    SettingsSection.storage => l10n.settingsSectionStorage,
    SettingsSection.session => l10n.settingsSectionSession,
    SettingsSection.controllers => l10n.settingsSectionControllers,
    SettingsSection.privacy => l10n.settingsSectionPrivacy,
  };
}

/// Sections shown in Settings. The curve editor is intentionally absent until
/// the dedicated editor ships.
const List<SettingsSection> kSettingsSections = [
  SettingsSection.ui,
  SettingsSection.layout,
  SettingsSection.audio,
  SettingsSection.mixer,
  SettingsSection.waveform,
  SettingsSection.deck,
  SettingsSection.library,
  SettingsSection.integrations,
  SettingsSection.storage,
  SettingsSection.session,
  SettingsSection.controllers,
  SettingsSection.privacy,
];
