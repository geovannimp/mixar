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

  /// No description provided for @settingsSectionLayout.
  ///
  /// In en, this message translates to:
  /// **'Layout'**
  String get settingsSectionLayout;

  /// No description provided for @settingsSectionLibrary.
  ///
  /// In en, this message translates to:
  /// **'Library'**
  String get settingsSectionLibrary;

  /// No description provided for @settingsSectionIntegrations.
  ///
  /// In en, this message translates to:
  /// **'Integrations'**
  String get settingsSectionIntegrations;

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

  /// No description provided for @settingsSectionPrivacy.
  ///
  /// In en, this message translates to:
  /// **'Privacy'**
  String get settingsSectionPrivacy;

  /// No description provided for @settingsPrivacyDescription.
  ///
  /// In en, this message translates to:
  /// **'Anonymous app usage data.'**
  String get settingsPrivacyDescription;

  /// No description provided for @settingsLayoutDescription.
  ///
  /// In en, this message translates to:
  /// **'Which panels the mixer shows, where the waveform sits, and how the library opens.'**
  String get settingsLayoutDescription;

  /// No description provided for @settingsLayoutPresetTitle.
  ///
  /// In en, this message translates to:
  /// **'Presets'**
  String get settingsLayoutPresetTitle;

  /// No description provided for @settingsLayoutPresetDescription.
  ///
  /// In en, this message translates to:
  /// **'Start from a device, then show or hide individual panels.'**
  String get settingsLayoutPresetDescription;

  /// No description provided for @settingsLayoutPreset.
  ///
  /// In en, this message translates to:
  /// **'Preset'**
  String get settingsLayoutPreset;

  /// No description provided for @settingsLayoutPresetDesktop.
  ///
  /// In en, this message translates to:
  /// **'Desktop'**
  String get settingsLayoutPresetDesktop;

  /// No description provided for @settingsLayoutPresetDesktopSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Waveform, mixer strip, FX bar, and the docked library.'**
  String get settingsLayoutPresetDesktopSubtitle;

  /// No description provided for @settingsLayoutPresetTablet.
  ///
  /// In en, this message translates to:
  /// **'Tablet'**
  String get settingsLayoutPresetTablet;

  /// No description provided for @settingsLayoutPresetTabletSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Waveform, mixer strip, FX bar, and the library in a drawer.'**
  String get settingsLayoutPresetTabletSubtitle;

  /// No description provided for @settingsLayoutPresetMobile.
  ///
  /// In en, this message translates to:
  /// **'Mobile'**
  String get settingsLayoutPresetMobile;

  /// No description provided for @settingsLayoutPresetMobileSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Decks with the waveform between them and the library in a drawer. No mixer strip.'**
  String get settingsLayoutPresetMobileSubtitle;

  /// No description provided for @settingsLayoutPresetCustom.
  ///
  /// In en, this message translates to:
  /// **'Custom'**
  String get settingsLayoutPresetCustom;

  /// No description provided for @settingsLayoutPresetCustomSubtitle.
  ///
  /// In en, this message translates to:
  /// **'No preset matches the visible panels below.'**
  String get settingsLayoutPresetCustomSubtitle;

  /// No description provided for @settingsLayoutMixerTitle.
  ///
  /// In en, this message translates to:
  /// **'Mixer strip'**
  String get settingsLayoutMixerTitle;

  /// No description provided for @settingsLayoutMixerDescription.
  ///
  /// In en, this message translates to:
  /// **'Channels, faders, EQ, crossfader, and meters between the decks. Turning it off widens the decks into a single row, or hands the column to the centered waveform.'**
  String get settingsLayoutMixerDescription;

  /// No description provided for @settingsLayoutMixerEnable.
  ///
  /// In en, this message translates to:
  /// **'Show mixer strip'**
  String get settingsLayoutMixerEnable;

  /// No description provided for @settingsLayoutShowCrossfader.
  ///
  /// In en, this message translates to:
  /// **'Crossfader'**
  String get settingsLayoutShowCrossfader;

  /// No description provided for @settingsLayoutMixerMeters.
  ///
  /// In en, this message translates to:
  /// **'Mixer meters'**
  String get settingsLayoutMixerMeters;

  /// No description provided for @settingsLayoutMixerMetersNone.
  ///
  /// In en, this message translates to:
  /// **'None'**
  String get settingsLayoutMixerMetersNone;

  /// No description provided for @settingsLayoutMixerMetersNoneSubtitle.
  ///
  /// In en, this message translates to:
  /// **'No deck meters; the channel columns drop the meter lane.'**
  String get settingsLayoutMixerMetersNoneSubtitle;

  /// No description provided for @settingsLayoutMixerMetersMono.
  ///
  /// In en, this message translates to:
  /// **'Mono'**
  String get settingsLayoutMixerMetersMono;

  /// No description provided for @settingsLayoutMixerMetersStereo.
  ///
  /// In en, this message translates to:
  /// **'Stereo'**
  String get settingsLayoutMixerMetersStereo;

  /// No description provided for @settingsLayoutMixerMetersMonoSubtitle.
  ///
  /// In en, this message translates to:
  /// **'One ladder per deck (max of left/right).'**
  String get settingsLayoutMixerMetersMonoSubtitle;

  /// No description provided for @settingsLayoutMixerMetersStereoSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Separate left and right ladders per deck.'**
  String get settingsLayoutMixerMetersStereoSubtitle;

  /// No description provided for @settingsLayoutFxBarTitle.
  ///
  /// In en, this message translates to:
  /// **'FX bar'**
  String get settingsLayoutFxBarTitle;

  /// No description provided for @settingsLayoutFxBarDescription.
  ///
  /// In en, this message translates to:
  /// **'The shared effect unit and master meter at the foot of the mixer pane. Master and headphone levels open from the meter.'**
  String get settingsLayoutFxBarDescription;

  /// No description provided for @settingsLayoutFxBarEnable.
  ///
  /// In en, this message translates to:
  /// **'Show FX bar'**
  String get settingsLayoutFxBarEnable;

  /// No description provided for @settingsLayoutDeckCount.
  ///
  /// In en, this message translates to:
  /// **'Deck layout'**
  String get settingsLayoutDeckCount;

  /// No description provided for @settingsLayoutDeckCountDescription.
  ///
  /// In en, this message translates to:
  /// **'How many decks the mixer shows. Changing it restarts the audio engine and reloads tracks.'**
  String get settingsLayoutDeckCountDescription;

  /// No description provided for @settingsLayoutDeckCountDecks.
  ///
  /// In en, this message translates to:
  /// **'Decks'**
  String get settingsLayoutDeckCountDecks;

  /// No description provided for @settingsLayoutDeckCountTwo.
  ///
  /// In en, this message translates to:
  /// **'2 decks'**
  String get settingsLayoutDeckCountTwo;

  /// No description provided for @settingsLayoutDeckCountFour.
  ///
  /// In en, this message translates to:
  /// **'4 decks'**
  String get settingsLayoutDeckCountFour;

  /// No description provided for @settingsLayoutWaveformTitle.
  ///
  /// In en, this message translates to:
  /// **'Waveform position'**
  String get settingsLayoutWaveformTitle;

  /// No description provided for @settingsLayoutWaveformDescription.
  ///
  /// In en, this message translates to:
  /// **'Top keeps the waveform above the decks; bottom puts it between the decks and the library. The middle positions replace the center mixer with the lanes. Turn the switch off to hide the waveform.'**
  String get settingsLayoutWaveformDescription;

  /// No description provided for @settingsLayoutWaveformPosition.
  ///
  /// In en, this message translates to:
  /// **'Position'**
  String get settingsLayoutWaveformPosition;

  /// No description provided for @settingsLayoutShowWaveform.
  ///
  /// In en, this message translates to:
  /// **'Show waveform'**
  String get settingsLayoutShowWaveform;

  /// No description provided for @settingsLayoutWaveformTop.
  ///
  /// In en, this message translates to:
  /// **'Top'**
  String get settingsLayoutWaveformTop;

  /// No description provided for @settingsLayoutWaveformMiddleHorizontal.
  ///
  /// In en, this message translates to:
  /// **'Middle horizontal'**
  String get settingsLayoutWaveformMiddleHorizontal;

  /// No description provided for @settingsLayoutWaveformMiddleVertical.
  ///
  /// In en, this message translates to:
  /// **'Middle vertical'**
  String get settingsLayoutWaveformMiddleVertical;

  /// No description provided for @settingsLayoutWaveformBottom.
  ///
  /// In en, this message translates to:
  /// **'Bottom'**
  String get settingsLayoutWaveformBottom;

  /// No description provided for @settingsLayoutLibraryDrawerTitle.
  ///
  /// In en, this message translates to:
  /// **'Library drawer'**
  String get settingsLayoutLibraryDrawerTitle;

  /// No description provided for @settingsLayoutLibraryDrawerDescription.
  ///
  /// In en, this message translates to:
  /// **'Where the library opens. Drawer mode hides the docked library and opens it over the mixer.'**
  String get settingsLayoutLibraryDrawerDescription;

  /// No description provided for @settingsLayoutLibraryDrawerEnable.
  ///
  /// In en, this message translates to:
  /// **'Library as a drawer'**
  String get settingsLayoutLibraryDrawerEnable;

  /// No description provided for @settingsLayoutLibraryDrawerPosition.
  ///
  /// In en, this message translates to:
  /// **'Position'**
  String get settingsLayoutLibraryDrawerPosition;

  /// No description provided for @settingsLayoutLibraryDrawerLeft.
  ///
  /// In en, this message translates to:
  /// **'Left'**
  String get settingsLayoutLibraryDrawerLeft;

  /// No description provided for @settingsLayoutLibraryDrawerRight.
  ///
  /// In en, this message translates to:
  /// **'Right'**
  String get settingsLayoutLibraryDrawerRight;

  /// No description provided for @settingsLayoutLibraryDrawerTop.
  ///
  /// In en, this message translates to:
  /// **'Top'**
  String get settingsLayoutLibraryDrawerTop;

  /// No description provided for @settingsLayoutLibraryDrawerBottom.
  ///
  /// In en, this message translates to:
  /// **'Bottom'**
  String get settingsLayoutLibraryDrawerBottom;

  /// No description provided for @settingsUiDescription.
  ///
  /// In en, this message translates to:
  /// **'Theme, language, hover tips, and control presentation.'**
  String get settingsUiDescription;

  /// No description provided for @settingsUiChromeTitle.
  ///
  /// In en, this message translates to:
  /// **'Chrome and controls'**
  String get settingsUiChromeTitle;

  /// No description provided for @settingsUiChromeDescription.
  ///
  /// In en, this message translates to:
  /// **'Language, hover tips, and how selects open.'**
  String get settingsUiChromeDescription;

  /// No description provided for @settingsUiThemeTitle.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get settingsUiThemeTitle;

  /// No description provided for @settingsUiThemeDescription.
  ///
  /// In en, this message translates to:
  /// **'Brightness and the accent color used by brand chrome.'**
  String get settingsUiThemeDescription;

  /// No description provided for @settingsUiTheme.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get settingsUiTheme;

  /// No description provided for @settingsUiThemeLight.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get settingsUiThemeLight;

  /// No description provided for @settingsUiThemeDark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get settingsUiThemeDark;

  /// No description provided for @settingsUiAccentColor.
  ///
  /// In en, this message translates to:
  /// **'Accent color'**
  String get settingsUiAccentColor;

  /// No description provided for @settingsUiAccentTeal.
  ///
  /// In en, this message translates to:
  /// **'Teal'**
  String get settingsUiAccentTeal;

  /// No description provided for @settingsUiAccentBlue.
  ///
  /// In en, this message translates to:
  /// **'Blue'**
  String get settingsUiAccentBlue;

  /// No description provided for @settingsUiAccentViolet.
  ///
  /// In en, this message translates to:
  /// **'Violet'**
  String get settingsUiAccentViolet;

  /// No description provided for @settingsUiAccentOrange.
  ///
  /// In en, this message translates to:
  /// **'Orange'**
  String get settingsUiAccentOrange;

  /// No description provided for @settingsUiAccentPink.
  ///
  /// In en, this message translates to:
  /// **'Pink'**
  String get settingsUiAccentPink;

  /// No description provided for @settingsUiLanguageLabel.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get settingsUiLanguageLabel;

  /// No description provided for @settingsSystemDefault.
  ///
  /// In en, this message translates to:
  /// **'System default'**
  String get settingsSystemDefault;

  /// No description provided for @settingsLanguageEn.
  ///
  /// In en, this message translates to:
  /// **'English'**
  String get settingsLanguageEn;

  /// No description provided for @settingsLanguagePtBr.
  ///
  /// In en, this message translates to:
  /// **'Português (Brasil)'**
  String get settingsLanguagePtBr;

  /// No description provided for @settingsShowTooltips.
  ///
  /// In en, this message translates to:
  /// **'Show tooltips'**
  String get settingsShowTooltips;

  /// No description provided for @settingsUiOnboardingTitle.
  ///
  /// In en, this message translates to:
  /// **'First-run setup'**
  String get settingsUiOnboardingTitle;

  /// No description provided for @settingsUiOnboardingDescription.
  ///
  /// In en, this message translates to:
  /// **'Show the welcome steps again instead of the mixer. Saved with the rest of Settings.'**
  String get settingsUiOnboardingDescription;

  /// No description provided for @settingsUiOnboardingReplay.
  ///
  /// In en, this message translates to:
  /// **'Show setup again'**
  String get settingsUiOnboardingReplay;

  /// No description provided for @onboardingStepProgress.
  ///
  /// In en, this message translates to:
  /// **'Step {index} of {total}'**
  String onboardingStepProgress(int index, int total);

  /// No description provided for @onboardingSkip.
  ///
  /// In en, this message translates to:
  /// **'Skip setup'**
  String get onboardingSkip;

  /// No description provided for @onboardingBack.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get onboardingBack;

  /// No description provided for @onboardingNext.
  ///
  /// In en, this message translates to:
  /// **'Continue'**
  String get onboardingNext;

  /// No description provided for @onboardingSaveFailedToast.
  ///
  /// In en, this message translates to:
  /// **'Setup could not be saved'**
  String get onboardingSaveFailedToast;

  /// No description provided for @onboardingWelcomeTitle.
  ///
  /// In en, this message translates to:
  /// **'Welcome to Mixar'**
  String get onboardingWelcomeTitle;

  /// No description provided for @onboardingWelcomeBody.
  ///
  /// In en, this message translates to:
  /// **'Just a few steps to get Mixar in your style.'**
  String get onboardingWelcomeBody;

  /// No description provided for @onboardingAppearanceTitle.
  ///
  /// In en, this message translates to:
  /// **'Make it yours'**
  String get onboardingAppearanceTitle;

  /// No description provided for @onboardingAppearanceBody.
  ///
  /// In en, this message translates to:
  /// **'Pick a theme and an accent color. Both repaint the app as you choose.'**
  String get onboardingAppearanceBody;

  /// No description provided for @onboardingLibraryTitle.
  ///
  /// In en, this message translates to:
  /// **'Bring your music'**
  String get onboardingLibraryTitle;

  /// No description provided for @onboardingLibraryBody.
  ///
  /// In en, this message translates to:
  /// **'Point Mixar at a folder of tracks, or import a library from another DJ app. Audio files are never moved.'**
  String get onboardingLibraryBody;

  /// No description provided for @onboardingLibraryChooseFolder.
  ///
  /// In en, this message translates to:
  /// **'Choose music folder'**
  String get onboardingLibraryChooseFolder;

  /// No description provided for @onboardingLibraryFolderAdded.
  ///
  /// In en, this message translates to:
  /// **'Folder added'**
  String get onboardingLibraryFolderAdded;

  /// No description provided for @onboardingLibraryFolderFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not add that folder'**
  String get onboardingLibraryFolderFailed;

  /// No description provided for @onboardingPrivacyTitle.
  ///
  /// In en, this message translates to:
  /// **'Your data stays here'**
  String get onboardingPrivacyTitle;

  /// No description provided for @onboardingPrivacyBody.
  ///
  /// In en, this message translates to:
  /// **'Off by default. Sends anonymous events and error reports; file paths are redacted and your music is never uploaded.'**
  String get onboardingPrivacyBody;

  /// No description provided for @onboardingControllersTitle.
  ///
  /// In en, this message translates to:
  /// **'Set up your controller'**
  String get onboardingControllersTitle;

  /// No description provided for @onboardingControllersTrusted.
  ///
  /// In en, this message translates to:
  /// **'Trusted devices connect automatically next launch.'**
  String get onboardingControllersTrusted;

  /// No description provided for @onboardingControllersMappings.
  ///
  /// In en, this message translates to:
  /// **'Mappings live in Settings → Controllers.'**
  String get onboardingControllersMappings;

  /// No description provided for @onboardingKickerSetup.
  ///
  /// In en, this message translates to:
  /// **'Setup'**
  String get onboardingKickerSetup;

  /// No description provided for @onboardingKickerAppearance.
  ///
  /// In en, this message translates to:
  /// **'Appearance'**
  String get onboardingKickerAppearance;

  /// No description provided for @onboardingKickerLibrary.
  ///
  /// In en, this message translates to:
  /// **'Library'**
  String get onboardingKickerLibrary;

  /// No description provided for @onboardingKickerPrivacy.
  ///
  /// In en, this message translates to:
  /// **'Privacy'**
  String get onboardingKickerPrivacy;

  /// No description provided for @onboardingKickerControllers.
  ///
  /// In en, this message translates to:
  /// **'Controllers'**
  String get onboardingKickerControllers;

  /// No description provided for @onboardingLibraryImportLabel.
  ///
  /// In en, this message translates to:
  /// **'Or import an existing library'**
  String get onboardingLibraryImportLabel;

  /// No description provided for @onboardingPrivacyOffTitle.
  ///
  /// In en, this message translates to:
  /// **'Keep it private'**
  String get onboardingPrivacyOffTitle;

  /// No description provided for @onboardingPrivacyOffBody.
  ///
  /// In en, this message translates to:
  /// **'No analytics. Nothing leaves this machine.'**
  String get onboardingPrivacyOffBody;

  /// No description provided for @onboardingPrivacyOnTitle.
  ///
  /// In en, this message translates to:
  /// **'Share anonymous usage'**
  String get onboardingPrivacyOnTitle;

  /// No description provided for @onboardingPrivacyOnBody.
  ///
  /// In en, this message translates to:
  /// **'Anonymous events and error reports. No music, no file names.'**
  String get onboardingPrivacyOnBody;

  /// No description provided for @onboardingLanguageFollowsOs.
  ///
  /// In en, this message translates to:
  /// **'Follows the system language'**
  String get onboardingLanguageFollowsOs;

  /// No description provided for @onboardingLibraryFolderHint.
  ///
  /// In en, this message translates to:
  /// **'Mixar reads the files where they are. Nothing is moved or copied.'**
  String get onboardingLibraryFolderHint;

  /// No description provided for @onboardingLibraryFolderReady.
  ///
  /// In en, this message translates to:
  /// **'Added “{name}”. Pick another folder if you want more.'**
  String onboardingLibraryFolderReady(String name);

  /// No description provided for @onboardingLibraryBrowse.
  ///
  /// In en, this message translates to:
  /// **'Browse'**
  String get onboardingLibraryBrowse;

  /// No description provided for @onboardingReadyTitle.
  ///
  /// In en, this message translates to:
  /// **'You\'re all set'**
  String get onboardingReadyTitle;

  /// No description provided for @onboardingReadyBody.
  ///
  /// In en, this message translates to:
  /// **'Save these choices and the mixer opens ready to play.'**
  String get onboardingReadyBody;

  /// No description provided for @onboardingReadyFootnote.
  ///
  /// In en, this message translates to:
  /// **'Nothing here is permanent — every choice lives in Settings.'**
  String get onboardingReadyFootnote;

  /// No description provided for @onboardingStart.
  ///
  /// In en, this message translates to:
  /// **'Start mixing'**
  String get onboardingStart;

  /// No description provided for @onboardingTipLoadTitle.
  ///
  /// In en, this message translates to:
  /// **'Load your first track'**
  String get onboardingTipLoadTitle;

  /// No description provided for @onboardingTipLoadBody.
  ///
  /// In en, this message translates to:
  /// **'Drag a file onto a deck, or double-click a row in the library.'**
  String get onboardingTipLoadBody;

  /// No description provided for @onboardingTipAnalyzeTitle.
  ///
  /// In en, this message translates to:
  /// **'Analysis runs itself'**
  String get onboardingTipAnalyzeTitle;

  /// No description provided for @onboardingTipAnalyzeBody.
  ///
  /// In en, this message translates to:
  /// **'Beat grid, waveform, key and BPM are written as soon as a track lands in the library.'**
  String get onboardingTipAnalyzeBody;

  /// No description provided for @onboardingTipSettingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Tune it later'**
  String get onboardingTipSettingsTitle;

  /// No description provided for @onboardingTipSettingsBody.
  ///
  /// In en, this message translates to:
  /// **'Audio, controllers, layout and waveform are all under Settings.'**
  String get onboardingTipSettingsBody;

  /// No description provided for @onboardingKickerReady.
  ///
  /// In en, this message translates to:
  /// **'Ready'**
  String get onboardingKickerReady;

  /// No description provided for @onboardingControllersDetected.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{One controller found. Trust it to connect automatically.} other{{count} controllers found. Trust the ones you want to connect automatically.}}'**
  String onboardingControllersDetected(int count);

  /// No description provided for @onboardingControllersMapping.
  ///
  /// In en, this message translates to:
  /// **'Mapping: {name}'**
  String onboardingControllersMapping(String name);

  /// No description provided for @onboardingControllersTrust.
  ///
  /// In en, this message translates to:
  /// **'Trust'**
  String get onboardingControllersTrust;

  /// No description provided for @onboardingControllersTrustedAction.
  ///
  /// In en, this message translates to:
  /// **'Trusted'**
  String get onboardingControllersTrustedAction;

  /// No description provided for @onboardingFloatDecks.
  ///
  /// In en, this message translates to:
  /// **'Two decks'**
  String get onboardingFloatDecks;

  /// No description provided for @onboardingFloatWaveform.
  ///
  /// In en, this message translates to:
  /// **'Waveform'**
  String get onboardingFloatWaveform;

  /// No description provided for @onboardingFloatStems.
  ///
  /// In en, this message translates to:
  /// **'Stems'**
  String get onboardingFloatStems;

  /// No description provided for @onboardingFloatMixer.
  ///
  /// In en, this message translates to:
  /// **'Mixer'**
  String get onboardingFloatMixer;

  /// No description provided for @onboardingFloatJog.
  ///
  /// In en, this message translates to:
  /// **'Jog'**
  String get onboardingFloatJog;

  /// No description provided for @onboardingFloatSync.
  ///
  /// In en, this message translates to:
  /// **'Sync'**
  String get onboardingFloatSync;

  /// No description provided for @onboardingFloatCue.
  ///
  /// In en, this message translates to:
  /// **'Cue'**
  String get onboardingFloatCue;

  /// No description provided for @onboardingFloatLoop.
  ///
  /// In en, this message translates to:
  /// **'Loop'**
  String get onboardingFloatLoop;

  /// No description provided for @onboardingFloatFx.
  ///
  /// In en, this message translates to:
  /// **'FX'**
  String get onboardingFloatFx;

  /// No description provided for @onboardingFloatLibrary.
  ///
  /// In en, this message translates to:
  /// **'Library'**
  String get onboardingFloatLibrary;

  /// No description provided for @onboardingKickerLanguage.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get onboardingKickerLanguage;

  /// No description provided for @onboardingLanguageTitle.
  ///
  /// In en, this message translates to:
  /// **'Pick your language'**
  String get onboardingLanguageTitle;

  /// No description provided for @onboardingLanguageBody.
  ///
  /// In en, this message translates to:
  /// **'Mixar follows the system language unless you choose one. The whole app switches as soon as you pick.'**
  String get onboardingLanguageBody;

  /// No description provided for @onboardingFloatBpm.
  ///
  /// In en, this message translates to:
  /// **'BPM'**
  String get onboardingFloatBpm;

  /// No description provided for @onboardingFloatKeys.
  ///
  /// In en, this message translates to:
  /// **'Keys'**
  String get onboardingFloatKeys;

  /// No description provided for @onboardingFloatCues.
  ///
  /// In en, this message translates to:
  /// **'Hot cues'**
  String get onboardingFloatCues;

  /// No description provided for @onboardingFloatAnalyze.
  ///
  /// In en, this message translates to:
  /// **'Analyze'**
  String get onboardingFloatAnalyze;

  /// No description provided for @settingsUiSelectStyle.
  ///
  /// In en, this message translates to:
  /// **'Select style'**
  String get settingsUiSelectStyle;

  /// No description provided for @settingsUiSelectStyleAuto.
  ///
  /// In en, this message translates to:
  /// **'Auto'**
  String get settingsUiSelectStyleAuto;

  /// No description provided for @settingsUiSelectStyleDesktop.
  ///
  /// In en, this message translates to:
  /// **'Desktop'**
  String get settingsUiSelectStyleDesktop;

  /// No description provided for @settingsUiSelectStyleMobile.
  ///
  /// In en, this message translates to:
  /// **'Mobile'**
  String get settingsUiSelectStyleMobile;

  /// No description provided for @settingsUiSelectStyleAutoSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Platform default — desktop OS uses popovers, phones use dialogs.'**
  String get settingsUiSelectStyleAutoSubtitle;

  /// No description provided for @settingsUiSelectStyleDesktopSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Anchored popovers for selects and ⋯ menus.'**
  String get settingsUiSelectStyleDesktopSubtitle;

  /// No description provided for @settingsUiSelectStyleMobileSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Open selects and ⋯ menus in a dialog.'**
  String get settingsUiSelectStyleMobileSubtitle;

  /// No description provided for @settingsAudioDescription.
  ///
  /// In en, this message translates to:
  /// **'Engine output and buses.'**
  String get settingsAudioDescription;

  /// No description provided for @settingsAudioEngineTitle.
  ///
  /// In en, this message translates to:
  /// **'Engine'**
  String get settingsAudioEngineTitle;

  /// No description provided for @settingsAudioEngineDescription.
  ///
  /// In en, this message translates to:
  /// **'Resampler and buffer size.'**
  String get settingsAudioEngineDescription;

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

  /// No description provided for @settingsAudioSampleRateHint.
  ///
  /// In en, this message translates to:
  /// **'Device default follows the rate the output device mixes at.'**
  String get settingsAudioSampleRateHint;

  /// No description provided for @settingsAudioSampleRateHz.
  ///
  /// In en, this message translates to:
  /// **'{rate} Hz'**
  String settingsAudioSampleRateHz(int rate);

  /// No description provided for @settingsAudioSampleRateDeviceDefault.
  ///
  /// In en, this message translates to:
  /// **'Device default'**
  String get settingsAudioSampleRateDeviceDefault;

  /// No description provided for @settingsAudioSampleRateDeviceDefaultWithRate.
  ///
  /// In en, this message translates to:
  /// **'Device default ({rate} Hz)'**
  String settingsAudioSampleRateDeviceDefaultWithRate(int rate);

  /// No description provided for @settingsAudioSampleRateLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading rates for this device…'**
  String get settingsAudioSampleRateLoading;

  /// No description provided for @settingsAudioResamplerQuality.
  ///
  /// In en, this message translates to:
  /// **'Resampler quality'**
  String get settingsAudioResamplerQuality;

  /// No description provided for @settingsAudioResamplerLow.
  ///
  /// In en, this message translates to:
  /// **'Low'**
  String get settingsAudioResamplerLow;

  /// No description provided for @settingsAudioResamplerMedium.
  ///
  /// In en, this message translates to:
  /// **'Medium'**
  String get settingsAudioResamplerMedium;

  /// No description provided for @settingsAudioResamplerHigh.
  ///
  /// In en, this message translates to:
  /// **'High'**
  String get settingsAudioResamplerHigh;

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

  /// No description provided for @settingsAudioMasterBusDescription.
  ///
  /// In en, this message translates to:
  /// **'Primary output device and channel routing.'**
  String get settingsAudioMasterBusDescription;

  /// No description provided for @settingsAudioPreviewBusTitle.
  ///
  /// In en, this message translates to:
  /// **'Preview bus'**
  String get settingsAudioPreviewBusTitle;

  /// No description provided for @settingsAudioPreviewBusDescription.
  ///
  /// In en, this message translates to:
  /// **'Headphones / cue output when preview is enabled.'**
  String get settingsAudioPreviewBusDescription;

  /// No description provided for @settingsAudioPreviewBus.
  ///
  /// In en, this message translates to:
  /// **'Enable preview bus'**
  String get settingsAudioPreviewBus;

  /// No description provided for @settingsAudioDevice.
  ///
  /// In en, this message translates to:
  /// **'Device'**
  String get settingsAudioDevice;

  /// Device picker entry for an output that cannot reach 44.1 kHz.
  ///
  /// In en, this message translates to:
  /// **'{device} — low quality output'**
  String settingsAudioDeviceLowQuality(String device);

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
  /// **'Loudness matching, sampler routing, and realtime stem fallback.'**
  String get settingsMixerDescription;

  /// No description provided for @settingsMixerNormalizerTitle.
  ///
  /// In en, this message translates to:
  /// **'Volume normalizer'**
  String get settingsMixerNormalizerTitle;

  /// No description provided for @settingsMixerNormalizerDescription.
  ///
  /// In en, this message translates to:
  /// **'Match analyzed tracks to a target loudness.'**
  String get settingsMixerNormalizerDescription;

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

  /// No description provided for @settingsMixerSamplerRoutingTitle.
  ///
  /// In en, this message translates to:
  /// **'Sampler routing'**
  String get settingsMixerSamplerRoutingTitle;

  /// No description provided for @settingsMixerSamplerRoutingDescription.
  ///
  /// In en, this message translates to:
  /// **'Where sampler hits enter the signal path, relative to the channel strip.'**
  String get settingsMixerSamplerRoutingDescription;

  /// No description provided for @settingsMixerSamplerStripRoute.
  ///
  /// In en, this message translates to:
  /// **'Sampler strip route'**
  String get settingsMixerSamplerStripRoute;

  /// No description provided for @settingsMixerSamplerStripBefore.
  ///
  /// In en, this message translates to:
  /// **'Before channel strip'**
  String get settingsMixerSamplerStripBefore;

  /// No description provided for @settingsMixerSamplerStripAfter.
  ///
  /// In en, this message translates to:
  /// **'After channel strip'**
  String get settingsMixerSamplerStripAfter;

  /// No description provided for @settingsMixerRealtimeStemsTitle.
  ///
  /// In en, this message translates to:
  /// **'Realtime Stems'**
  String get settingsMixerRealtimeStemsTitle;

  /// No description provided for @settingsMixerRealtimeStemsDescription.
  ///
  /// In en, this message translates to:
  /// **'Separates tracks without cached stems while they play. Tracks with offline stems always take priority.'**
  String get settingsMixerRealtimeStemsDescription;

  /// No description provided for @settingsMixerRealtimeStemsNotRecommended.
  ///
  /// In en, this message translates to:
  /// **'Not recommended.'**
  String get settingsMixerRealtimeStemsNotRecommended;

  /// No description provided for @settingsMixerRealtimeStemsWarning.
  ///
  /// In en, this message translates to:
  /// **'Quality is lower and it uses a CPU core.'**
  String get settingsMixerRealtimeStemsWarning;

  /// No description provided for @settingsWaveformDescription.
  ///
  /// In en, this message translates to:
  /// **'RGB mixes low/mid/high into one color. Filtered stacks the three bands.'**
  String get settingsWaveformDescription;

  /// No description provided for @settingsWaveformDisplayTitle.
  ///
  /// In en, this message translates to:
  /// **'Display'**
  String get settingsWaveformDisplayTitle;

  /// No description provided for @settingsWaveformDisplayDescription.
  ///
  /// In en, this message translates to:
  /// **'RGB mixes low/mid/high into one color. Filtered stacks the three bands.'**
  String get settingsWaveformDisplayDescription;

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
  /// **'Default jog, tempo, brake, and sampler behavior for new decks.'**
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

  /// No description provided for @settingsDeckVinylBrakePause.
  ///
  /// In en, this message translates to:
  /// **'Vinyl brake pause'**
  String get settingsDeckVinylBrakePause;

  /// No description provided for @settingsDeckVinylBrakePauseHint.
  ///
  /// In en, this message translates to:
  /// **'Pause spins the deck down like a turntable instead of cutting instantly.'**
  String get settingsDeckVinylBrakePauseHint;

  /// No description provided for @settingsDeckVinylBrakePauseToggle.
  ///
  /// In en, this message translates to:
  /// **'Brake on pause'**
  String get settingsDeckVinylBrakePauseToggle;

  /// No description provided for @settingsDeckVinylBrakeStopTime.
  ///
  /// In en, this message translates to:
  /// **'Stop time'**
  String get settingsDeckVinylBrakeStopTime;

  /// No description provided for @settingsDeckVinylBrakeStopTimeHint.
  ///
  /// In en, this message translates to:
  /// **'How long the platter takes to stop after pause.'**
  String get settingsDeckVinylBrakeStopTimeHint;

  /// Screen-reader label for a numeric stepper's decrement button.
  ///
  /// In en, this message translates to:
  /// **'Decrease {field}'**
  String settingsStepperDecrease(String field);

  /// Screen-reader label for a numeric stepper's increment button.
  ///
  /// In en, this message translates to:
  /// **'Increase {field}'**
  String settingsStepperIncrease(String field);

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

  /// No description provided for @settingsDeckDefaultSamplerBankC.
  ///
  /// In en, this message translates to:
  /// **'Deck C default sampler bank'**
  String get settingsDeckDefaultSamplerBankC;

  /// No description provided for @settingsDeckDefaultSamplerBankD.
  ///
  /// In en, this message translates to:
  /// **'Deck D default sampler bank'**
  String get settingsDeckDefaultSamplerBankD;

  /// No description provided for @settingsLayoutDeckCountRestartTitle.
  ///
  /// In en, this message translates to:
  /// **'Change deck layout?'**
  String get settingsLayoutDeckCountRestartTitle;

  /// No description provided for @settingsLayoutDeckCountRestartBody.
  ///
  /// In en, this message translates to:
  /// **'Changing the deck count restarts the audio engine and reloads tracks. Tracks on decks removed by switching to 2 decks will not be restored.'**
  String get settingsLayoutDeckCountRestartBody;

  /// No description provided for @settingsLayoutDeckCountRestartConfirm.
  ///
  /// In en, this message translates to:
  /// **'Restart and apply'**
  String get settingsLayoutDeckCountRestartConfirm;

  /// No description provided for @settingsLibraryListTitle.
  ///
  /// In en, this message translates to:
  /// **'Library list'**
  String get settingsLibraryListTitle;

  /// No description provided for @settingsLibraryListDescription.
  ///
  /// In en, this message translates to:
  /// **'How tracks look in the library table.'**
  String get settingsLibraryListDescription;

  /// No description provided for @settingsLibraryDescription.
  ///
  /// In en, this message translates to:
  /// **'Offline analysis, metadata, and list display.'**
  String get settingsLibraryDescription;

  /// No description provided for @settingsIntegrationsDescription.
  ///
  /// In en, this message translates to:
  /// **'Read another DJ application into your library.'**
  String get settingsIntegrationsDescription;

  /// No description provided for @settingsLibraryAnalysisTitle.
  ///
  /// In en, this message translates to:
  /// **'Analysis'**
  String get settingsLibraryAnalysisTitle;

  /// No description provided for @settingsLibraryAnalysisDescription.
  ///
  /// In en, this message translates to:
  /// **'How thoroughly offline analysis scans each track.'**
  String get settingsLibraryAnalysisDescription;

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

  /// No description provided for @settingsLibraryTagWriteBackTitle.
  ///
  /// In en, this message translates to:
  /// **'File tags'**
  String get settingsLibraryTagWriteBackTitle;

  /// No description provided for @settingsLibraryTagWriteBackDescription.
  ///
  /// In en, this message translates to:
  /// **'Mixar keeps BPM and key in its own database. When enabled, it also writes them into each file\'s tags so other DJ software sees the same values. Off by default — Mixar never modifies your files while this is off.'**
  String get settingsLibraryTagWriteBackDescription;

  /// No description provided for @settingsLibraryTagWriteBack.
  ///
  /// In en, this message translates to:
  /// **'Write BPM and key to file tags'**
  String get settingsLibraryTagWriteBack;

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

  /// No description provided for @padStemsOffline.
  ///
  /// In en, this message translates to:
  /// **'Offline'**
  String get padStemsOffline;

  /// No description provided for @padStemsRealtime.
  ///
  /// In en, this message translates to:
  /// **'Realtime'**
  String get padStemsRealtime;

  /// No description provided for @settingsLibraryStemsTitle.
  ///
  /// In en, this message translates to:
  /// **'Stems'**
  String get settingsLibraryStemsTitle;

  /// No description provided for @settingsLibraryStemsDescription.
  ///
  /// In en, this message translates to:
  /// **'Model and codec used when writing separated stem caches.'**
  String get settingsLibraryStemsDescription;

  /// No description provided for @settingsLibraryStemFormat.
  ///
  /// In en, this message translates to:
  /// **'Stem format'**
  String get settingsLibraryStemFormat;

  /// No description provided for @settingsLibraryStemModel.
  ///
  /// In en, this message translates to:
  /// **'Stem model'**
  String get settingsLibraryStemModel;

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

  /// No description provided for @settingsStorageOverviewTitle.
  ///
  /// In en, this message translates to:
  /// **'Mixar storage'**
  String get settingsStorageOverviewTitle;

  /// No description provided for @settingsStorageOverviewDescription.
  ///
  /// In en, this message translates to:
  /// **'Disk use for caches and library metadata under app support.'**
  String get settingsStorageOverviewDescription;

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
  /// **'Bring your Mixxx library into Mixar.'**
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

  /// No description provided for @rekordboxImportTitle.
  ///
  /// In en, this message translates to:
  /// **'Import from Rekordbox'**
  String get rekordboxImportTitle;

  /// No description provided for @rekordboxImportDescription.
  ///
  /// In en, this message translates to:
  /// **'Bring your Rekordbox library into Mixar.'**
  String get rekordboxImportDescription;

  /// No description provided for @rekordboxImportFound.
  ///
  /// In en, this message translates to:
  /// **'Found: {path}'**
  String rekordboxImportFound(String path);

  /// No description provided for @rekordboxImportCheckFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not check for a Rekordbox library.'**
  String get rekordboxImportCheckFailed;

  /// No description provided for @rekordboxImportLooking.
  ///
  /// In en, this message translates to:
  /// **'Looking for a Rekordbox library…'**
  String get rekordboxImportLooking;

  /// No description provided for @rekordboxImportNotFound.
  ///
  /// In en, this message translates to:
  /// **'No Rekordbox library found on this computer.'**
  String get rekordboxImportNotFound;

  /// No description provided for @rekordboxImportButton.
  ///
  /// In en, this message translates to:
  /// **'Import from Rekordbox library…'**
  String get rekordboxImportButton;

  /// No description provided for @rekordboxImporting.
  ///
  /// In en, this message translates to:
  /// **'Importing…'**
  String get rekordboxImporting;

  /// No description provided for @rekordboxImportConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Import from Rekordbox?'**
  String get rekordboxImportConfirmTitle;

  /// No description provided for @rekordboxImportConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'This imports tracks, playlists, hot cues, and loops from:\n{path}\n\n{trackCount, plural, =1{1 track} other{{trackCount} tracks}} ({missingFileCount, plural, =1{1 missing} other{{missingFileCount} missing}}), {playlistCount, plural, =1{1 playlist} other{{playlistCount} playlists}} ({smartPlaylistCount, plural, =1{1 smart} other{{smartPlaylistCount} smart}}), {folderCount, plural, =1{1 folder} other{{folderCount} folders}}, {hotCueCount, plural, =1{1 hot cue} other{{hotCueCount} hot cues}}, {loopCount, plural, =1{1 loop} other{{loopCount} loops}}.\n\nMissing files are imported as unavailable tracks.'**
  String rekordboxImportConfirmBody(
    String path,
    int trackCount,
    int missingFileCount,
    int playlistCount,
    int smartPlaylistCount,
    int folderCount,
    int hotCueCount,
    int loopCount,
  );

  /// No description provided for @rekordboxImportReadFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not read Rekordbox library'**
  String get rekordboxImportReadFailed;

  /// No description provided for @rekordboxImportFailed.
  ///
  /// In en, this message translates to:
  /// **'Rekordbox import failed'**
  String get rekordboxImportFailed;

  /// No description provided for @rekordboxImportFinishedWithErrors.
  ///
  /// In en, this message translates to:
  /// **'Rekordbox import finished with {count} error(s)'**
  String rekordboxImportFinishedWithErrors(int count);

  /// No description provided for @rekordboxImportAlreadyImported.
  ///
  /// In en, this message translates to:
  /// **'Rekordbox library already imported'**
  String get rekordboxImportAlreadyImported;

  /// No description provided for @rekordboxImportNothing.
  ///
  /// In en, this message translates to:
  /// **'Nothing to import from Rekordbox'**
  String get rekordboxImportNothing;

  /// No description provided for @rekordboxImportUpdated.
  ///
  /// In en, this message translates to:
  /// **'Updated {updated} from Rekordbox{missing}'**
  String rekordboxImportUpdated(String updated, String missing);

  /// No description provided for @rekordboxImportImported.
  ///
  /// In en, this message translates to:
  /// **'Imported {parts} from Rekordbox{missing}'**
  String rekordboxImportImported(String parts, String missing);

  /// No description provided for @rekordboxImportMissingSuffix.
  ///
  /// In en, this message translates to:
  /// **' ({count} missing)'**
  String rekordboxImportMissingSuffix(int count);

  /// No description provided for @rekordboxCountTracks.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 track} other{{count} tracks}}'**
  String rekordboxCountTracks(int count);

  /// No description provided for @rekordboxCountPlaylists.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 playlist} other{{count} playlists}}'**
  String rekordboxCountPlaylists(int count);

  /// No description provided for @rekordboxCountHotCues.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 hot cue} other{{count} hot cues}}'**
  String rekordboxCountHotCues(int count);

  /// No description provided for @rekordboxCountLoops.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 loop} other{{count} loops}}'**
  String rekordboxCountLoops(int count);

  /// No description provided for @rekordboxCountUpdated.
  ///
  /// In en, this message translates to:
  /// **'{count} updated'**
  String rekordboxCountUpdated(int count);

  /// No description provided for @cdjExportTitle.
  ///
  /// In en, this message translates to:
  /// **'Export to CDJ USB'**
  String get cdjExportTitle;

  /// No description provided for @cdjExportDescription.
  ///
  /// In en, this message translates to:
  /// **'Write playlists and folder collections to a Pioneer CDJ/XDJ USB drive, so players browse them without Rekordbox. Each export adds to what the drive already holds.'**
  String get cdjExportDescription;

  /// No description provided for @cdjExportDestination.
  ///
  /// In en, this message translates to:
  /// **'Destination'**
  String get cdjExportDestination;

  /// No description provided for @cdjExportDestinationHint.
  ///
  /// In en, this message translates to:
  /// **'USB drive or folder'**
  String get cdjExportDestinationHint;

  /// No description provided for @cdjExportDestinationMissing.
  ///
  /// In en, this message translates to:
  /// **'Choose a destination drive or folder.'**
  String get cdjExportDestinationMissing;

  /// No description provided for @cdjExportBrowse.
  ///
  /// In en, this message translates to:
  /// **'Browse…'**
  String get cdjExportBrowse;

  /// No description provided for @cdjExportCollection.
  ///
  /// In en, this message translates to:
  /// **'Collection'**
  String get cdjExportCollection;

  /// No description provided for @cdjExportOverride.
  ///
  /// In en, this message translates to:
  /// **'Override files already on the drive'**
  String get cdjExportOverride;

  /// No description provided for @cdjExportConfirmAction.
  ///
  /// In en, this message translates to:
  /// **'Export'**
  String get cdjExportConfirmAction;

  /// No description provided for @cdjExporting.
  ///
  /// In en, this message translates to:
  /// **'Exporting…'**
  String get cdjExporting;

  /// No description provided for @cdjExportFailed.
  ///
  /// In en, this message translates to:
  /// **'CDJ USB export failed'**
  String get cdjExportFailed;

  /// No description provided for @cdjExportFinishedWithErrors.
  ///
  /// In en, this message translates to:
  /// **'CDJ USB export finished with {count} error(s)'**
  String cdjExportFinishedWithErrors(int count);

  /// No description provided for @cdjExportFinishedWithWarnings.
  ///
  /// In en, this message translates to:
  /// **'CDJ USB export finished with {count} warning(s)'**
  String cdjExportFinishedWithWarnings(int count);

  /// No description provided for @cdjExportExported.
  ///
  /// In en, this message translates to:
  /// **'Exported {parts}'**
  String cdjExportExported(String parts);

  /// No description provided for @cdjExportNothing.
  ///
  /// In en, this message translates to:
  /// **'Nothing to export'**
  String get cdjExportNothing;

  /// No description provided for @cdjCountTracks.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 track} other{{count} tracks}}'**
  String cdjCountTracks(int count);

  /// No description provided for @cdjCountReused.
  ///
  /// In en, this message translates to:
  /// **'{count} reused'**
  String cdjCountReused(int count);

  /// No description provided for @cdjCountPlaylists.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 playlist} other{{count} playlists}}'**
  String cdjCountPlaylists(int count);

  /// No description provided for @cdjCountFolders.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 folder} other{{count} folders}}'**
  String cdjCountFolders(int count);

  /// No description provided for @cdjCountCopied.
  ///
  /// In en, this message translates to:
  /// **'{size} copied'**
  String cdjCountCopied(String size);

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

  /// No description provided for @settingsAnalyticsTitle.
  ///
  /// In en, this message translates to:
  /// **'Analytics'**
  String get settingsAnalyticsTitle;

  /// No description provided for @settingsAnalyticsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Share anonymous app data.'**
  String get settingsAnalyticsSubtitle;

  /// No description provided for @settingsAnalyticsToggle.
  ///
  /// In en, this message translates to:
  /// **'Send anonymous usage data'**
  String get settingsAnalyticsToggle;

  /// No description provided for @settingsLibraryVisibleFields.
  ///
  /// In en, this message translates to:
  /// **'Visible fields'**
  String get settingsLibraryVisibleFields;

  /// No description provided for @settingsLibraryFieldAlbum.
  ///
  /// In en, this message translates to:
  /// **'Album'**
  String get settingsLibraryFieldAlbum;

  /// No description provided for @settingsLibraryFieldGenre.
  ///
  /// In en, this message translates to:
  /// **'Genre'**
  String get settingsLibraryFieldGenre;

  /// No description provided for @settingsLibraryFieldYear.
  ///
  /// In en, this message translates to:
  /// **'Year'**
  String get settingsLibraryFieldYear;

  /// No description provided for @settingsLibraryFieldTrackNumber.
  ///
  /// In en, this message translates to:
  /// **'Track #'**
  String get settingsLibraryFieldTrackNumber;

  /// No description provided for @settingsLibraryFieldDiscNumber.
  ///
  /// In en, this message translates to:
  /// **'Disc #'**
  String get settingsLibraryFieldDiscNumber;

  /// No description provided for @settingsLibraryFieldComment.
  ///
  /// In en, this message translates to:
  /// **'Comment'**
  String get settingsLibraryFieldComment;

  /// No description provided for @settingsLibraryFieldIsrc.
  ///
  /// In en, this message translates to:
  /// **'ISRC'**
  String get settingsLibraryFieldIsrc;

  /// No description provided for @settingsLibraryFieldDuration.
  ///
  /// In en, this message translates to:
  /// **'Length'**
  String get settingsLibraryFieldDuration;

  /// No description provided for @settingsLibraryFieldRating.
  ///
  /// In en, this message translates to:
  /// **'Rating'**
  String get settingsLibraryFieldRating;

  /// No description provided for @settingsLibraryFieldColor.
  ///
  /// In en, this message translates to:
  /// **'Color label'**
  String get settingsLibraryFieldColor;

  /// No description provided for @settingsLibraryFieldBpm.
  ///
  /// In en, this message translates to:
  /// **'BPM'**
  String get settingsLibraryFieldBpm;

  /// No description provided for @settingsLibraryFieldKey.
  ///
  /// In en, this message translates to:
  /// **'Key'**
  String get settingsLibraryFieldKey;

  /// No description provided for @onboardingLibraryNotFoundOne.
  ///
  /// In en, this message translates to:
  /// **'No {name} library found on this computer. You can import one later in Settings → Integrations.'**
  String onboardingLibraryNotFoundOne(String name);

  /// No description provided for @onboardingLibraryNotFoundBoth.
  ///
  /// In en, this message translates to:
  /// **'No Mixxx or Rekordbox library found on this computer. You can import one later in Settings → Integrations.'**
  String get onboardingLibraryNotFoundBoth;
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
