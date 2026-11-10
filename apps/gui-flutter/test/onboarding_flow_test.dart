import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/onboarding/onboarding_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/screens/app_shell.dart';
import 'package:gui_flutter/screens/application.dart';
import 'package:gui_flutter/screens/onboarding_page.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_backdrop.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:gui_flutter/widgets/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/widgets/settings/rekordbox_import_panel.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_settings_transport.dart';
import 'support/mixar_material_app.dart';

/// The shipped bundle's `display_name()` ("vendor_name product_name");
/// both fixtures below are that one device.
const _ddjMappingName = 'Pioneer DDJ-400';
const _ddjDeviceId = 'pioneer.ddj-400';

/// A second MIDI port of the same physical device.
const _ddjPortTwo = DetectedController(
  portName: 'DDJ-400:DDJ-400 Control 40:0',
  mappingName: _ddjMappingName,
  deviceId: _ddjDeviceId,
);

const _ddj = DetectedController(
  portName: 'Pioneer DJ DDJ-400 MIDI 1',
  mappingName: _ddjMappingName,
  deviceId: _ddjDeviceId,
);

void main() {
  // Application builds DesktopChrome, which touches window_manager when the
  // host looks like a desktop. Widget tests have no window manager.
  setUp(() => debugOverrideDesktopWindow = false);
  tearDown(() => debugOverrideDesktopWindow = null);

  /// A desktop window, not the 800x600 test default: the steps are designed
  /// for the real minimum window and the narrow form would hide the columns.
  ///
  /// Motion off by default: the stage animates forever by design, so a
  /// settling test would never settle. This is also the OS "reduce motion"
  /// path; `motion: true` exercises the animated one.
  void configureTestSurface(WidgetTester tester, {bool motion = false}) {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    tester.platformDispatcher.accessibilityFeaturesTestValue = motion
        ? const FakeAccessibilityFeatures()
        : const FakeAccessibilityFeatures(disableAnimations: true);
    addTearDown(tester.platformDispatcher.clearAccessibilityFeaturesTestValue);
  }

  /// The providers every setup screen needs, with the host's probes stubbed.
  ///
  /// [settings] is optional so a test can supply its own settings override (a
  /// failing host, for example) instead of depending on list order.
  List<Override> setupOverrides({
    FakeSettingsTransport? settings,
    List<DetectedController> controllers = const [],
    String? mixxxPath,
    String? rekordboxPath,
  }) => [
    if (settings != null)
      settingsTransportProvider.overrideWith((_) async => settings),
    audioDevicesProvider.overrideWith((_) async => const []),
    samplerBanksProvider.overrideWith((_) async => const []),
    controllerTransportProvider.overrideWith((_) async => null),
    onboardingControllersProvider.overrideWith((_) async => controllers),
    mixxxDatabasePathProvider.overrideWith((_) async => mixxxPath),
    rekordboxDatabasePathProvider.overrideWith((_) async => rekordboxPath),
  ];

  /// Pump the setup page on its own, for the cases `Application` cannot reach
  /// (it falls back to the mixer whenever settings are not resolved).
  Future<void> pumpSetupPage(
    WidgetTester tester,
    ProviderContainer container,
  ) async {
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const OnboardingPage(appTitle: 'Mixar'),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Pump the setup page on a host whose settings read throws.
  /// Pump the setup page with the host's probes stubbed.
  ///
  /// [settingsUnreadable] models a host whose settings read fails; [settings]
  /// lets a test supply its own host (a failing save, for example).
  Future<void> pumpSetup(
    WidgetTester tester, {
    FakeSettingsTransport? settings,
    bool settingsUnreadable = false,
    void Function(ProviderContainer container)? onContainer,
  }) async {
    configureTestSurface(tester);
    final container = ProviderContainer(
      overrides: [
        ...setupOverrides(settings: settings),
        if (settingsUnreadable)
          settingsTransportProvider.overrideWith(
            (_) async => throw StateError('settings unavailable'),
          ),
      ],
    );
    addTearDown(container.dispose);
    onContainer?.call(container);
    await pumpSetupPage(tester, container);
  }

  /// Boot the real app tree on top of a stored settings file.
  ///
  /// Settings are preloaded exactly as `main()` does, so frame one already
  /// knows whether setup is done. Without it the first frame builds the mixer
  /// shell, whose notice bridge would toast bridge errors over the wizard.
  Future<FakeSettingsTransport> pumpApp(
    WidgetTester tester, {
    bool onboardingCompleted = false,
    bool motion = false,
    List<DetectedController> controllers = const [],
    String? mixxxPath,
    String? rekordboxPath,
  }) async {
    configureTestSurface(tester, motion: motion);
    final settings = FakeSettingsTransport(
      copyAppSettings(
        defaultAppSettings(),
        onboardingCompleted: onboardingCompleted,
      ),
    );
    final overrides = setupOverrides(
      settings: settings,
      controllers: controllers,
      mixxxPath: mixxxPath,
      rekordboxPath: rekordboxPath,
    );
    final container = ProviderContainer(overrides: overrides);
    addTearDown(container.dispose);
    await preloadAppSettings(container);

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const Application(appTitle: 'Mixar'),
      ),
    );
    // One frame paints the wizard; the page then seeds its draft from the
    // already-resolved settings future. With motion on, settling would never
    // finish, so pump a fixed number of frames instead.
    await tester.pump();
    if (motion) {
      await tester.pump(const Duration(milliseconds: 16));
      await tester.pump(const Duration(milliseconds: 16));
    } else {
      await tester.pumpAndSettle();
    }
    return settings;
  }

  Future<void> tapNext(WidgetTester tester) async {
    await tester.tap(find.byKey(kOnboardingNextKey));
    await tester.pumpAndSettle();
  }

  /// Walk to the appearance step and pick Violet: the step order lives here,
  /// not in every test that needs a non-default accent.
  Future<void> pickVioletAccent(WidgetTester tester) async {
    await tapNext(tester); // Language.
    await tapNext(tester); // Appearance.
    await tester.tap(find.bySemanticsLabel('Violet'));
    await tester.pumpAndSettle();
  }

  /// Press Continue/Finish [times] times: the run's length lives in the flow,
  /// not in each test.
  Future<void> tapNextTimes(WidgetTester tester, int times) async {
    for (var press = 0; press < times; press++) {
      await tapNext(tester);
    }
  }

  testWidgets('the stage animates unless motion is reduced', (tester) async {
    await pumpApp(tester, motion: true);

    expect(
      tester
          .widget<OnboardingBackdrop>(find.byType(OnboardingBackdrop))
          .animate,
      isTrue,
    );
    // A scheduled frame without any input means the ambient stage is ticking.
    expect(
      tester.binding.hasScheduledFrame,
      isTrue,
      reason: 'the backdrop animates on a normal boot',
    );

    // Drop the animated tree so the test ends with no live ticker.
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump();
  });

  testWidgets('a fresh install opens setup instead of the mixer', (
    tester,
  ) async {
    await pumpApp(tester);

    expect(find.byType(OnboardingPage), findsOneWidget);
    expect(find.byType(AppShell), findsNothing);
    expect(find.text('Welcome to Mixar'), findsOneWidget);
  });

  testWidgets('the progress rail keeps its place while pages change', (
    tester,
  ) async {
    await pumpApp(tester);
    final lane = find.byType(OnboardingProgressLane);
    final first = tester.getRect(lane);

    await tapNext(tester);
    await tapNext(tester);
    expect(
      tester.getRect(lane),
      first,
      reason: 'the rail lives outside the step switcher, so it cannot move',
    );
    // It still tracks the step it is on.
    expect(find.bySemanticsLabel('Step 3 of 6'), findsOneWidget);
  });

  testWidgets('without a controller the run is six steps and Start mixing '
      'opens the mixer', (tester) async {
    final settings = await pumpApp(tester);

    for (var step = 1; step <= 6; step++) {
      expect(
        find.bySemanticsLabel('Step $step of 6'),
        findsOneWidget,
        reason: 'step $step of the controller-less run',
      );
      if (step < 6) {
        await tapNext(tester);
      }
    }
    // The last page is the "ready" one, not a controller page.
    expect(find.text("You're all set"), findsOneWidget);
    expect(find.text('Start mixing'), findsOneWidget);

    await tapNext(tester);
    await tester.pumpAndSettle();

    expect(settings.stored.onboardingCompleted, isTrue);
    expect(find.byType(OnboardingPage), findsNothing);
    expect(find.byType(AppShell), findsOneWidget);
  });

  testWidgets('a detected controller adds its setup step and can be trusted', (
    tester,
  ) async {
    final settings = await pumpApp(tester, controllers: const [_ddj]);

    await tapNext(tester); // Language.
    await tapNext(tester); // Appearance.
    await tapNext(tester); // Controller setup.
    expect(find.bySemanticsLabel('Step 4 of 7'), findsOneWidget);
    expect(find.text(_ddjMappingName), findsOneWidget);
    expect(
      find.textContaining('MIDI 1'),
      findsNothing,
      reason: 'the raw ALSA port name is not shown',
    );

    await tester.tap(find.text('Trust'));
    await tester.pumpAndSettle();
    expect(find.text('Trusted'), findsOneWidget);

    // Walk to the end and finish: 4 → … → 7, then the finish tap.
    await tapNextTimes(tester, 4);
    await tester.pumpAndSettle();

    expect(settings.stored.trustedControllerDeviceIds, [_ddjDeviceId]);
    expect(settings.stored.onboardingCompleted, isTrue);
  });

  testWidgets('a port without a mapping does not create the controller step', (
    tester,
  ) async {
    await pumpApp(
      tester,
      controllers: const [
        DetectedController(portName: 'Midi Through:Midi Through Port-0 14:0'),
      ],
    );

    // Six steps, and the controller page is not one of them.
    expect(find.bySemanticsLabel('Step 1 of 6'), findsOneWidget);
    await tapNext(tester); // Language.
    await tapNext(tester); // Appearance.
    await tapNext(tester); // Straight to the library.
    expect(find.textContaining('Midi Through'), findsNothing);
  });

  testWidgets('only libraries found on this machine get an import row', (
    tester,
  ) async {
    await pumpApp(tester, mixxxPath: '/home/someone/.mixxx/mixxxdb.sqlite');

    await tapNext(tester); // Language.
    await tapNext(tester); // Appearance.
    await tapNext(tester); // Library.

    expect(find.byType(MixxxImportPanel), findsOneWidget);
    expect(
      find.byType(RekordboxImportPanel),
      findsNothing,
      reason: 'a library that is not on disk gets no row',
    );
    expect(
      find.textContaining('Rekordbox'),
      findsOneWidget,
      reason: 'the missing library is named once, under the rows',
    );
  });

  testWidgets('one device with several MIDI ports is one controller', (
    tester,
  ) async {
    await pumpApp(
      tester,
      controllers: const [
        DetectedController(portName: 'Midi Through:Midi Through Port-0 14:0'),
        _ddj,
        _ddjPortTwo,
      ],
    );

    await tapNext(tester); // Language.
    await tapNext(tester); // Appearance.
    await tapNext(tester); // Controller setup.

    expect(
      find.textContaining('One controller found'),
      findsOneWidget,
      reason: 'the count is devices, not MIDI ports',
    );
    expect(
      find.text(_ddjMappingName),
      findsOneWidget,
      reason: 'two ports of one device get a single row',
    );
    expect(
      find.textContaining('Midi Through'),
      findsNothing,
      reason: 'an unmapped port never reaches the step',
    );
  });

  testWidgets('the language step sits before appearance and saves its pick', (
    tester,
  ) async {
    final settings = await pumpApp(tester);

    await tapNext(tester); // Welcome → language.
    expect(find.bySemanticsLabel('Step 2 of 6'), findsOneWidget);
    expect(find.text('Pick your language'), findsOneWidget);

    await tester.tap(find.text('Português (Brasil)'));
    await tester.pumpAndSettle();
    // The draft drives the app: the page itself switches language immediately.
    expect(find.text('Escolha o seu idioma'), findsOneWidget);
    expect(find.text('Pick your language'), findsNothing);

    await tapNext(tester); // Language → appearance.
    // Accessibility copy is localized too: the rail now announces Portuguese.
    expect(find.bySemanticsLabel('Passo 3 de 6'), findsOneWidget);

    // Walk to the end and finish with the language saved (3 → … → 6).
    await tapNextTimes(tester, 4);
    await tester.pumpAndSettle();
    expect(settings.stored.uiLanguage, UiLanguageSetting.ptBr);
  });

  testWidgets('an accent picked during setup is what Finish saves', (
    tester,
  ) async {
    final settings = await pumpApp(tester);

    await pickVioletAccent(tester);

    // Two taps put us on appearance; four more reach the ready page and finish.
    await tapNextTimes(tester, 4);
    await tester.pumpAndSettle();

    expect(settings.stored.accentPalette, AccentPaletteSetting.violet);
    expect(settings.stored.onboardingCompleted, isTrue);
  });

  testWidgets('Enter advances the flow and the theme tile previews live', (
    tester,
  ) async {
    final settings = await pumpApp(tester);

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    expect(find.bySemanticsLabel('Step 2 of 6'), findsOneWidget);

    await tapNext(tester); // Language → appearance.
    expect(find.bySemanticsLabel('Step 3 of 6'), findsOneWidget);

    await tester.tap(find.bySemanticsLabel('Dark'));
    await tester.pumpAndSettle();
    // The draft, not the saved file, drives the window: it is already dark.
    expect(
      tester.widget<MaterialApp>(find.byType(MaterialApp)).themeMode,
      ThemeMode.dark,
    );

    await tapNext(tester);
    expect(find.bySemanticsLabel('Step 4 of 6'), findsOneWidget);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await tester.pumpAndSettle();
    expect(find.bySemanticsLabel('Step 3 of 6'), findsOneWidget);

    await tapNextTimes(tester, 4);
    await tester.pumpAndSettle();
    expect(find.byType(AppShell), findsOneWidget);
    expect(settings.stored.theme, ThemeSetting.dark);
  });

  testWidgets('Skip marks setup done and drops the step edits', (tester) async {
    final settings = await pumpApp(tester);

    await pickVioletAccent(tester);
    expect(
      find.bySemanticsLabel('Violet'),
      findsOneWidget,
      reason: 'the edit must preview before Skip is pressed',
    );

    await tester.tap(find.byKey(kOnboardingSkipKey));
    await tester.pumpAndSettle();

    expect(settings.stored.onboardingCompleted, isTrue);
    expect(settings.stored.accentPalette, AccentPaletteSetting.teal);
    expect(find.byType(OnboardingPage), findsNothing);
    expect(find.byType(AppShell), findsOneWidget);
  });

  testWidgets('An unreadable settings file still starts setup', (tester) async {
    // No preload: the page's own seed is the read that fails.
    await pumpSetup(tester, settingsUnreadable: true);

    expect(
      find.text('Welcome to Mixar'),
      findsOneWidget,
      reason: 'a failed load falls back to defaults instead of a spinner',
    );
    expect(find.byKey(kOnboardingNextKey), findsOneWidget);
  });

  testWidgets('Skip reports a failing settings read instead of throwing', (
    tester,
  ) async {
    await pumpSetup(tester, settingsUnreadable: true);

    await tester.tap(find.byKey(kOnboardingSkipKey));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.byType(OnboardingPage), findsOneWidget, reason: 'still stuck');
  });

  testWidgets('A failed save leaves the flow usable', (tester) async {
    await pumpSetup(
      tester,
      settings: FakeSettingsTransport(
        defaultAppSettings(),
        saveError: Exception('disk full'),
      ),
    );

    // Walk to the last step and press Finish; the save throws.
    await tapNextTimes(tester, 6);
    await tester.pumpAndSettle();

    expect(find.byType(OnboardingPage), findsOneWidget);
    // Skip is hidden on the last step, so Finish is the control that proves
    // the page is out of its busy lock.
    expect(
      tester.widget<AppButton>(find.byKey(kOnboardingNextKey)).onPress,
      isNotNull,
      reason: 'a failed save must not leave every control disabled',
    );
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await tester.pumpAndSettle();
    expect(find.bySemanticsLabel('Step 5 of 6'), findsOneWidget);
  });

  testWidgets('Finishing leaves no preview draft behind for a replay', (
    tester,
  ) async {
    late ProviderContainer container;
    final settings = FakeSettingsTransport(defaultAppSettings());
    await pumpSetup(
      tester,
      settings: settings,
      onContainer: (value) => container = value,
    );

    await tapNextTimes(tester, 6);
    await tester.pumpAndSettle();

    expect(settings.stored.onboardingCompleted, isTrue);
    // Settings → UI → Show setup again flips the flag and re-opens the wizard;
    // a draft left here would preview itself over the settings it is editing.
    expect(container.read(onboardingDraftProvider), isNull);
  });
}
