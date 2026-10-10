import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_pt.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations? of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations);
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('pt'),
    Locale('pt', 'BR'),
  ];

  /// Settings page header title
  ///
  /// In en, this message translates to:
  /// **'SETTINGS'**
  String get settingsTitle;

  /// No description provided for @settingsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Saving restarts the engine automatically if it\'s running.'**
  String get settingsSubtitle;

  /// No description provided for @settingsLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading settings…'**
  String get settingsLoading;

  /// No description provided for @settingsCloseSemantics.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get settingsCloseSemantics;

  /// No description provided for @settingsSave.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get settingsSave;

  /// No description provided for @settingsSaving.
  ///
  /// In en, this message translates to:
  /// **'Saving…'**
  String get settingsSaving;

  /// No description provided for @settingsSavedToast.
  ///
  /// In en, this message translates to:
  /// **'Settings saved'**
  String get settingsSavedToast;

  /// No description provided for @settingsSavedNotAppliedToast.
  ///
  /// In en, this message translates to:
  /// **'Settings saved, but not applied'**
  String get settingsSavedNotAppliedToast;

  /// No description provided for @settingsSaveFailedToast.
  ///
  /// In en, this message translates to:
  /// **'Save failed'**
  String get settingsSaveFailedToast;

  /// No description provided for @settingsUnsavedTitle.
  ///
  /// In en, this message translates to:
  /// **'Unsaved settings'**
  String get settingsUnsavedTitle;

  /// No description provided for @settingsUnsavedBody.
  ///
  /// In en, this message translates to:
  /// **'Save changes before closing?'**
  String get settingsUnsavedBody;

  /// No description provided for @settingsCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get settingsCancel;

  /// No description provided for @settingsDiscard.
  ///
  /// In en, this message translates to:
  /// **'Discard'**
  String get settingsDiscard;

  /// No description provided for @commonLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading…'**
  String get commonLoading;

  /// No description provided for @commonSync.
  ///
  /// In en, this message translates to:
  /// **'Sync'**
  String get commonSync;

  /// No description provided for @commonClear.
  ///
  /// In en, this message translates to:
  /// **'Clear'**
  String get commonClear;

  /// No description provided for @commonUpdate.
  ///
  /// In en, this message translates to:
  /// **'Update'**
  String get commonUpdate;

  /// No description provided for @commonImport.
  ///
  /// In en, this message translates to:
  /// **'Import'**
  String get commonImport;

  /// No description provided for @commonNone.
  ///
  /// In en, this message translates to:
  /// **'None'**
  String get commonNone;

  /// No description provided for @settingsSectionAudio.
  ///
  /// In en, this message translates to:
  /// **'Audio'**
  String get settingsSectionAudio;

  /// No description provided for @settingsSectionMixer.
  ///
  /// In en, this message translates to:
  /// **'Mixer'**
  String get settingsSectionMixer;

  /// No description provided for @settingsSectionWaveform.
  ///
  /// In en, this message translates to:
  /// **'Waveform'**
  String get settingsSectionWaveform;

  /// No description provided for @settingsSectionDeck.
  ///
  /// In en, this message translates to:
  /// **'Deck'**
  String get settingsSectionDeck;

  /// No description provided for @settingsSectionUi.
  ///
  /// In en, this message translates to:
  /// **'UI'**
  String get settingsSectionUi;

  /// No description provided for @settingsSectionLibrary.
  ///
  /// In en, this message translates to:
  /// **'Library'**
  String get settingsSectionLibrary;

  /// No description provided for @settingsSectionStorage.
  ///
  /// In en, this message translates to:
  /// **'Storage'**
  String get settingsSectionStorage;

  /// No description provided for @settingsSectionSession.
  ///
  /// In en, this message translates to:
  /// **'Session'**
  String get settingsSectionSession;

  /// No description provided for @settingsSectionControllers.
  ///
  /// In en, this message translates to:
  /// **'Controllers'**
  String get settingsSectionControllers;

  /// No description provided for @settingsUiDescription.
  ///
  /// In en, this message translates to:
  /// **'Chrome and hover tips for the desktop app.'**
  String get settingsUiDescription;

  /// No description provided for @settingsUiLanguageLabel.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get settingsUiLanguageLabel;

  /// No description provided for @settingsLanguageSystemDefault.
  ///
  /// In en, this message translates to:
  /// **'System default'**
  String get settingsLanguageSystemDefault;

  /// No description provided for @settingsShowTooltips.
  ///
  /// In en, this message translates to:
  /// **'Show tooltips'**
  String get settingsShowTooltips;

  /// No description provided for @settingsAudioDescription.
  ///
  /// In en, this message translates to:
  /// **'Engine output and buses.'**
  String get settingsAudioDescription;

  /// No description provided for @settingsAudioBackend.
  ///
  /// In en, this message translates to:
  /// **'Backend'**
  String get settingsAudioBackend;

  /// No description provided for @settingsAudioLowLatency.
  ///
  /// In en, this message translates to:
  /// **'Low latency'**
  String get settingsAudioLowLatency;

  /// No description provided for @settingsAudioSampleRate.
  ///
  /// In en, this message translates to:
  /// **'Sample rate'**
  String get settingsAudioSampleRate;

  /// No description provided for @settingsAudioSampleRateLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading rates for the master output device…'**
  String get settingsAudioSampleRateLoading;

  /// No description provided for @settingsAudioResamplerQuality.
  ///
  /// In en, this message translates to:
  /// **'Resampler quality'**
  String get settingsAudioResamplerQuality;

  /// No description provided for @settingsAudioBufferSize.
  ///
  /// In en, this message translates to:
  /// **'Buffer size'**
  String get settingsAudioBufferSize;

  /// No description provided for @settingsAudioBufferSizeHint.
  ///
  /// In en, this message translates to:
  /// **'Must be a multiple of 64 frames (mixer graph chunk size).'**
  String get settingsAudioBufferSizeHint;

  /// No description provided for @settingsAudioBufferSizeSemantics.
  ///
  /// In en, this message translates to:
  /// **'{frames} frames'**
  String settingsAudioBufferSizeSemantics(int frames);

  /// No description provided for @settingsAudioMasterBus.
  ///
  /// In en, this message translates to:
  /// **'Master bus'**
  String get settingsAudioMasterBus;

  /// No description provided for @settingsAudioPreviewBus.
  ///
  /// In en, this message translates to:
  /// **'Preview bus (headphones / cue)'**
  String get settingsAudioPreviewBus;

  /// No description provided for @settingsAudioDevice.
  ///
  /// In en, this message translates to:
  /// **'Device'**
  String get settingsAudioDevice;

  /// No description provided for @settingsAudioChannelMode.
  ///
  /// In en, this message translates to:
  /// **'Channel mode'**
  String get settingsAudioChannelMode;

  /// No description provided for @settingsAudioStereoPair.
  ///
  /// In en, this message translates to:
  /// **'Stereo pair'**
  String get settingsAudioStereoPair;

  /// No description provided for @settingsAudioMonoFold.
  ///
  /// In en, this message translates to:
  /// **'Mono (fold L+R)'**
  String get settingsAudioMonoFold;

  /// No description provided for @settingsAudioLeftChannel.
  ///
  /// In en, this message translates to:
  /// **'Left channel'**
  String get settingsAudioLeftChannel;

  /// No description provided for @settingsAudioRightChannel.
  ///
  /// In en, this message translates to:
  /// **'Right channel'**
  String get settingsAudioRightChannel;

  /// No description provided for @settingsAudioLoadingDevices.
  ///
  /// In en, this message translates to:
  /// **'Loading devices…'**
  String get settingsAudioLoadingDevices;

  /// No description provided for @settingsMixerDescription.
  ///
  /// In en, this message translates to:
  /// **'Keep analyzed tracks near a consistent perceived loudness.'**
  String get settingsMixerDescription;

  /// No description provided for @settingsMixerVolumeNormalizer.
  ///
  /// In en, this message translates to:
  /// **'Volume normalizer'**
  String get settingsMixerVolumeNormalizer;

  /// No description provided for @settingsMixerTargetLufs.
  ///
  /// In en, this message translates to:
  /// **'Target LUFS'**
  String get settingsMixerTargetLufs;

  /// No description provided for @settingsWaveformDescription.
  ///
  /// In en, this message translates to:
  /// **'RGB mixes low/mid/high into one color. Filtered stacks the three bands.'**
  String get settingsWaveformDescription;

  /// No description provided for @settingsWaveformDisplayMode.
  ///
  /// In en, this message translates to:
  /// **'Display mode'**
  String get settingsWaveformDisplayMode;

  /// No description provided for @settingsWaveformModeRgb.
  ///
  /// In en, this message translates to:
  /// **'RGB'**
  String get settingsWaveformModeRgb;

  /// No description provided for @settingsWaveformModeFiltered.
  ///
  /// In en, this message translates to:
  /// **'Filtered'**
  String get settingsWaveformModeFiltered;

  /// No description provided for @settingsDeckDescription.
  ///
  /// In en, this message translates to:
  /// **'Default jog, tempo, and sampler behavior for new decks.'**
  String get settingsDeckDescription;

  /// No description provided for @settingsDeckJogTitle.
  ///
  /// In en, this message translates to:
  /// **'Jog wheel'**
  String get settingsDeckJogTitle;

  /// No description provided for @settingsDeckJogDescription.
  ///
  /// In en, this message translates to:
  /// **'Defaults for top (touch) and outer (freewheel) platter policy.'**
  String get settingsDeckJogDescription;

  /// No description provided for @settingsDeckTopJogMode.
  ///
  /// In en, this message translates to:
  /// **'Top jog mode'**
  String get settingsDeckTopJogMode;

  /// No description provided for @settingsDeckOuterJogMode.
  ///
  /// In en, this message translates to:
  /// **'Outer jog mode'**
  String get settingsDeckOuterJogMode;

  /// No description provided for @settingsDeckJogVinyl.
  ///
  /// In en, this message translates to:
  /// **'Vinyl (scratch)'**
  String get settingsDeckJogVinyl;

  /// No description provided for @settingsDeckJogPitchBend.
  ///
  /// In en, this message translates to:
  /// **'Pitch bend'**
  String get settingsDeckJogPitchBend;

  /// No description provided for @settingsDeckJogIgnore.
  ///
  /// In en, this message translates to:
  /// **'Ignore'**
  String get settingsDeckJogIgnore;

  /// No description provided for @settingsDeckTempoKeyTitle.
  ///
  /// In en, this message translates to:
  /// **'Tempo and Key'**
  String get settingsDeckTempoKeyTitle;

  /// No description provided for @settingsDeckTempoKeyDescription.
  ///
  /// In en, this message translates to:
  /// **'Default pitch-fader range and key lock for new decks.'**
  String get settingsDeckTempoKeyDescription;

  /// No description provided for @settingsDeckDefaultTempoRange.
  ///
  /// In en, this message translates to:
  /// **'Default tempo range'**
  String get settingsDeckDefaultTempoRange;

  /// No description provided for @settingsDeckDefaultKeyLock.
  ///
  /// In en, this message translates to:
  /// **'Default key lock'**
  String get settingsDeckDefaultKeyLock;

  /// No description provided for @settingsDeckDefaultKeyLockHint.
  ///
  /// In en, this message translates to:
  /// **'Tempo-only pitch when on (time-stretch). Off = vinyl tempo.'**
  String get settingsDeckDefaultKeyLockHint;

  /// No description provided for @settingsDeckKeyLock.
  ///
  /// In en, this message translates to:
  /// **'Key lock'**
  String get settingsDeckKeyLock;

  /// No description provided for @settingsDeckSamplerTitle.
  ///
  /// In en, this message translates to:
  /// **'Sampler'**
  String get settingsDeckSamplerTitle;

  /// No description provided for @settingsDeckSamplerDescription.
  ///
  /// In en, this message translates to:
  /// **'Default play mode for inherit banks and default bank per deck.'**
  String get settingsDeckSamplerDescription;

  /// No description provided for @settingsDeckSamplerPlayMode.
  ///
  /// In en, this message translates to:
  /// **'Sampler play mode'**
  String get settingsDeckSamplerPlayMode;

  /// No description provided for @settingsDeckSamplerPlayOneshot.
  ///
  /// In en, this message translates to:
  /// **'Oneshot'**
  String get settingsDeckSamplerPlayOneshot;

  /// No description provided for @settingsDeckSamplerPlayHold.
  ///
  /// In en, this message translates to:
  /// **'Hold'**
  String get settingsDeckSamplerPlayHold;

  /// No description provided for @settingsDeckSamplerPlayLoop.
  ///
  /// In en, this message translates to:
  /// **'Loop'**
  String get settingsDeckSamplerPlayLoop;

  /// No description provided for @settingsDeckSamplerStripRoute.
  ///
  /// In en, this message translates to:
  /// **'Sampler strip route'**
  String get settingsDeckSamplerStripRoute;

  /// No description provided for @settingsDeckSamplerStripBefore.
  ///
  /// In en, this message translates to:
  /// **'Before channel strip'**
  String get settingsDeckSamplerStripBefore;

  /// No description provided for @settingsDeckSamplerStripAfter.
  ///
  /// In en, this message translates to:
  /// **'After channel strip'**
  String get settingsDeckSamplerStripAfter;

  /// No description provided for @settingsDeckDefaultSamplerBankA.
  ///
  /// In en, this message translates to:
  /// **'Deck A default sampler bank'**
  String get settingsDeckDefaultSamplerBankA;

  /// No description provided for @settingsDeckDefaultSamplerBankB.
  ///
  /// In en, this message translates to:
  /// **'Deck B default sampler bank'**
  String get settingsDeckDefaultSamplerBankB;

  /// No description provided for @settingsLibraryDescription.
  ///
  /// In en, this message translates to:
  /// **'Track import, offline analysis, and list display.'**
  String get settingsLibraryDescription;

  /// No description provided for @settingsLibraryAnalysisQuality.
  ///
  /// In en, this message translates to:
  /// **'Analysis quality'**
  String get settingsLibraryAnalysisQuality;

  /// No description provided for @settingsLibraryAnalysisFast.
  ///
  /// In en, this message translates to:
  /// **'Fast'**
  String get settingsLibraryAnalysisFast;

  /// No description provided for @settingsLibraryAnalysisFastSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Analyze a short preview for quick library scans.'**
  String get settingsLibraryAnalysisFastSubtitle;

  /// No description provided for @settingsLibraryAnalysisPrecise.
  ///
  /// In en, this message translates to:
  /// **'Precise'**
  String get settingsLibraryAnalysisPrecise;

  /// No description provided for @settingsLibraryAnalysisPreciseSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Balanced analysis for most libraries.'**
  String get settingsLibraryAnalysisPreciseSubtitle;

  /// No description provided for @settingsLibraryAnalysisComplete.
  ///
  /// In en, this message translates to:
  /// **'Complete'**
  String get settingsLibraryAnalysisComplete;

  /// No description provided for @settingsLibraryAnalysisCompleteSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Analyze the full track (slowest, most accurate).'**
  String get settingsLibraryAnalysisCompleteSubtitle;

  /// No description provided for @settingsLibraryMusicalKeyTitle.
  ///
  /// In en, this message translates to:
  /// **'Musical key'**
  String get settingsLibraryMusicalKeyTitle;

  /// No description provided for @settingsLibraryMusicalKeyDescription.
  ///
  /// In en, this message translates to:
  /// **'How keys are labeled and color-coded in deck chrome and the library table.'**
  String get settingsLibraryMusicalKeyDescription;

  /// No description provided for @settingsLibraryKeyDisplayMode.
  ///
  /// In en, this message translates to:
  /// **'Key display mode'**
  String get settingsLibraryKeyDisplayMode;

  /// No description provided for @settingsLibraryKeyDisplayMusical.
  ///
  /// In en, this message translates to:
  /// **'Musical'**
  String get settingsLibraryKeyDisplayMusical;

  /// No description provided for @settingsLibraryKeyDisplayMusicalSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Note names in deck chip and library key column — e.g. C, Am, F#m.'**
  String get settingsLibraryKeyDisplayMusicalSubtitle;

  /// No description provided for @settingsLibraryKeyDisplayCamelot.
  ///
  /// In en, this message translates to:
  /// **'Camelot'**
  String get settingsLibraryKeyDisplayCamelot;

  /// No description provided for @settingsLibraryKeyDisplayCamelotSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Mixed In Key codes — e.g. 8B (C major), 8A (A minor), 11B.'**
  String get settingsLibraryKeyDisplayCamelotSubtitle;

  /// No description provided for @settingsLibraryKeyColorMode.
  ///
  /// In en, this message translates to:
  /// **'Key color mode'**
  String get settingsLibraryKeyColorMode;

  /// No description provided for @settingsLibraryKeyColorOff.
  ///
  /// In en, this message translates to:
  /// **'Off'**
  String get settingsLibraryKeyColorOff;

  /// No description provided for @settingsLibraryKeyColorOffSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Key labels use the default text color everywhere.'**
  String get settingsLibraryKeyColorOffSubtitle;

  /// No description provided for @settingsLibraryKeyColorAbsolute.
  ///
  /// In en, this message translates to:
  /// **'Absolute (circle of fifths)'**
  String get settingsLibraryKeyColorAbsolute;

  /// No description provided for @settingsLibraryKeyColorAbsoluteSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Fixed color per key on the wheel — majors vivid, minors muted (e.g. 8B bright, 8A softer).'**
  String get settingsLibraryKeyColorAbsoluteSubtitle;

  /// No description provided for @settingsLibraryKeyColorHarmonic.
  ///
  /// In en, this message translates to:
  /// **'Harmonic (playing deck)'**
  String get settingsLibraryKeyColorHarmonic;

  /// No description provided for @settingsLibraryKeyColorHarmonicSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Green/yellow vs the playing deck — e.g. with 2A playing, 1A/2A/3A/2B green, 1B/3B yellow.'**
  String get settingsLibraryKeyColorHarmonicSubtitle;

  /// No description provided for @settingsLibraryStemFormat.
  ///
  /// In en, this message translates to:
  /// **'Stem format'**
  String get settingsLibraryStemFormat;

  /// No description provided for @settingsLibraryStemOpus.
  ///
  /// In en, this message translates to:
  /// **'Opus (160 kbps)'**
  String get settingsLibraryStemOpus;

  /// No description provided for @settingsLibraryStemFlac.
  ///
  /// In en, this message translates to:
  /// **'FLAC (lossless)'**
  String get settingsLibraryStemFlac;

  /// No description provided for @settingsLibraryDimPlayedTracks.
  ///
  /// In en, this message translates to:
  /// **'Dim played tracks'**
  String get settingsLibraryDimPlayedTracks;

  /// No description provided for @settingsLibraryTrackRowLayout.
  ///
  /// In en, this message translates to:
  /// **'Track row layout'**
  String get settingsLibraryTrackRowLayout;

  /// No description provided for @settingsLibraryRowCompact.
  ///
  /// In en, this message translates to:
  /// **'Compact'**
  String get settingsLibraryRowCompact;

  /// No description provided for @settingsLibraryRowCompactSubtitle.
  ///
  /// In en, this message translates to:
  /// **'One dense line per track — fits the most rows on screen.'**
  String get settingsLibraryRowCompactSubtitle;

  /// No description provided for @settingsLibraryRowComfortable.
  ///
  /// In en, this message translates to:
  /// **'Comfortable'**
  String get settingsLibraryRowComfortable;

  /// No description provided for @settingsLibraryRowComfortableSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Two lines per track: title above artist, BPM, key and length.'**
  String get settingsLibraryRowComfortableSubtitle;

  /// No description provided for @settingsSessionDescription.
  ///
  /// In en, this message translates to:
  /// **'Performance history and session boundaries.'**
  String get settingsSessionDescription;

  /// No description provided for @settingsSessionHistoryTitle.
  ///
  /// In en, this message translates to:
  /// **'Performance history'**
  String get settingsSessionHistoryTitle;

  /// No description provided for @settingsSessionHistoryDescription.
  ///
  /// In en, this message translates to:
  /// **'Log deck playback to XSPF session files under app support.'**
  String get settingsSessionHistoryDescription;

  /// No description provided for @settingsSessionRecordHistory.
  ///
  /// In en, this message translates to:
  /// **'Record performance history'**
  String get settingsSessionRecordHistory;

  /// No description provided for @settingsSessionIdleTimeout.
  ///
  /// In en, this message translates to:
  /// **'Session idle timeout'**
  String get settingsSessionIdleTimeout;

  /// No description provided for @settingsSessionIdleTimeoutHint.
  ///
  /// In en, this message translates to:
  /// **'Close after this long with no qualifying deck output.'**
  String get settingsSessionIdleTimeoutHint;

  /// No description provided for @settingsSessionMinPlayDuration.
  ///
  /// In en, this message translates to:
  /// **'Minimum play duration'**
  String get settingsSessionMinPlayDuration;

  /// No description provided for @settingsSessionMinPlayDurationHint.
  ///
  /// In en, this message translates to:
  /// **'Commit entries after this much qualifying playback.'**
  String get settingsSessionMinPlayDurationHint;

  /// No description provided for @settingsSessionMinutes.
  ///
  /// In en, this message translates to:
  /// **'minutes'**
  String get settingsSessionMinutes;

  /// No description provided for @settingsSessionSeconds.
  ///
  /// In en, this message translates to:
  /// **'seconds'**
  String get settingsSessionSeconds;

  /// No description provided for @settingsSessionMinDeckVolume.
  ///
  /// In en, this message translates to:
  /// **'Minimum effective deck volume'**
  String get settingsSessionMinDeckVolume;

  /// No description provided for @settingsSessionPercentSemantics.
  ///
  /// In en, this message translates to:
  /// **'{percent} percent'**
  String settingsSessionPercentSemantics(int percent);

  /// No description provided for @settingsStorageDescription.
  ///
  /// In en, this message translates to:
  /// **'Disk use for Mixar caches and library metadata under app support.'**
  String get settingsStorageDescription;

  /// No description provided for @settingsStorageMixarStorage.
  ///
  /// In en, this message translates to:
  /// **'Mixar storage'**
  String get settingsStorageMixarStorage;

  /// No description provided for @settingsStorageUsed.
  ///
  /// In en, this message translates to:
  /// **'{size} used'**
  String settingsStorageUsed(String size);

  /// No description provided for @settingsStorageStemCache.
  ///
  /// In en, this message translates to:
  /// **'Stem cache'**
  String get settingsStorageStemCache;

  /// No description provided for @settingsStorageStemModel.
  ///
  /// In en, this message translates to:
  /// **'Stem model'**
  String get settingsStorageStemModel;

  /// No description provided for @settingsStorageWaveform.
  ///
  /// In en, this message translates to:
  /// **'Waveform'**
  String get settingsStorageWaveform;

  /// No description provided for @settingsStorageTrackMetadata.
  ///
  /// In en, this message translates to:
  /// **'Track metadata'**
  String get settingsStorageTrackMetadata;

  /// No description provided for @settingsStorageSyncStemTitle.
  ///
  /// In en, this message translates to:
  /// **'Sync stem cache?'**
  String get settingsStorageSyncStemTitle;

  /// No description provided for @settingsStorageSyncStemBody.
  ///
  /// In en, this message translates to:
  /// **'Removes stem files that are not listed in the library database, and drops DB rows whose files are missing. Referenced cache files and original audio are kept.'**
  String get settingsStorageSyncStemBody;

  /// No description provided for @settingsStorageSyncFailed.
  ///
  /// In en, this message translates to:
  /// **'Sync failed'**
  String get settingsStorageSyncFailed;

  /// No description provided for @settingsStorageClearFailed.
  ///
  /// In en, this message translates to:
  /// **'Clear failed'**
  String get settingsStorageClearFailed;

  /// No description provided for @settingsStorageClearStemTitle.
  ///
  /// In en, this message translates to:
  /// **'Clear stem cache?'**
  String get settingsStorageClearStemTitle;

  /// No description provided for @settingsStorageClearStemBody.
  ///
  /// In en, this message translates to:
  /// **'Deletes generated stem files for all tracks. Original library audio is not touched.'**
  String get settingsStorageClearStemBody;

  /// No description provided for @settingsStorageClearStemOk.
  ///
  /// In en, this message translates to:
  /// **'Stem cache cleared'**
  String get settingsStorageClearStemOk;

  /// No description provided for @settingsStorageClearModelTitle.
  ///
  /// In en, this message translates to:
  /// **'Clear stem model cache?'**
  String get settingsStorageClearModelTitle;

  /// No description provided for @settingsStorageClearModelBody.
  ///
  /// In en, this message translates to:
  /// **'Deletes downloaded stem separation models. They will re-download when needed.'**
  String get settingsStorageClearModelBody;

  /// No description provided for @settingsStorageClearModelOk.
  ///
  /// In en, this message translates to:
  /// **'Stem model cleared'**
  String get settingsStorageClearModelOk;

  /// No description provided for @settingsStorageClearWaveformTitle.
  ///
  /// In en, this message translates to:
  /// **'Clear waveform cache?'**
  String get settingsStorageClearWaveformTitle;

  /// No description provided for @settingsStorageClearWaveformBody.
  ///
  /// In en, this message translates to:
  /// **'Deletes cached waveform overviews. They regenerate when you open a track.'**
  String get settingsStorageClearWaveformBody;

  /// No description provided for @settingsStorageClearWaveformOk.
  ///
  /// In en, this message translates to:
  /// **'Waveform cache cleared'**
  String get settingsStorageClearWaveformOk;

  /// No description provided for @settingsControllersDescription.
  ///
  /// In en, this message translates to:
  /// **'MIDI mappings and connected hardware. Trust a device to auto-enable it on connect after Save.'**
  String get settingsControllersDescription;

  /// No description provided for @settingsControllersMappingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Mappings'**
  String get settingsControllersMappingsTitle;

  /// No description provided for @settingsControllersMappingsDescription.
  ///
  /// In en, this message translates to:
  /// **'Stored in app data. Seed copies shipped maps when missing; Update overwrites from the app bundle.'**
  String get settingsControllersMappingsDescription;

  /// No description provided for @settingsControllersUpdateAll.
  ///
  /// In en, this message translates to:
  /// **'Update All'**
  String get settingsControllersUpdateAll;

  /// No description provided for @settingsControllersNoMappings.
  ///
  /// In en, this message translates to:
  /// **'No mappings in app data yet.'**
  String get settingsControllersNoMappings;

  /// No description provided for @settingsControllersMidiPortsTitle.
  ///
  /// In en, this message translates to:
  /// **'MIDI ports'**
  String get settingsControllersMidiPortsTitle;

  /// No description provided for @settingsControllersMidiPortsDescription.
  ///
  /// In en, this message translates to:
  /// **'Detected MIDI inputs and outputs, and the mapping each one matches.'**
  String get settingsControllersMidiPortsDescription;

  /// No description provided for @settingsControllersNoMidiPorts.
  ///
  /// In en, this message translates to:
  /// **'No MIDI ports detected.'**
  String get settingsControllersNoMidiPorts;

  /// No description provided for @settingsControllersNoMapping.
  ///
  /// In en, this message translates to:
  /// **'No mapping'**
  String get settingsControllersNoMapping;

  /// No description provided for @settingsControllersMappingArrow.
  ///
  /// In en, this message translates to:
  /// **'→ {mapping}'**
  String settingsControllersMappingArrow(String mapping);

  /// No description provided for @settingsControllersDirectionIn.
  ///
  /// In en, this message translates to:
  /// **'IN'**
  String get settingsControllersDirectionIn;

  /// No description provided for @settingsControllersDirectionOut.
  ///
  /// In en, this message translates to:
  /// **'OUT'**
  String get settingsControllersDirectionOut;

  /// No description provided for @settingsControllersAttached.
  ///
  /// In en, this message translates to:
  /// **'Attached'**
  String get settingsControllersAttached;

  /// No description provided for @settingsControllersUpdateAvailable.
  ///
  /// In en, this message translates to:
  /// **'Update available'**
  String get settingsControllersUpdateAvailable;

  /// No description provided for @settingsControllersTrust.
  ///
  /// In en, this message translates to:
  /// **'Trust'**
  String get settingsControllersTrust;

  /// No description provided for @settingsControllersAttach.
  ///
  /// In en, this message translates to:
  /// **'Attach'**
  String get settingsControllersAttach;

  /// No description provided for @settingsControllersTrustSemantics.
  ///
  /// In en, this message translates to:
  /// **'Trust device {name}'**
  String settingsControllersTrustSemantics(String name);

  /// No description provided for @settingsControllersEnableSemantics.
  ///
  /// In en, this message translates to:
  /// **'Enable {name}'**
  String settingsControllersEnableSemantics(String name);

  /// No description provided for @mixxxImportTitle.
  ///
  /// In en, this message translates to:
  /// **'Import from Mixxx'**
  String get mixxxImportTitle;

  /// No description provided for @mixxxImportDescription.
  ///
  /// In en, this message translates to:
  /// **'Bring tracks, playlists, and crates from your Mixxx library into Mixar.'**
  String get mixxxImportDescription;

  /// No description provided for @mixxxImportFound.
  ///
  /// In en, this message translates to:
  /// **'Found: {path}'**
  String mixxxImportFound(String path);

  /// No description provided for @mixxxImportCheckFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not check for a Mixxx library.'**
  String get mixxxImportCheckFailed;

  /// No description provided for @mixxxImportLooking.
  ///
  /// In en, this message translates to:
  /// **'Looking for a Mixxx library…'**
  String get mixxxImportLooking;

  /// No description provided for @mixxxImportNotFound.
  ///
  /// In en, this message translates to:
  /// **'No Mixxx library found on this computer.'**
  String get mixxxImportNotFound;

  /// No description provided for @mixxxImportButton.
  ///
  /// In en, this message translates to:
  /// **'Import from Mixxx library…'**
  String get mixxxImportButton;

  /// No description provided for @mixxxImporting.
  ///
  /// In en, this message translates to:
  /// **'Importing…'**
  String get mixxxImporting;

  /// No description provided for @mixxxImportConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Import from Mixxx?'**
  String get mixxxImportConfirmTitle;

  /// No description provided for @mixxxImportConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'This imports tracks, playlists, crates, and watched folders from:\n{path}\n\n{trackCount} tracks ({missingFileCount} missing), {playlistCount} playlists, {crateCount} crates, {folderCount} folders.\n\nMissing files are imported as unavailable tracks.'**
  String mixxxImportConfirmBody(
    String path,
    int trackCount,
    int missingFileCount,
    int playlistCount,
    int crateCount,
    int folderCount,
  );

  /// No description provided for @mixxxImportReadFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not read Mixxx library'**
  String get mixxxImportReadFailed;

  /// No description provided for @mixxxImportFailed.
  ///
  /// In en, this message translates to:
  /// **'Mixxx import failed'**
  String get mixxxImportFailed;

  /// No description provided for @mixxxImportFinishedWithErrors.
  ///
  /// In en, this message translates to:
  /// **'Mixxx import finished with {count} error(s)'**
  String mixxxImportFinishedWithErrors(int count);

  /// No description provided for @mixxxImportAlreadyImported.
  ///
  /// In en, this message translates to:
  /// **'Mixxx library already imported'**
  String get mixxxImportAlreadyImported;

  /// No description provided for @mixxxImportNothing.
  ///
  /// In en, this message translates to:
  /// **'Nothing to import from Mixxx'**
  String get mixxxImportNothing;

  /// No description provided for @mixxxImportUpdated.
  ///
  /// In en, this message translates to:
  /// **'Updated {updated} from Mixxx{missing}'**
  String mixxxImportUpdated(String updated, String missing);

  /// No description provided for @mixxxImportImported.
  ///
  /// In en, this message translates to:
  /// **'Imported {parts} from Mixxx{missing}'**
  String mixxxImportImported(String parts, String missing);

  /// No description provided for @mixxxImportMissingSuffix.
  ///
  /// In en, this message translates to:
  /// **' ({count} missing)'**
  String mixxxImportMissingSuffix(int count);

  /// No description provided for @mixxxCountTracks.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 track} other{{count} tracks}}'**
  String mixxxCountTracks(int count);

  /// No description provided for @mixxxCountPlaylists.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 playlist} other{{count} playlists}}'**
  String mixxxCountPlaylists(int count);

  /// No description provided for @mixxxCountCrates.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 crate} other{{count} crates}}'**
  String mixxxCountCrates(int count);

  /// No description provided for @mixxxCountFolders.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 folder} other{{count} folders}}'**
  String mixxxCountFolders(int count);

  /// No description provided for @mixxxCountUpdated.
  ///
  /// In en, this message translates to:
  /// **'{count} updated'**
  String mixxxCountUpdated(int count);

  /// No description provided for @settingsStorageStemAlreadyInSync.
  ///
  /// In en, this message translates to:
  /// **'Stem cache already in sync'**
  String get settingsStorageStemAlreadyInSync;

  /// No description provided for @settingsStorageRemovedOrphans.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Removed 1 orphan stem file} other{Removed {count} orphan stem files}}'**
  String settingsStorageRemovedOrphans(int count);
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'pt'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when language+country codes are specified.
  switch (locale.languageCode) {
    case 'pt':
      {
        switch (locale.countryCode) {
          case 'BR':
            return AppLocalizationsPtBr();
        }
        break;
      }
  }

  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'pt':
      return AppLocalizationsPt();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
