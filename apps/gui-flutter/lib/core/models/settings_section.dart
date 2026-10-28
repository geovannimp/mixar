import 'package:gui_flutter/l10n/app_localizations.dart';

enum SettingsSection {
  ui,
  audio,
  mixer,
  waveform,
  deck,
  library,
  storage,
  session,
  controllers,
  privacy,
}

extension SettingsSectionLabel on SettingsSection {
  String label(AppLocalizations l10n) => switch (this) {
    SettingsSection.ui => l10n.settingsSectionUi,
    SettingsSection.audio => l10n.settingsSectionAudio,
    SettingsSection.mixer => l10n.settingsSectionMixer,
    SettingsSection.waveform => l10n.settingsSectionWaveform,
    SettingsSection.deck => l10n.settingsSectionDeck,
    SettingsSection.library => l10n.settingsSectionLibrary,
    SettingsSection.storage => l10n.settingsSectionStorage,
    SettingsSection.session => l10n.settingsSectionSession,
    SettingsSection.controllers => l10n.settingsSectionControllers,
    SettingsSection.privacy => l10n.settingsSectionPrivacy,
  };
}

const List<SettingsSection> kSettingsSections = SettingsSection.values;
