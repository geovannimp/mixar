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
  String get settingsSearchHint => 'Search';

  @override
  String get settingsSearchClose => 'Close search';

  @override
  String get settingsSearchNoResults => 'No matching settings';

  @override
  String settingsSearchNoResultsFor(String query) {
    return 'No settings available for your \"$query\"';
  }

  @override
  String get settingsLoading => 'Loading settings…';

  @override
  String get settingsSave => 'Save';

  @override
  String get settingsSaveChanges => 'Save changes';

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
  String get settingsSectionLayout => 'Layout';

  @override
  String get settingsSectionLibrary => 'Library';

  @override
  String get settingsSectionAnalysis => 'Analysis';

  @override
  String get settingsAnalysisDescription =>
      'Offline analysis: how much audio is scanned and how the main cue is detected.';

  @override
  String get settingsAnalysisCueTitle => 'Cue detection';

  @override
  String get settingsAnalysisCueDescription =>
      'Analysis sets a track\'s cue to its first sound. A cue you set or import is never overwritten.';

  @override
  String get settingsAnalysisCueThreshold => 'Silence threshold';

  @override
  String get settingsAnalysisCueThresholdHint =>
      'Audio quieter than this counts as silence; the cue is the first frame above it.';

  @override
  String get settingsSectionIntegrations => 'Integrations';

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
  String get settingsLayoutDescription =>
      'Which panels the mixer shows, where the waveform sits, and how the library opens.';

  @override
  String get settingsLayoutPresetTitle => 'Presets';

  @override
  String get settingsLayoutPresetDescription =>
      'Start from a device, then show or hide individual panels.';

  @override
  String get settingsLayoutPreset => 'Preset';

  @override
  String get settingsLayoutPresetDesktop => 'Desktop';

  @override
  String get settingsLayoutPresetDesktopSubtitle =>
      'Waveform, mixer strip, FX bar, and the docked library.';

  @override
  String get settingsLayoutPresetTablet => 'Tablet';

  @override
  String get settingsLayoutPresetTabletSubtitle =>
      'Waveform, mixer strip, FX bar, and the library in a drawer.';

  @override
  String get settingsLayoutPresetMobile => 'Mobile';

  @override
  String get settingsLayoutPresetMobileSubtitle =>
      'Decks with the waveform between them and the library in a drawer. No mixer strip.';

  @override
  String get settingsLayoutPresetCustom => 'Custom';

  @override
  String get settingsLayoutPresetCustomSubtitle =>
      'No preset matches the visible panels below.';

  @override
  String get settingsLayoutMixerTitle => 'Mixer strip';

  @override
  String get settingsLayoutMixerDescription =>
      'Channels, faders, EQ, crossfader, and meters between the decks. Turning it off widens the decks into a single row, or hands the column to the centered waveform.';

  @override
  String get settingsLayoutMixerEnable => 'Show mixer strip';

  @override
  String get settingsLayoutShowCrossfader => 'Crossfader';

  @override
  String get settingsLayoutMixerMeters => 'Meters';

  @override
  String get settingsLayoutMixerMetersNone => 'None';

  @override
  String get settingsLayoutMixerMetersNoneSubtitle =>
      'No deck meters; the channel columns drop the meter lane.';

  @override
  String get settingsLayoutMixerMetersMono => 'Mono';

  @override
  String get settingsLayoutMixerMetersStereo => 'Stereo';

  @override
  String get settingsLayoutMixerMetersMonoSubtitle =>
      'One ladder per deck (max of left/right).';

  @override
  String get settingsLayoutMixerMetersStereoSubtitle =>
      'Separate left and right ladders per deck.';

  @override
  String get settingsLayoutFxBarTitle => 'FX bar';

  @override
  String get settingsLayoutFxBarDescription =>
      'The shared effect unit and master meter at the foot of the mixer pane. Master and headphone levels open from the meter.';

  @override
  String get settingsLayoutFxBarEnable => 'Show FX bar';

  @override
  String get settingsLayoutDeckCount => 'Deck layout';

  @override
  String get settingsLayoutDeckCountDescription =>
      'How many decks the mixer shows. Changing it restarts the audio engine and reloads tracks.';

  @override
  String get settingsLayoutDeckCountDecks => 'Decks';

  @override
  String get settingsLayoutDeckCountTwo => '2 decks';

  @override
  String get settingsLayoutDeckCountFour => '4 decks';

  @override
  String get settingsLayoutWaveformTitle => 'Waveform position';

  @override
  String get settingsLayoutWaveformDescription =>
      'Top keeps the waveform above the decks; bottom puts it between the decks and the library. The middle positions replace the center mixer with the lanes. Turn the switch off to hide the waveform.';

  @override
  String get settingsLayoutWaveformPosition => 'Position';

  @override
  String get settingsLayoutShowWaveform => 'Show waveform';

  @override
  String get settingsLayoutWaveformTop => 'Top';

  @override
  String get settingsLayoutWaveformMiddleHorizontal => 'Middle horizontal';

  @override
  String get settingsLayoutWaveformMiddleVertical => 'Middle vertical';

  @override
  String get settingsLayoutWaveformBottom => 'Bottom';

  @override
  String get settingsLayoutLibraryDrawerTitle => 'Library drawer';

  @override
  String get settingsLayoutLibraryDrawerDescription =>
      'Where the library opens. Drawer mode hides the docked library and opens it over the mixer.';

  @override
  String get settingsLayoutLibraryDrawerEnable => 'Library as a drawer';

  @override
  String get settingsLayoutLibraryDrawerPosition => 'Position';

  @override
  String get settingsLayoutLibraryDrawerLeft => 'Left';

  @override
  String get settingsLayoutLibraryDrawerRight => 'Right';

  @override
  String get settingsLayoutLibraryDrawerTop => 'Top';

  @override
  String get settingsLayoutLibraryDrawerBottom => 'Bottom';

  @override
  String get settingsUiDescription =>
      'Theme, language, hover tips, and control presentation.';

  @override
  String get settingsUiChromeTitle => 'Language and tooltips';

  @override
  String get settingsUiChromeDescription =>
      'Interface language and hover tips.';

  @override
  String get settingsUiThemeTitle => 'Theme';

  @override
  String get settingsUiThemeDescription => 'Brightness of the app window.';

  @override
  String get settingsUiTheme => 'Theme';

  @override
  String get settingsUiThemeLight => 'Light';

  @override
  String get settingsUiThemeDark => 'Dark';

  @override
  String get settingsUiAccentColor => 'Accent color';

  @override
  String get settingsUiAccentDescription =>
      'Color used by buttons, highlights, and deck chrome.';

  @override
  String get settingsUiAccentTeal => 'Teal';

  @override
  String get settingsUiAccentBlue => 'Blue';

  @override
  String get settingsUiAccentViolet => 'Violet';

  @override
  String get settingsUiAccentOrange => 'Orange';

  @override
  String get settingsUiAccentPink => 'Pink';

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
  String get settingsUiOnboardingTitle => 'First-run setup';

  @override
  String get settingsUiOnboardingDescription =>
      'Show the welcome steps again instead of the mixer. Saved with the rest of Settings.';

  @override
  String get settingsUiOnboardingReplay => 'Show setup again';

  @override
  String onboardingStepProgress(int index, int total) {
    return 'Step $index of $total';
  }

  @override
  String get onboardingSkip => 'Skip setup';

  @override
  String get onboardingBack => 'Back';

  @override
  String get onboardingNext => 'Continue';

  @override
  String get onboardingSaveFailedToast => 'Setup could not be saved';

  @override
  String get onboardingWelcomeTitle => 'Welcome to Mixar';

  @override
  String get onboardingWelcomeBody =>
      'Just a few steps to get Mixar in your style.';

  @override
  String get onboardingAppearanceTitle => 'Make it yours';

  @override
  String get onboardingAppearanceBody =>
      'Pick a theme and an accent color. Both repaint the app as you choose.';

  @override
  String get onboardingLibraryTitle => 'Bring your music';

  @override
  String get onboardingLibraryBody =>
      'Point Mixar at a folder of tracks, or import a library from another DJ app. Audio files are never moved.';

  @override
  String get onboardingLibraryChooseFolder => 'Choose music folder';

  @override
  String get onboardingLibraryFolderAdded => 'Folder added';

  @override
  String get onboardingLibraryFolderFailed => 'Could not add that folder';

  @override
  String get onboardingPrivacyTitle => 'Your data stays here';

  @override
  String get onboardingPrivacyBody =>
      'Off by default. Sends anonymous events and error reports; file paths are redacted and your music is never uploaded.';

  @override
  String get onboardingControllersTitle => 'Set up your controller';

  @override
  String get onboardingControllersTrusted =>
      'Trusted devices connect automatically next launch.';

  @override
  String get onboardingControllersMappings =>
      'Mappings live in Settings → Controllers.';

  @override
  String get onboardingKickerSetup => 'Setup';

  @override
  String get onboardingKickerAppearance => 'Appearance';

  @override
  String get onboardingKickerLibrary => 'Library';

  @override
  String get onboardingKickerPrivacy => 'Privacy';

  @override
  String get onboardingKickerControllers => 'Controllers';

  @override
  String get onboardingLibraryImportLabel => 'Or import an existing library';

  @override
  String get onboardingPrivacyOffTitle => 'Keep it private';

  @override
  String get onboardingPrivacyOffBody =>
      'No analytics. Nothing leaves this machine.';

  @override
  String get onboardingPrivacyOnTitle => 'Share anonymous usage';

  @override
  String get onboardingPrivacyOnBody =>
      'Anonymous events and error reports. No music, no file names.';

  @override
  String get onboardingLanguageFollowsOs => 'Follows the system language';

  @override
  String get onboardingLibraryFolderHint =>
      'Mixar reads the files where they are. Nothing is moved or copied.';

  @override
  String onboardingLibraryFolderReady(String name) {
    return 'Added “$name”. Pick another folder if you want more.';
  }

  @override
  String get onboardingLibraryBrowse => 'Browse';

  @override
  String get onboardingReadyTitle => 'You\'re all set';

  @override
  String get onboardingReadyBody =>
      'Save these choices and the mixer opens ready to play.';

  @override
  String get onboardingReadyFootnote =>
      'Nothing here is permanent — every choice lives in Settings.';

  @override
  String get onboardingStart => 'Start mixing';

  @override
  String get onboardingTipLoadTitle => 'Load your first track';

  @override
  String get onboardingTipLoadBody =>
      'Drag a file onto a deck, or double-click a row in the library.';

  @override
  String get onboardingTipAnalyzeTitle => 'Analysis runs itself';

  @override
  String get onboardingTipAnalyzeBody =>
      'Beat grid, waveform, key and BPM are written as soon as a track lands in the library.';

  @override
  String get onboardingTipSettingsTitle => 'Tune it later';

  @override
  String get onboardingTipSettingsBody =>
      'Audio, controllers, layout and waveform are all under Settings.';

  @override
  String get onboardingKickerReady => 'Ready';

  @override
  String onboardingControllersDetected(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count controllers found. Trust the ones you want to connect automatically.',
      one: 'One controller found. Trust it to connect automatically.',
    );
    return '$_temp0';
  }

  @override
  String onboardingControllersMapping(String name) {
    return 'Mapping: $name';
  }

  @override
  String get onboardingControllersTrust => 'Trust';

  @override
  String get onboardingControllersTrustedAction => 'Trusted';

  @override
  String get onboardingFloatDecks => 'Two decks';

  @override
  String get onboardingFloatWaveform => 'Waveform';

  @override
  String get onboardingFloatStems => 'Stems';

  @override
  String get onboardingFloatMixer => 'Mixer';

  @override
  String get onboardingFloatJog => 'Jog';

  @override
  String get onboardingFloatSync => 'Sync';

  @override
  String get onboardingFloatCue => 'Cue';

  @override
  String get onboardingFloatLoop => 'Loop';

  @override
  String get onboardingFloatFx => 'FX';

  @override
  String get onboardingFloatLibrary => 'Library';

  @override
  String get onboardingKickerLanguage => 'Language';

  @override
  String get onboardingLanguageTitle => 'Pick your language';

  @override
  String get onboardingLanguageBody =>
      'Mixar follows the system language unless you choose one. The whole app switches as soon as you pick.';

  @override
  String get onboardingFloatBpm => 'BPM';

  @override
  String get onboardingFloatKeys => 'Keys';

  @override
  String get onboardingFloatCues => 'Hot cues';

  @override
  String get onboardingFloatAnalyze => 'Analyze';

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
  String settingsAudioControllerOverride(String controllerName) {
    return 'Overridden by $controllerName mapping';
  }

  @override
  String get settingsAudioDevice => 'Device';

  @override
  String settingsAudioDeviceLowQuality(String device) {
    return '$device — low quality output';
  }

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
  String get settingsMixerCrossfaderTitle => 'Crossfader';

  @override
  String get settingsMixerCrossfaderDescription =>
      'The curve the crossfader follows, plus the two performance flags. Changing it restarts the audio engine.';

  @override
  String get settingsMixerCrossfaderCurve => 'Default curve';

  @override
  String get settingsMixerCrossfaderInvert => 'Invert (swap A and B)';

  @override
  String get settingsMixerCrossfaderDisable => 'Disable the crossfader';

  @override
  String get settingsCrossfaderCurveConstantPower => 'Constant power';

  @override
  String get settingsCrossfaderCurveLinear => 'Linear';

  @override
  String get settingsCrossfaderCurveFastCut => 'Fast cut';

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
  String get settingsMixerRealtimeStemsDeviceReady =>
      'Your device is fast enough for this feature, free to enable.';

  @override
  String get settingsMixerRealtimeStemsDeviceTooSlow =>
      'Your device is too slow for this feature, enabling it would not work.';

  @override
  String get settingsMixerRealtimeStemsRunBenchmark => 'Run benchmark';

  @override
  String get settingsMixerRealtimeStemsRerunBenchmark => 'Re-run benchmark';

  @override
  String get settingsMixerRealtimeStemsBenchmarking => 'Benchmarking…';

  @override
  String settingsMixerRealtimeStemsBenchmarkFailed(String error) {
    return 'Benchmark failed: $error';
  }

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
  String get settingsWaveformShapingTitle => 'EQ & gain shaping';

  @override
  String get settingsWaveformShapingDescription =>
      'Scale the lane bars with the channel gain and the deck EQ. Off keeps the waveform still while you move them.';

  @override
  String get settingsWaveformShapingEnable =>
      'Shape the waveform with EQ and gain';

  @override
  String get settingsDeckDescription =>
      'Default jog, tempo, brake, and sampler behavior for new decks.';

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
  String get settingsDeckEndWarning => 'Track end warning';

  @override
  String get settingsDeckEndWarningHint =>
      'Blink the waveform and track details once a playing track is this close to its end.';

  @override
  String get settingsDeckEndWarningWindow => 'Warn before the end';

  @override
  String get settingsDeckEndWarningToggle => 'Blink before the track ends';

  @override
  String get settingsDeckAutoCue => 'Auto cue';

  @override
  String get settingsDeckAutoCueHint =>
      'On load, move the playhead to the track\'s cue instead of the start of the file.';

  @override
  String get settingsDeckAutoCueToggle => 'Cue to the track cue on load';

  @override
  String settingsDeckEndWarningSeconds(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count seconds',
      one: '1 second',
    );
    return '$_temp0';
  }

  @override
  String get settingsDeckFaderStart => 'Fader start';

  @override
  String get settingsDeckFaderStartHint =>
      'Start or stop playback by moving the channel fader away from or back to the bottom.';

  @override
  String get settingsDeckFaderStartToggle => 'Fader start';

  @override
  String get settingsDeckFaderStartThreshold => 'Fader threshold';

  @override
  String get settingsDeckFaderStartThresholdHint =>
      'How far the fader must rise before it counts as open.';

  @override
  String settingsDeckFaderStartThresholdPercent(int percent) {
    return '$percent %';
  }

  @override
  String get settingsDeckFaderStartOnNonZero => 'When fader opens';

  @override
  String get settingsDeckFaderStartOnZero => 'When fader closes';

  @override
  String get settingsDeckFaderStartOnNonZeroPlay => 'Play';

  @override
  String get settingsDeckFaderStartOnNonZeroTogglePlay => 'Toggle play';

  @override
  String get settingsDeckFaderStartOnNonZeroNothing => 'Nothing';

  @override
  String get settingsDeckFaderStartOnZeroPause => 'Pause';

  @override
  String get settingsDeckFaderStartOnZeroPauseAndCue => 'Pause and back to cue';

  @override
  String get settingsDeckFaderStartOnZeroNothing => 'Nothing';

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
  String get settingsLayoutDeckCountRestartTitle => 'Change deck layout?';

  @override
  String get settingsLayoutDeckCountRestartBody =>
      'Changing the deck count restarts the audio engine and reloads tracks. Tracks on decks removed by switching to 2 decks will not be restored.';

  @override
  String get settingsLayoutDeckCountRestartConfirm => 'Restart and apply';

  @override
  String get settingsLibraryListTitle => 'Library list';

  @override
  String get settingsLibraryListDescription =>
      'How tracks look in the library table.';

  @override
  String get settingsLibraryDescription =>
      'Offline analysis, metadata, and list display.';

  @override
  String get settingsIntegrationsDescription =>
      'Read another DJ application\'s library and curve settings.';

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
  String get settingsControllersAudio => 'Audio';

  @override
  String settingsControllersAudioSemantics(String name) {
    return 'Always switch audio to $name';
  }

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
  String get mixxxImportDescription => 'Bring your Mixxx library into Mixar.';

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
  String get rekordboxImportTitle => 'Import from Rekordbox';

  @override
  String get rekordboxImportDescription =>
      'Bring your Rekordbox library into Mixar.';

  @override
  String rekordboxImportFound(String path) {
    return 'Found: $path';
  }

  @override
  String get rekordboxImportCheckFailed =>
      'Could not check for a Rekordbox library.';

  @override
  String get rekordboxImportLooking => 'Looking for a Rekordbox library…';

  @override
  String get rekordboxImportNotFound =>
      'No Rekordbox library found on this computer.';

  @override
  String get rekordboxImportButton => 'Import from Rekordbox library…';

  @override
  String get rekordboxImporting => 'Importing…';

  @override
  String get rekordboxImportConfirmTitle => 'Import from Rekordbox?';

  @override
  String rekordboxImportConfirmBody(
    String path,
    int trackCount,
    int missingFileCount,
    int playlistCount,
    int smartPlaylistCount,
    int folderCount,
    int hotCueCount,
    int loopCount,
  ) {
    String _temp0 = intl.Intl.pluralLogic(
      trackCount,
      locale: localeName,
      other: '$trackCount tracks',
      one: '1 track',
    );
    String _temp1 = intl.Intl.pluralLogic(
      missingFileCount,
      locale: localeName,
      other: '$missingFileCount missing',
      one: '1 missing',
    );
    String _temp2 = intl.Intl.pluralLogic(
      playlistCount,
      locale: localeName,
      other: '$playlistCount playlists',
      one: '1 playlist',
    );
    String _temp3 = intl.Intl.pluralLogic(
      smartPlaylistCount,
      locale: localeName,
      other: '$smartPlaylistCount smart',
      one: '1 smart',
    );
    String _temp4 = intl.Intl.pluralLogic(
      folderCount,
      locale: localeName,
      other: '$folderCount folders',
      one: '1 folder',
    );
    String _temp5 = intl.Intl.pluralLogic(
      hotCueCount,
      locale: localeName,
      other: '$hotCueCount hot cues',
      one: '1 hot cue',
    );
    String _temp6 = intl.Intl.pluralLogic(
      loopCount,
      locale: localeName,
      other: '$loopCount loops',
      one: '1 loop',
    );
    return 'This imports tracks, playlists, hot cues, and loops from:\n$path\n\n$_temp0 ($_temp1), $_temp2 ($_temp3), $_temp4, $_temp5, $_temp6.\n\nMissing files are imported as unavailable tracks.';
  }

  @override
  String get rekordboxImportReadFailed => 'Could not read Rekordbox library';

  @override
  String get rekordboxImportFailed => 'Rekordbox import failed';

  @override
  String rekordboxImportFinishedWithErrors(int count) {
    return 'Rekordbox import finished with $count error(s)';
  }

  @override
  String get rekordboxImportAlreadyImported =>
      'Rekordbox library already imported';

  @override
  String get rekordboxImportNothing => 'Nothing to import from Rekordbox';

  @override
  String rekordboxImportUpdated(String updated, String missing) {
    return 'Updated $updated from Rekordbox$missing';
  }

  @override
  String rekordboxImportImported(String parts, String missing) {
    return 'Imported $parts from Rekordbox$missing';
  }

  @override
  String rekordboxImportMissingSuffix(int count) {
    return ' ($count missing)';
  }

  @override
  String rekordboxCountTracks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count tracks',
      one: '1 track',
    );
    return '$_temp0';
  }

  @override
  String rekordboxCountPlaylists(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count playlists',
      one: '1 playlist',
    );
    return '$_temp0';
  }

  @override
  String rekordboxCountHotCues(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count hot cues',
      one: '1 hot cue',
    );
    return '$_temp0';
  }

  @override
  String rekordboxCountLoops(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count loops',
      one: '1 loop',
    );
    return '$_temp0';
  }

  @override
  String rekordboxCountUpdated(int count) {
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

  @override
  String onboardingLibraryNotFoundOne(String name) {
    return 'No $name library found on this computer. You can import one later in Settings → Integrations.';
  }

  @override
  String get onboardingLibraryNotFoundBoth =>
      'No Mixxx or Rekordbox library found on this computer. You can import one later in Settings → Integrations.';

  @override
  String get settingsSectionCurves => 'Curves';

  @override
  String get settingsCurvesDescription =>
      'Build custom crossfader curves and curve sets, then import or export them through the VirtualDJ adapter.';

  @override
  String get curvesCustomTitle => 'Custom curves';

  @override
  String get curvesCustomDescription =>
      'A curve maps fader position to gain. A/B curves edit deck A and deck B separately; scalar curves drive one value.';

  @override
  String get curvesSetsTitle => 'Curve sets';

  @override
  String get curvesSetsDescription =>
      'A set sweeps between two or more A/B curves as the morph knob moves.';

  @override
  String get curvesImportExportTitle => 'Import & export';

  @override
  String get curvesImportExportDescription =>
      'Move custom crossfader curves to and from VirtualDJ through the settings adapter.';

  @override
  String get curvesExportCurveLabel => 'Curve';

  @override
  String get curvesNew => 'New curve';

  @override
  String get curvesNewSet => 'New set';

  @override
  String get curvesEmpty => 'No custom curves yet.';

  @override
  String get curvesEmptySets => 'No curve sets yet.';

  @override
  String get curvesScalar => 'Scalar';

  @override
  String get curvesAb => 'A/B';

  @override
  String get curvesChannelA => 'Deck A';

  @override
  String get curvesChannelB => 'Deck B';

  @override
  String get curvesScalarCurve => 'Scalar curve';

  @override
  String get curvesSelectToEdit => 'Select a curve to edit.';

  @override
  String get curvesSelectSetToEdit => 'Select a curve set to edit.';

  @override
  String get curvesRename => 'Rename';

  @override
  String get curvesDelete => 'Delete';

  @override
  String get curvesAddPoint => 'Add point';

  @override
  String get curvesRemovePoint => 'Remove point';

  @override
  String get curvesPointX => 'Point X';

  @override
  String get curvesPointY => 'Point Y';

  @override
  String get curvesChartHint =>
      'Drag a handle to move it. Tap the chart to add a point.';

  @override
  String curvesChartSemantics(String name) {
    return 'Curve chart for $name';
  }

  @override
  String curvesMorphSemantics(String name) {
    return 'Morph preview for $name';
  }

  @override
  String curvesSegment(int index) {
    return 'Segment $index interpolation';
  }

  @override
  String get curvesSegmentLinear => 'Linear';

  @override
  String get curvesSegmentExponential => 'Exponential';

  @override
  String get curvesSegmentLogarithmic => 'Logarithmic';

  @override
  String get curvesSegmentCubicBezier => 'Cubic Bézier';

  @override
  String get curvesRate => 'Rate';

  @override
  String get curvesBezierC1X => 'Handle 1 X';

  @override
  String get curvesBezierC1Y => 'Handle 1 Y';

  @override
  String get curvesBezierC2X => 'Handle 2 X';

  @override
  String get curvesBezierC2Y => 'Handle 2 Y';

  @override
  String get curvesMembers => 'Members';

  @override
  String get curvesAddMember => 'Add member';

  @override
  String get curvesMoveUp => 'Move up';

  @override
  String get curvesMoveDown => 'Move down';

  @override
  String get curvesSetHint =>
      'The morph knob blends adjacent members in order; the first member is 0 and the last is 1.';

  @override
  String get curvesMorphPreview => 'Morph preview';

  @override
  String curvesMemberCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count members',
      one: '1 member',
    );
    return '$_temp0';
  }

  @override
  String get curvesCreateTitle => 'New curve';

  @override
  String get curvesCreateSetTitle => 'New curve set';

  @override
  String get curvesRenameTitle => 'Rename curve';

  @override
  String get curvesRenameSetTitle => 'Rename curve set';

  @override
  String get curvesName => 'Name';

  @override
  String get curvesId => 'Id';

  @override
  String get curvesIdHint =>
      'Used by controller mappings and settings. Lowercase letters, digits and underscores.';

  @override
  String get curvesControl => 'Control';

  @override
  String get curvesControlAb => 'A/B (crossfader)';

  @override
  String get curvesControlScalar => 'Scalar (channel, tempo, FX)';

  @override
  String get curvesCancel => 'Cancel';

  @override
  String get curvesCreate => 'Create';

  @override
  String get curvesSave => 'Save';

  @override
  String get curvesImport => 'Import VirtualDJ';

  @override
  String get curvesImporting => 'Importing…';

  @override
  String get curvesExport => 'Export to VirtualDJ';

  @override
  String get curvesExporting => 'Exporting…';

  @override
  String get curvesImportTitle => 'Import VirtualDJ curve';

  @override
  String get curvesImportHelp =>
      'Paste the two VirtualDJ crossfader options. A custom point script becomes an editable native A/B curve.';

  @override
  String get curvesImportCurveLabel => 'crossfaderCurve';

  @override
  String get curvesImportCurveHint =>
      'Preset name or amount, e.g. Smooth, Cut, 30%';

  @override
  String get curvesImportCustomLabel => 'crossfaderCustom';

  @override
  String get curvesImportCustomHint => '0=[1,0]/0.5=[0.5,0.5]/1=[0,1]';

  @override
  String get curvesExportTitle => 'VirtualDJ curve';

  @override
  String get curvesExportCustomLabel => 'crossfaderCustom';

  @override
  String get curvesCopy => 'Copy';

  @override
  String get curvesCopied => 'Copied';

  @override
  String get curvesClose => 'Close';

  @override
  String curvesImported(String name) {
    return 'Imported “$name”';
  }

  @override
  String get curvesInUseTitle => 'Curve is in use';

  @override
  String curvesInUseBody(String name) {
    return 'Remove “$name” from the curve sets that reference it first.';
  }

  @override
  String get curvesDefaultInUseTitle => 'Curve is the default';

  @override
  String curvesDefaultInUseBody(String name) {
    return 'Choose another default crossfader curve before deleting “$name”.';
  }

  @override
  String get curvesDeleteConfirmTitle => 'Delete curve?';

  @override
  String get curvesDeleteSetConfirmTitle => 'Delete curve set?';

  @override
  String curvesDeleteConfirmBody(String name) {
    return '“$name” will be removed from your library.';
  }
}
