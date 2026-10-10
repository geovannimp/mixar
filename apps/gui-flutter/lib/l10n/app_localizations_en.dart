// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get settingsTitle => 'SETTINGS';

  @override
  String get settingsSubtitle =>
      'Saving restarts the engine automatically if it\'s running.';

  @override
  String get settingsLoading => 'Loading settings…';

  @override
  String get settingsCloseSemantics => 'Close';

  @override
  String get settingsSave => 'Save';

  @override
  String get settingsSaving => 'Saving…';

  @override
  String get settingsSavedToast => 'Settings saved';

  @override
  String get settingsSavedNotAppliedToast => 'Settings saved, but not applied';

  @override
  String get settingsSaveFailedToast => 'Save failed';

  @override
  String get settingsUnsavedTitle => 'Unsaved settings';

  @override
  String get settingsUnsavedBody => 'Save changes before closing?';

  @override
  String get settingsCancel => 'Cancel';

  @override
  String get settingsDiscard => 'Discard';

  @override
  String get commonLoading => 'Loading…';

  @override
  String get commonSync => 'Sync';

  @override
  String get commonClear => 'Clear';

  @override
  String get commonUpdate => 'Update';

  @override
  String get commonImport => 'Import';

  @override
  String get commonNone => 'None';

  @override
  String get settingsSectionAudio => 'Audio';

  @override
  String get settingsSectionMixer => 'Mixer';

  @override
  String get settingsSectionWaveform => 'Waveform';

  @override
  String get settingsSectionDeck => 'Deck';

  @override
  String get settingsSectionUi => 'UI';

  @override
  String get settingsSectionLibrary => 'Library';

  @override
  String get settingsSectionStorage => 'Storage';

  @override
  String get settingsSectionSession => 'Session';

  @override
  String get settingsSectionControllers => 'Controllers';

  @override
  String get settingsUiDescription =>
      'Chrome and hover tips for the desktop app.';

  @override
  String get settingsUiLanguageLabel => 'Language';

  @override
  String get settingsLanguageSystemDefault => 'System default';

  @override
  String get settingsShowTooltips => 'Show tooltips';

  @override
  String get settingsAudioDescription => 'Engine output and buses.';

  @override
  String get settingsAudioBackend => 'Backend';

  @override
  String get settingsAudioLowLatency => 'Low latency';

  @override
  String get settingsAudioSampleRate => 'Sample rate';

  @override
  String get settingsAudioSampleRateLoading =>
      'Loading rates for the master output device…';

  @override
  String get settingsAudioResamplerQuality => 'Resampler quality';

  @override
  String get settingsAudioBufferSize => 'Buffer size';

  @override
  String get settingsAudioBufferSizeHint =>
      'Must be a multiple of 64 frames (mixer graph chunk size).';

  @override
  String settingsAudioBufferSizeSemantics(int frames) {
    return '$frames frames';
  }

  @override
  String get settingsAudioMasterBus => 'Master bus';

  @override
  String get settingsAudioPreviewBus => 'Preview bus (headphones / cue)';

  @override
  String get settingsAudioDevice => 'Device';

  @override
  String get settingsAudioChannelMode => 'Channel mode';

  @override
  String get settingsAudioStereoPair => 'Stereo pair';

  @override
  String get settingsAudioMonoFold => 'Mono (fold L+R)';

  @override
  String get settingsAudioLeftChannel => 'Left channel';

  @override
  String get settingsAudioRightChannel => 'Right channel';

  @override
  String get settingsAudioLoadingDevices => 'Loading devices…';

  @override
  String get settingsMixerDescription =>
      'Keep analyzed tracks near a consistent perceived loudness.';

  @override
  String get settingsMixerVolumeNormalizer => 'Volume normalizer';

  @override
  String get settingsMixerTargetLufs => 'Target LUFS';

  @override
  String get settingsWaveformDescription =>
      'RGB mixes low/mid/high into one color. Filtered stacks the three bands.';

  @override
  String get settingsWaveformDisplayMode => 'Display mode';

  @override
  String get settingsWaveformModeRgb => 'RGB';

  @override
  String get settingsWaveformModeFiltered => 'Filtered';

  @override
  String get settingsDeckDescription =>
      'Default jog, tempo, and sampler behavior for new decks.';

  @override
  String get settingsDeckJogTitle => 'Jog wheel';

  @override
  String get settingsDeckJogDescription =>
      'Defaults for top (touch) and outer (freewheel) platter policy.';

  @override
  String get settingsDeckTopJogMode => 'Top jog mode';

  @override
  String get settingsDeckOuterJogMode => 'Outer jog mode';

  @override
  String get settingsDeckJogVinyl => 'Vinyl (scratch)';

  @override
  String get settingsDeckJogPitchBend => 'Pitch bend';

  @override
  String get settingsDeckJogIgnore => 'Ignore';

  @override
  String get settingsDeckTempoKeyTitle => 'Tempo and Key';

  @override
  String get settingsDeckTempoKeyDescription =>
      'Default pitch-fader range and key lock for new decks.';

  @override
  String get settingsDeckDefaultTempoRange => 'Default tempo range';

  @override
  String get settingsDeckDefaultKeyLock => 'Default key lock';

  @override
  String get settingsDeckDefaultKeyLockHint =>
      'Tempo-only pitch when on (time-stretch). Off = vinyl tempo.';

  @override
  String get settingsDeckKeyLock => 'Key lock';

  @override
  String get settingsDeckSamplerTitle => 'Sampler';

  @override
  String get settingsDeckSamplerDescription =>
      'Default play mode for inherit banks and default bank per deck.';

  @override
  String get settingsDeckSamplerPlayMode => 'Sampler play mode';

  @override
  String get settingsDeckSamplerPlayOneshot => 'Oneshot';

  @override
  String get settingsDeckSamplerPlayHold => 'Hold';

  @override
  String get settingsDeckSamplerPlayLoop => 'Loop';

  @override
  String get settingsDeckSamplerStripRoute => 'Sampler strip route';

  @override
  String get settingsDeckSamplerStripBefore => 'Before channel strip';

  @override
  String get settingsDeckSamplerStripAfter => 'After channel strip';

  @override
  String get settingsDeckDefaultSamplerBankA => 'Deck A default sampler bank';

  @override
  String get settingsDeckDefaultSamplerBankB => 'Deck B default sampler bank';

  @override
  String get settingsLibraryDescription =>
      'Track import, offline analysis, and list display.';

  @override
  String get settingsLibraryAnalysisQuality => 'Analysis quality';

  @override
  String get settingsLibraryAnalysisFast => 'Fast';

  @override
  String get settingsLibraryAnalysisFastSubtitle =>
      'Analyze a short preview for quick library scans.';

  @override
  String get settingsLibraryAnalysisPrecise => 'Precise';

  @override
  String get settingsLibraryAnalysisPreciseSubtitle =>
      'Balanced analysis for most libraries.';

  @override
  String get settingsLibraryAnalysisComplete => 'Complete';

  @override
  String get settingsLibraryAnalysisCompleteSubtitle =>
      'Analyze the full track (slowest, most accurate).';

  @override
  String get settingsLibraryMusicalKeyTitle => 'Musical key';

  @override
  String get settingsLibraryMusicalKeyDescription =>
      'How keys are labeled and color-coded in deck chrome and the library table.';

  @override
  String get settingsLibraryKeyDisplayMode => 'Key display mode';

  @override
  String get settingsLibraryKeyDisplayMusical => 'Musical';

  @override
  String get settingsLibraryKeyDisplayMusicalSubtitle =>
      'Note names in deck chip and library key column — e.g. C, Am, F#m.';

  @override
  String get settingsLibraryKeyDisplayCamelot => 'Camelot';

  @override
  String get settingsLibraryKeyDisplayCamelotSubtitle =>
      'Mixed In Key codes — e.g. 8B (C major), 8A (A minor), 11B.';

  @override
  String get settingsLibraryKeyColorMode => 'Key color mode';

  @override
  String get settingsLibraryKeyColorOff => 'Off';

  @override
  String get settingsLibraryKeyColorOffSubtitle =>
      'Key labels use the default text color everywhere.';

  @override
  String get settingsLibraryKeyColorAbsolute => 'Absolute (circle of fifths)';

  @override
  String get settingsLibraryKeyColorAbsoluteSubtitle =>
      'Fixed color per key on the wheel — majors vivid, minors muted (e.g. 8B bright, 8A softer).';

  @override
  String get settingsLibraryKeyColorHarmonic => 'Harmonic (playing deck)';

  @override
  String get settingsLibraryKeyColorHarmonicSubtitle =>
      'Green/yellow vs the playing deck — e.g. with 2A playing, 1A/2A/3A/2B green, 1B/3B yellow.';

  @override
  String get settingsLibraryStemFormat => 'Stem format';

  @override
  String get settingsLibraryStemOpus => 'Opus (160 kbps)';

  @override
  String get settingsLibraryStemFlac => 'FLAC (lossless)';

  @override
  String get settingsLibraryDimPlayedTracks => 'Dim played tracks';

  @override
  String get settingsLibraryTrackRowLayout => 'Track row layout';

  @override
  String get settingsLibraryRowCompact => 'Compact';

  @override
  String get settingsLibraryRowCompactSubtitle =>
      'One dense line per track — fits the most rows on screen.';

  @override
  String get settingsLibraryRowComfortable => 'Comfortable';

  @override
  String get settingsLibraryRowComfortableSubtitle =>
      'Two lines per track: title above artist, BPM, key and length.';

  @override
  String get settingsSessionDescription =>
      'Performance history and session boundaries.';

  @override
  String get settingsSessionHistoryTitle => 'Performance history';

  @override
  String get settingsSessionHistoryDescription =>
      'Log deck playback to XSPF session files under app support.';

  @override
  String get settingsSessionRecordHistory => 'Record performance history';

  @override
  String get settingsSessionIdleTimeout => 'Session idle timeout';

  @override
  String get settingsSessionIdleTimeoutHint =>
      'Close after this long with no qualifying deck output.';

  @override
  String get settingsSessionMinPlayDuration => 'Minimum play duration';

  @override
  String get settingsSessionMinPlayDurationHint =>
      'Commit entries after this much qualifying playback.';

  @override
  String get settingsSessionMinutes => 'minutes';

  @override
  String get settingsSessionSeconds => 'seconds';

  @override
  String get settingsSessionMinDeckVolume => 'Minimum effective deck volume';

  @override
  String settingsSessionPercentSemantics(int percent) {
    return '$percent percent';
  }

  @override
  String get settingsStorageDescription =>
      'Disk use for Mixar caches and library metadata under app support.';

  @override
  String get settingsStorageMixarStorage => 'Mixar storage';

  @override
  String settingsStorageUsed(String size) {
    return '$size used';
  }

  @override
  String get settingsStorageStemCache => 'Stem cache';

  @override
  String get settingsStorageStemModel => 'Stem model';

  @override
  String get settingsStorageWaveform => 'Waveform';

  @override
  String get settingsStorageTrackMetadata => 'Track metadata';

  @override
  String get settingsStorageSyncStemTitle => 'Sync stem cache?';

  @override
  String get settingsStorageSyncStemBody =>
      'Removes stem files that are not listed in the library database, and drops DB rows whose files are missing. Referenced cache files and original audio are kept.';

  @override
  String get settingsStorageSyncFailed => 'Sync failed';

  @override
  String get settingsStorageClearFailed => 'Clear failed';

  @override
  String get settingsStorageClearStemTitle => 'Clear stem cache?';

  @override
  String get settingsStorageClearStemBody =>
      'Deletes generated stem files for all tracks. Original library audio is not touched.';

  @override
  String get settingsStorageClearStemOk => 'Stem cache cleared';

  @override
  String get settingsStorageClearModelTitle => 'Clear stem model cache?';

  @override
  String get settingsStorageClearModelBody =>
      'Deletes downloaded stem separation models. They will re-download when needed.';

  @override
  String get settingsStorageClearModelOk => 'Stem model cleared';

  @override
  String get settingsStorageClearWaveformTitle => 'Clear waveform cache?';

  @override
  String get settingsStorageClearWaveformBody =>
      'Deletes cached waveform overviews. They regenerate when you open a track.';

  @override
  String get settingsStorageClearWaveformOk => 'Waveform cache cleared';

  @override
  String get settingsControllersDescription =>
      'MIDI mappings and connected hardware. Trust a device to auto-enable it on connect after Save.';

  @override
  String get settingsControllersMappingsTitle => 'Mappings';

  @override
  String get settingsControllersMappingsDescription =>
      'Stored in app data. Seed copies shipped maps when missing; Update overwrites from the app bundle.';

  @override
  String get settingsControllersUpdateAll => 'Update All';

  @override
  String get settingsControllersNoMappings => 'No mappings in app data yet.';

  @override
  String get settingsControllersMidiPortsTitle => 'MIDI ports';

  @override
  String get settingsControllersMidiPortsDescription =>
      'Detected MIDI inputs and outputs, and the mapping each one matches.';

  @override
  String get settingsControllersNoMidiPorts => 'No MIDI ports detected.';

  @override
  String get settingsControllersNoMapping => 'No mapping';

  @override
  String settingsControllersMappingArrow(String mapping) {
    return '→ $mapping';
  }

  @override
  String get settingsControllersDirectionIn => 'IN';

  @override
  String get settingsControllersDirectionOut => 'OUT';

  @override
  String get settingsControllersAttached => 'Attached';

  @override
  String get settingsControllersUpdateAvailable => 'Update available';

  @override
  String get settingsControllersTrust => 'Trust';

  @override
  String get settingsControllersAttach => 'Attach';

  @override
  String settingsControllersTrustSemantics(String name) {
    return 'Trust device $name';
  }

  @override
  String settingsControllersEnableSemantics(String name) {
    return 'Enable $name';
  }

  @override
  String get mixxxImportTitle => 'Import from Mixxx';

  @override
  String get mixxxImportDescription =>
      'Bring tracks, playlists, and crates from your Mixxx library into Mixar.';

  @override
  String mixxxImportFound(String path) {
    return 'Found: $path';
  }

  @override
  String get mixxxImportCheckFailed => 'Could not check for a Mixxx library.';

  @override
  String get mixxxImportLooking => 'Looking for a Mixxx library…';

  @override
  String get mixxxImportNotFound => 'No Mixxx library found on this computer.';

  @override
  String get mixxxImportButton => 'Import from Mixxx library…';

  @override
  String get mixxxImporting => 'Importing…';

  @override
  String get mixxxImportConfirmTitle => 'Import from Mixxx?';

  @override
  String mixxxImportConfirmBody(
    String path,
    int trackCount,
    int missingFileCount,
    int playlistCount,
    int crateCount,
    int folderCount,
  ) {
    return 'This imports tracks, playlists, crates, and watched folders from:\n$path\n\n$trackCount tracks ($missingFileCount missing), $playlistCount playlists, $crateCount crates, $folderCount folders.\n\nMissing files are imported as unavailable tracks.';
  }

  @override
  String get mixxxImportReadFailed => 'Could not read Mixxx library';

  @override
  String get mixxxImportFailed => 'Mixxx import failed';

  @override
  String mixxxImportFinishedWithErrors(int count) {
    return 'Mixxx import finished with $count error(s)';
  }

  @override
  String get mixxxImportAlreadyImported => 'Mixxx library already imported';

  @override
  String get mixxxImportNothing => 'Nothing to import from Mixxx';

  @override
  String mixxxImportUpdated(String updated, String missing) {
    return 'Updated $updated from Mixxx$missing';
  }

  @override
  String mixxxImportImported(String parts, String missing) {
    return 'Imported $parts from Mixxx$missing';
  }

  @override
  String mixxxImportMissingSuffix(int count) {
    return ' ($count missing)';
  }

  @override
  String mixxxCountTracks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count tracks',
      one: '1 track',
    );
    return '$_temp0';
  }

  @override
  String mixxxCountPlaylists(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count playlists',
      one: '1 playlist',
    );
    return '$_temp0';
  }

  @override
  String mixxxCountCrates(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count crates',
      one: '1 crate',
    );
    return '$_temp0';
  }

  @override
  String mixxxCountFolders(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count folders',
      one: '1 folder',
    );
    return '$_temp0';
  }

  @override
  String mixxxCountUpdated(int count) {
    return '$count updated';
  }

  @override
  String get settingsStorageStemAlreadyInSync => 'Stem cache already in sync';

  @override
  String settingsStorageRemovedOrphans(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Removed $count orphan stem files',
      one: 'Removed 1 orphan stem file',
    );
    return '$_temp0';
  }
}
