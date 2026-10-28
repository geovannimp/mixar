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
  String get settingsSectionPrivacy => 'Privacy';

  @override
  String get settingsPrivacyDescription => 'Anonymous app usage data.';

  @override
  String get settingsUiDescription =>
      'Theme, language, hover tips, control presentation, mixer meters, and the library drawer.';

  @override
  String get settingsUiChromeTitle => 'Chrome and controls';

  @override
  String get settingsUiChromeDescription =>
      'Theme, language, hover tips, and how selects open.';

  @override
  String get settingsUiTheme => 'Theme';

  @override
  String get settingsUiThemeLight => 'Light';

  @override
  String get settingsUiThemeDark => 'Dark';

  @override
  String get settingsUiLanguageLabel => 'Language';

  @override
  String get settingsSystemDefault => 'System default';

  @override
  String get settingsLanguageEn => 'English';

  @override
  String get settingsLanguagePtBr => 'Português (Brasil)';

  @override
  String get settingsShowTooltips => 'Show tooltips';

  @override
  String get settingsUiMixerMetersTitle => 'Mixer meters';

  @override
  String get settingsUiMixerMetersDescription =>
      'Deck VU meters between the volume faders. Master meter sits in the FX bar at the bottom.';

  @override
  String get settingsUiShowMixerMeters => 'Show mixer meters';

  @override
  String get settingsUiMixerMetersMode => 'Meter mode';

  @override
  String get settingsUiMixerMetersMono => 'Mono';

  @override
  String get settingsUiMixerMetersStereo => 'Stereo';

  @override
  String get settingsUiMixerMetersMonoSubtitle =>
      'One ladder per deck (max of left/right).';

  @override
  String get settingsUiMixerMetersStereoSubtitle =>
      'Separate left and right ladders per deck.';

  @override
  String get settingsUiLibraryDrawerTitle => 'Library drawer';

  @override
  String get settingsUiLibraryDrawerDescription =>
      'Where the library opens. Drawer mode hides the docked library and opens it over the mixer.';

  @override
  String get settingsUiLibraryDrawerEnable => 'Library as a drawer';

  @override
  String get settingsUiLibraryDrawerPosition => 'Position';

  @override
  String get settingsUiLibraryDrawerLeft => 'Left';

  @override
  String get settingsUiLibraryDrawerRight => 'Right';

  @override
  String get settingsUiLibraryDrawerTop => 'Top';

  @override
  String get settingsUiLibraryDrawerBottom => 'Bottom';

  @override
  String get settingsUiSelectStyle => 'Select style';

  @override
  String get settingsUiSelectStyleAuto => 'Auto';

  @override
  String get settingsUiSelectStyleDesktop => 'Desktop';

  @override
  String get settingsUiSelectStyleMobile => 'Mobile';

  @override
  String get settingsUiSelectStyleAutoSubtitle =>
      'Platform default — desktop OS uses popovers, phones use dialogs.';

  @override
  String get settingsUiSelectStyleDesktopSubtitle =>
      'Anchored popovers for selects and ⋯ menus.';

  @override
  String get settingsUiSelectStyleMobileSubtitle =>
      'Open selects and ⋯ menus in a dialog.';

  @override
  String get settingsAudioDescription => 'Engine output and buses.';

  @override
  String get settingsAudioEngineTitle => 'Engine';

  @override
  String get settingsAudioEngineDescription => 'Resampler and buffer size.';

  @override
  String get settingsAudioLowLatency => 'Low latency';

  @override
  String get settingsAudioSampleRate => 'Sample rate';

  @override
  String get settingsAudioSampleRateHint =>
      'Device default follows the rate the output device mixes at.';

  @override
  String settingsAudioSampleRateHz(int rate) {
    return '$rate Hz';
  }

  @override
  String get settingsAudioSampleRateDeviceDefault => 'Device default';

  @override
  String settingsAudioSampleRateDeviceDefaultWithRate(int rate) {
    return 'Device default ($rate Hz)';
  }

  @override
  String get settingsAudioSampleRateLoading => 'Loading rates for this device…';

  @override
  String get settingsAudioResamplerQuality => 'Resampler quality';

  @override
  String get settingsAudioResamplerLow => 'Low';

  @override
  String get settingsAudioResamplerMedium => 'Medium';

  @override
  String get settingsAudioResamplerHigh => 'High';

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
  String get settingsAudioMasterBusDescription =>
      'Primary output device and channel routing.';

  @override
  String get settingsAudioPreviewBusTitle => 'Preview bus';

  @override
  String get settingsAudioPreviewBusDescription =>
      'Headphones / cue output when preview is enabled.';

  @override
  String get settingsAudioPreviewBus => 'Enable preview bus';

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
      'Loudness matching, sampler routing, and realtime stem fallback.';

  @override
  String get settingsMixerNormalizerTitle => 'Volume normalizer';

  @override
  String get settingsMixerNormalizerDescription =>
      'Match analyzed tracks to a target loudness.';

  @override
  String get settingsMixerVolumeNormalizer => 'Volume normalizer';

  @override
  String get settingsMixerTargetLufs => 'Target LUFS';

  @override
  String get settingsMixerSamplerRoutingTitle => 'Sampler routing';

  @override
  String get settingsMixerSamplerRoutingDescription =>
      'Where sampler hits enter the signal path, relative to the channel strip.';

  @override
  String get settingsMixerSamplerStripRoute => 'Sampler strip route';

  @override
  String get settingsMixerSamplerStripBefore => 'Before channel strip';

  @override
  String get settingsMixerSamplerStripAfter => 'After channel strip';

  @override
  String get settingsMixerRealtimeStemsTitle => 'Realtime Stems';

  @override
  String get settingsMixerRealtimeStemsDescription =>
      'Separates tracks without cached stems while they play. Tracks with offline stems always take priority.';

  @override
  String get settingsMixerRealtimeStemsNotRecommended => 'Not recommended.';

  @override
  String get settingsMixerRealtimeStemsWarning =>
      'Quality is lower and it uses a CPU core.';

  @override
  String get settingsWaveformDescription =>
      'RGB mixes low/mid/high into one color. Filtered stacks the three bands.';

  @override
  String get settingsWaveformDisplayTitle => 'Display';

  @override
  String get settingsWaveformDisplayDescription =>
      'RGB mixes low/mid/high into one color. Filtered stacks the three bands.';

  @override
  String get settingsWaveformDisplayMode => 'Display mode';

  @override
  String get settingsWaveformModeRgb => 'RGB';

  @override
  String get settingsWaveformModeFiltered => 'Filtered';

  @override
  String get settingsDeckDescription =>
      'Deck layout, plus default jog, tempo, and sampler behavior for new decks.';

  @override
  String get settingsDeckCount => 'Deck layout';

  @override
  String get settingsDeckLayoutDescription =>
      'How many decks the mixer shows. Changing it restarts the audio engine and reloads tracks.';

  @override
  String get settingsDeckLayoutDecks => 'Decks';

  @override
  String get settingsDeckCountTwo => '2 decks';

  @override
  String get settingsDeckCountFour => '4 decks';

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
  String get settingsDeckVinylBrakePause => 'Vinyl brake pause';

  @override
  String get settingsDeckVinylBrakePauseHint =>
      'Pause spins the deck down like a turntable instead of cutting instantly.';

  @override
  String get settingsDeckVinylBrakePauseToggle => 'Brake on pause';

  @override
  String get settingsDeckVinylBrakeStopTime => 'Stop time';

  @override
  String get settingsDeckVinylBrakeStopTimeHint =>
      'How long the platter takes to stop after pause.';

  @override
  String settingsStepperDecrease(String field) {
    return 'Decrease $field';
  }

  @override
  String settingsStepperIncrease(String field) {
    return 'Increase $field';
  }

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
  String get settingsDeckDefaultSamplerBankA => 'Deck A default sampler bank';

  @override
  String get settingsDeckDefaultSamplerBankB => 'Deck B default sampler bank';

  @override
  String get settingsDeckDefaultSamplerBankC => 'Deck C default sampler bank';

  @override
  String get settingsDeckDefaultSamplerBankD => 'Deck D default sampler bank';

  @override
  String get settingsDeckCountRestartTitle => 'Change deck layout?';

  @override
  String get settingsDeckCountRestartBody =>
      'Changing the deck count restarts the audio engine and reloads tracks. Tracks on decks removed by switching to 2 decks will not be restored.';

  @override
  String get settingsDeckCountRestartConfirm => 'Restart and apply';

  @override
  String get settingsLibraryListTitle => 'Library list';

  @override
  String get settingsLibraryListDescription =>
      'How tracks look in the library table.';

  @override
  String get settingsLibraryDescription =>
      'Track import, offline analysis, and list display.';

  @override
  String get settingsLibraryAnalysisTitle => 'Analysis';

  @override
  String get settingsLibraryAnalysisDescription =>
      'How thoroughly offline analysis scans each track.';

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
  String get settingsLibraryTagWriteBackTitle => 'File tags';

  @override
  String get settingsLibraryTagWriteBackDescription =>
      'Mixar keeps BPM and key in its own database. When enabled, it also writes them into each file\'s tags so other DJ software sees the same values. Off by default — Mixar never modifies your files while this is off.';

  @override
  String get settingsLibraryTagWriteBack => 'Write BPM and key to file tags';

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
  String get padStemsOffline => 'Offline';

  @override
  String get padStemsRealtime => 'Realtime';

  @override
  String get settingsLibraryStemsTitle => 'Stems';

  @override
  String get settingsLibraryStemsDescription =>
      'Model and codec used when writing separated stem caches.';

  @override
  String get settingsLibraryStemFormat => 'Stem format';

  @override
  String get settingsLibraryStemModel => 'Stem model';

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
  String get settingsStorageOverviewTitle => 'Mixar storage';

  @override
  String get settingsStorageOverviewDescription =>
      'Disk use for caches and library metadata under app support.';

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
  String get cdjExportTitle => 'Export to CDJ USB';

  @override
  String get cdjExportDescription =>
      'Write playlists and folder collections to a Pioneer CDJ/XDJ USB drive, so players browse them without Rekordbox. Each export adds to what the drive already holds.';

  @override
  String get cdjExportDestination => 'Destination';

  @override
  String get cdjExportDestinationHint => 'USB drive or folder';

  @override
  String get cdjExportDestinationMissing =>
      'Choose a destination drive or folder.';

  @override
  String get cdjExportBrowse => 'Browse…';

  @override
  String get cdjExportCollection => 'Collection';

  @override
  String get cdjExportOverride => 'Override files already on the drive';

  @override
  String get cdjExportConfirmAction => 'Export';

  @override
  String get cdjExporting => 'Exporting…';

  @override
  String get cdjExportFailed => 'CDJ USB export failed';

  @override
  String cdjExportFinishedWithErrors(int count) {
    return 'CDJ USB export finished with $count error(s)';
  }

  @override
  String cdjExportFinishedWithWarnings(int count) {
    return 'CDJ USB export finished with $count warning(s)';
  }

  @override
  String cdjExportExported(String parts) {
    return 'Exported $parts';
  }

  @override
  String get cdjExportNothing => 'Nothing to export';

  @override
  String cdjCountTracks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count tracks',
      one: '1 track',
    );
    return '$_temp0';
  }

  @override
  String cdjCountReused(int count) {
    return '$count reused';
  }

  @override
  String cdjCountPlaylists(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count playlists',
      one: '1 playlist',
    );
    return '$_temp0';
  }

  @override
  String cdjCountFolders(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count folders',
      one: '1 folder',
    );
    return '$_temp0';
  }

  @override
  String cdjCountCopied(String size) {
    return '$size copied';
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

  @override
  String get settingsAnalyticsTitle => 'Analytics';

  @override
  String get settingsAnalyticsSubtitle => 'Share anonymous app data.';

  @override
  String get settingsAnalyticsToggle => 'Send anonymous usage data';

  @override
  String get settingsLibraryVisibleFields => 'Visible fields';

  @override
  String get settingsLibraryFieldAlbum => 'Album';

  @override
  String get settingsLibraryFieldGenre => 'Genre';

  @override
  String get settingsLibraryFieldYear => 'Year';

  @override
  String get settingsLibraryFieldTrackNumber => 'Track #';

  @override
  String get settingsLibraryFieldDiscNumber => 'Disc #';

  @override
  String get settingsLibraryFieldComment => 'Comment';

  @override
  String get settingsLibraryFieldIsrc => 'ISRC';

  @override
  String get settingsLibraryFieldDuration => 'Length';

  @override
  String get settingsLibraryFieldRating => 'Rating';

  @override
  String get settingsLibraryFieldColor => 'Color label';

  @override
  String get settingsLibraryFieldBpm => 'BPM';

  @override
  String get settingsLibraryFieldKey => 'Key';
}
