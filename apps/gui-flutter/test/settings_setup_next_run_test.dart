import 'dart:async';

import 'package:flutter/foundation.dart'
    show debugDefaultTargetPlatformOverride;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/onboarding/onboarding_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/screens/app_shell.dart';
import 'package:gui_flutter/screens/application.dart';
import 'package:gui_flutter/screens/onboarding_page.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';

import 'support/fake_settings_transport.dart';

void main() {
  setUp(() {
    debugOverrideDesktopWindow = false;
    debugOverrideMacOSChromePreview = false;
  });
  tearDown(() {
    debugOverrideDesktopWindow = null;
    debugOverrideMacOSChromePreview = null;
    debugDefaultTargetPlatformOverride = null;
  });

  void configureTestSurface(WidgetTester tester) {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    tester.platformDispatcher.accessibilityFeaturesTestValue =
        const FakeAccessibilityFeatures(disableAnimations: true);
    addTearDown(tester.platformDispatcher.clearAccessibilityFeaturesTestValue);
  }

  List<Override> appOverrides(FakeSettingsTransport settings) => [
    settingsTransportProvider.overrideWith((_) async => settings),
    audioDevicesProvider.overrideWith((_) async => const []),
    samplerBanksProvider.overrideWith((_) async => const []),
    controllerTransportProvider.overrideWith((_) async => null),
    onboardingControllersProvider.overrideWith((_) async => const []),
  ];

  /// A saved profile with setup done, or scheduled for the next launch.
  FakeSettingsTransport savedProfile({required bool setupDone}) =>
      FakeSettingsTransport(
        copyAppSettings(defaultAppSettings(), onboardingCompleted: setupDone),
      );

  /// Boots the app on [settings], resolving them exactly as `main()` does
  /// before the first frame.
  Future<ProviderContainer> pumpLaunch(
    WidgetTester tester,
    FakeSettingsTransport settings,
  ) async {
    configureTestSurface(tester);
    final container = ProviderContainer(overrides: appOverrides(settings));
    addTearDown(container.dispose);
    await preloadAppSettings(container);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const Application(appTitle: 'Mixar'),
      ),
    );
    await tester.pumpAndSettle();
    return container;
  }

  testWidgets(
    'saving the replay switch keeps the mixer until the next launch',
    (tester) async {
      final settings = savedProfile(setupDone: true);
      final container = await pumpLaunch(tester, settings);
      expect(find.byType(AppShell), findsOneWidget);

      container
          .read(pendingSettingsDeepLinkProvider.notifier)
          .request(const SettingsDeepLink());
      await tester.pumpAndSettle();

      final l10n = lookupAppLocalizations(const Locale('en'));
      expect(
        tester
            .widget<SettingsHeaderSwitch>(find.byType(SettingsHeaderSwitch))
            .value,
        isFalse,
      );
      await tester.tap(find.byType(SettingsHeaderSwitch));
      await tester.pumpAndSettle();

      await tester.tap(find.text(l10n.settingsSaveChanges));
      await tester.pumpAndSettle();

      expect(settings.stored.onboardingCompleted, isFalse);
      expect(
        find.text(l10n.settingsSaveChanges),
        findsNothing,
        reason: 'the save landed, so the draft is clean again',
      );
      expect(
        find.byType(OnboardingPage),
        findsNothing,
        reason: 'setup is scheduled for the next launch, not this one',
      );
      expect(find.byType(AppShell), findsOneWidget);
    },
  );

  testWidgets('a launch on the scheduled flag opens setup', (tester) async {
    await pumpLaunch(tester, savedProfile(setupDone: false));

    expect(find.byType(OnboardingPage), findsOneWidget);
  });

  testWidgets('a slow settings read still hands the launch to setup', (
    tester,
  ) async {
    configureTestSurface(tester);
    final gate = Completer<void>();
    final settings = FakeSettingsTransport(
      copyAppSettings(defaultAppSettings(), onboardingCompleted: false),
      ready: gate.future,
    );
    final container = ProviderContainer(overrides: appOverrides(settings));
    addTearDown(container.dispose);

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const Application(appTitle: 'Mixar'),
      ),
    );
    await tester.pump();
    expect(
      find.byType(AppShell),
      findsOneWidget,
      reason: 'unresolved settings keep the mixer, like a preload timeout',
    );

    gate.complete();
    await tester.pumpAndSettle();

    expect(
      find.byType(OnboardingPage),
      findsOneWidget,
      reason: 'the read landed and setup was never done',
    );
  });
}
