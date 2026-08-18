enum SettingsSection { audio, mixer, waveform, deck, library, controllers }

extension SettingsSectionLabel on SettingsSection {
  String get label => switch (this) {
    SettingsSection.audio => 'Audio',
    SettingsSection.mixer => 'Mixer',
    SettingsSection.waveform => 'Waveform',
    SettingsSection.deck => 'Deck',
    SettingsSection.library => 'Library',
    SettingsSection.controllers => 'Controllers',
  };
}

const kSettingsSections = SettingsSection.values;
