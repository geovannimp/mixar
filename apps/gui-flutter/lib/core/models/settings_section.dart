import 'package:gui_flutter/core/utils/settings_search.dart';
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

  /// One-line summary shown under the section title, also used by the sidebar
  /// search so a query like "midi" finds Controllers.
  String description(AppLocalizations l10n) => switch (this) {
    SettingsSection.ui => l10n.settingsUiDescription,
    SettingsSection.layout => l10n.settingsLayoutDescription,
    SettingsSection.audio => l10n.settingsAudioDescription,
    SettingsSection.mixer => l10n.settingsMixerDescription,
    SettingsSection.curves => l10n.settingsCurvesDescription,
    SettingsSection.waveform => l10n.settingsWaveformDescription,
    SettingsSection.deck => l10n.settingsDeckDescription,
    SettingsSection.library => l10n.settingsLibraryDescription,
    SettingsSection.integrations => l10n.settingsIntegrationsDescription,
    SettingsSection.storage => l10n.settingsStorageDescription,
    SettingsSection.session => l10n.settingsSessionDescription,
    SettingsSection.controllers => l10n.settingsControllersDescription,
    SettingsSection.privacy => l10n.settingsPrivacyDescription,
  };

  /// Localized card titles, subtitles, and field labels used by the sidebar
  /// search, so a query like "Rekordbox" or "cue" finds the owning section even
  /// before its panel is built.
  ///
  /// These are fallback terms: [matchingSettingsSections] only consults them
  /// when no section matches by its own label, description, or enum name, so
  /// prose such as "restarts the audio engine" cannot make Layout and Mixer
  /// match an `audio` query.
  List<String> searchTerms(AppLocalizations l10n) => switch (this) {
    SettingsSection.ui => [
      l10n.settingsUiThemeTitle,
      l10n.settingsUiThemeDescription,
      l10n.settingsUiChromeTitle,
      l10n.settingsUiChromeDescription,
      l10n.settingsUiOnboardingTitle,
      l10n.settingsUiOnboardingDescription,
      l10n.settingsShowTooltips,
      l10n.settingsUiAccentColor,
      l10n.settingsUiLanguageLabel,
      l10n.settingsUiSelectStyle,
    ],
    SettingsSection.layout => [
      l10n.settingsLayoutPresetTitle,
      l10n.settingsLayoutPresetDescription,
      l10n.settingsLayoutDeckCount,
      l10n.settingsLayoutDeckCountDescription,
      l10n.settingsLayoutMixerTitle,
      l10n.settingsLayoutMixerDescription,
      l10n.settingsLayoutFxBarTitle,
      l10n.settingsLayoutFxBarDescription,
      l10n.settingsLayoutWaveformTitle,
      l10n.settingsLayoutWaveformDescription,
      l10n.settingsLayoutLibraryDrawerTitle,
      l10n.settingsLayoutLibraryDrawerDescription,
      l10n.settingsLayoutMixerMeters,
    ],
    SettingsSection.audio => [
      l10n.settingsAudioEngineTitle,
      l10n.settingsAudioEngineDescription,
      l10n.settingsAudioMasterBus,
      l10n.settingsAudioMasterBusDescription,
      l10n.settingsAudioPreviewBusTitle,
      l10n.settingsAudioPreviewBusDescription,
      l10n.settingsAudioResamplerQuality,
      l10n.settingsAudioBufferSize,
    ],
    SettingsSection.mixer => [
      l10n.settingsMixerCrossfaderTitle,
      l10n.settingsMixerCrossfaderDescription,
      l10n.settingsMixerNormalizerTitle,
      l10n.settingsMixerNormalizerDescription,
      l10n.settingsMixerSamplerRoutingTitle,
      l10n.settingsMixerSamplerRoutingDescription,
      l10n.settingsMixerRealtimeStemsTitle,
      l10n.settingsMixerRealtimeStemsDescription,
    ],
    SettingsSection.curves => [
      l10n.curvesCustomTitle,
      l10n.curvesCustomDescription,
      l10n.curvesSetsTitle,
      l10n.curvesSetsDescription,
      l10n.curvesImportExportTitle,
      l10n.curvesImportExportDescription,
    ],
    SettingsSection.waveform => [
      l10n.settingsWaveformDisplayTitle,
      l10n.settingsWaveformDisplayDescription,
    ],
    SettingsSection.deck => [
      l10n.settingsDeckJogTitle,
      l10n.settingsDeckJogDescription,
      l10n.settingsDeckTempoKeyTitle,
      l10n.settingsDeckTempoKeyDescription,
      l10n.settingsDeckSamplerTitle,
      l10n.settingsDeckSamplerDescription,
      l10n.settingsDeckVinylBrakePause,
    ],
    SettingsSection.library => [
      l10n.settingsLibraryAnalysisTitle,
      l10n.settingsLibraryAnalysisDescription,
      l10n.settingsLibraryTagWriteBackTitle,
      l10n.settingsLibraryTagWriteBackDescription,
      l10n.settingsLibraryMusicalKeyTitle,
      l10n.settingsLibraryMusicalKeyDescription,
      l10n.settingsLibraryListTitle,
      l10n.settingsLibraryListDescription,
      l10n.settingsLibraryVisibleFields,
      l10n.settingsLibraryStemsTitle,
      l10n.settingsLibraryStemsDescription,
    ],
    SettingsSection.integrations => [
      l10n.mixxxImportTitle,
      l10n.mixxxImportDescription,
      l10n.rekordboxImportTitle,
      l10n.rekordboxImportDescription,
      l10n.curvesImportExportTitle,
      l10n.curvesImportExportDescription,
    ],
    SettingsSection.storage => [
      l10n.settingsStorageOverviewTitle,
      l10n.settingsStorageOverviewDescription,
      l10n.settingsStorageClearStemTitle,
      l10n.settingsStorageClearModelTitle,
      l10n.settingsStorageClearWaveformTitle,
      l10n.settingsStorageSyncStemTitle,
    ],
    SettingsSection.session => [
      l10n.settingsSessionHistoryTitle,
      l10n.settingsSessionHistoryDescription,
    ],
    SettingsSection.controllers => [
      l10n.settingsControllersMappingsTitle,
      l10n.settingsControllersMappingsDescription,
      l10n.settingsControllersMidiPortsTitle,
      l10n.settingsControllersMidiPortsDescription,
      l10n.settingsControllersAudio,
    ],
    SettingsSection.privacy => [
      l10n.settingsAnalyticsTitle,
      l10n.settingsAnalyticsSubtitle,
    ],
  };

  /// Whether the section's own label, description, or enum name matches.
  bool matchesSectionSearch(String foldedQuery, AppLocalizations l10n) =>
      settingsSearchMatches(
        '${label(l10n)} ${description(l10n)} $name',
        foldedQuery,
      );

  /// Whether any card title, subtitle, or field label matches.
  bool matchesCardSearch(String foldedQuery, AppLocalizations l10n) =>
      settingsSearchMatches(searchTerms(l10n).join(' '), foldedQuery);
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

/// Sections shown in Settings that match [query]; every section when the
/// trimmed query is empty.
///
/// Section-level matches (label, description, enum name) win outright. Card
/// terms are only consulted when no section matches by its own metadata, so a
/// card subtitle mentioning "audio engine" does not make Layout and Mixer
/// match an `audio` query.
List<SettingsSection> matchingSettingsSections(
  String query,
  AppLocalizations l10n,
) {
  final needle = foldSettingsSearchText(query.trim());
  if (needle.isEmpty) {
    return kSettingsSections;
  }
  final bySection = _filterSections(
    (section) => section.matchesSectionSearch(needle, l10n),
  );
  if (bySection.isNotEmpty) {
    return bySection;
  }
  final byCard = _filterSections(
    (section) => section.matchesCardSearch(needle, l10n),
  );
  if (byCard.isNotEmpty) {
    return byCard;
  }
  return _fuzzyMatchingSections(needle, l10n);
}

/// [kSettingsSections] narrowed to those satisfying [test].
///
/// Shared by every match tier in [matchingSettingsSections] so a new fallback
/// stays a one-line addition instead of another copy of the filter-and-return
/// shape.
List<SettingsSection> _filterSections(
  bool Function(SettingsSection section) test,
) => [
  for (final section in kSettingsSections)
    if (test(section)) section,
];

/// Typo-tolerant fallback: Jaro-Winkler scores the query against section
/// labels and card terms, returning the best matches above
/// [settingsSearchFuzzyThreshold] in descending score order.
List<SettingsSection> _fuzzyMatchingSections(
  String needle,
  AppLocalizations l10n,
) {
  final scored = <(SettingsSection, double)>[];
  for (final section in kSettingsSections) {
    final score = settingsSearchFuzzyScore(needle, [
      section.label(l10n),
      section.name,
      ...section.searchTerms(l10n),
    ]);
    if (score >= settingsSearchFuzzyThreshold) {
      scored.add((section, score));
    }
  }
  scored.sort((a, b) => b.$2.compareTo(a.$2));
  return [for (final entry in scored) entry.$1];
}
