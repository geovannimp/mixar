enum SettingsSection {
  audio,
  mixer,
  waveform,
  deck,
  ui,
  library,
  storage,
  session,
  controllers,
}

extension SettingsSectionLabel on SettingsSection {
  String get label => switch (this) {
    SettingsSection.audio => 'Audio',
    SettingsSection.mixer => 'Mixer',
    SettingsSection.waveform => 'Waveform',
    SettingsSection.deck => 'Deck',
    SettingsSection.ui => 'UI',
    SettingsSection.library => 'Library',
    SettingsSection.storage => 'Storage',
    SettingsSection.session => 'Session',
    SettingsSection.controllers => 'Controllers',
  };
}

const List<SettingsSection> kSettingsSections = SettingsSection.values;
