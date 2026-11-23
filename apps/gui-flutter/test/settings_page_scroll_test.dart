import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/settings_page.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_sidebar.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

/// Pumps the settings page in a short window, so the pane is always taller than
/// one section and a carried-over scroll offset would be visible.
Future<void> pumpShortWindow(
  WidgetTester tester, {
  required double height,
  AppSettings? settings,
  SettingsSection? initialSection,
  List<Override> extraOverrides = const [],
}) async {
  await tester.binding.setSurfaceSize(Size(1200, height));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        appSettingsProvider.overrideWith(
          (ref) async => settings ?? defaultAppSettings(),
        ),
        audioDevicesProvider.overrideWith(
          (ref) async => const <OutputDevice>[],
        ),
        ...extraOverrides,
      ],
      child: mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          // UI is the page's own default landing section.
          body: SettingsPage(
            initialSection: initialSection ?? SettingsSection.ui,
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

/// The settings content pane's scroll position.
ScrollPosition contentScrollPosition(WidgetTester tester) {
  final scrollable = find.descendant(
    of: find.byKey(kSettingsContentScrollKey),
    matching: find.byType(Scrollable),
  );
  return tester.state<ScrollableState>(scrollable.first).position;
}

/// Scrolls the pane to its end and answers the position it landed on.
Future<ScrollPosition> jumpContentToEnd(WidgetTester tester) async {
  final position = contentScrollPosition(tester);
  position.jumpTo(position.maxScrollExtent);
  await tester.pumpAndSettle();
  return position;
}

void main() {
  testWidgets('a section change opens the new section at its top', (
    tester,
  ) async {
    await pumpShortWindow(tester, height: 480);
    final position = await jumpContentToEnd(tester);
    expect(position.maxScrollExtent, greaterThan(0));
    expect(position.pixels, greaterThan(0));

    await tester.tap(
      find.descendant(
        of: find.byType(SettingsSidebar),
        matching: find.text('Layout'),
      ),
    );
    await tester.pumpAndSettle();

    // Layout is long enough to scroll on its own, so a zero offset here is the
    // reset and not a clamp against a shorter panel.
    expect(position.maxScrollExtent, greaterThan(0));
    expect(position.pixels, 0);
    expect(
      tester.getRect(find.byType(SettingsSectionCard).first).top,
      greaterThan(
        tester.getRect(find.byKey(kSettingsContentScrollKey)).top + 44,
      ),
      reason: 'the first card sits below the section header, not above it',
    );
  }, semanticsEnabled: false);

  testWidgets('opening the preview bus scrolls its last fields into view', (
    tester,
  ) async {
    // Short window: the collapsed card sits at the end of the page, so the
    // body opens entirely below the fold.
    const windowHeight = 430.0;
    await pumpShortWindow(
      tester,
      height: windowHeight,
      settings: copyAppSettings(defaultAppSettings(), previewEnabled: false),
      initialSection: SettingsSection.audio,
      extraOverrides: [
        samplerBanksProvider.overrideWith((ref) async => const []),
      ],
    );
    final position = await jumpContentToEnd(tester);
    final collapsedOffset = position.pixels;

    await tester.tap(find.byType(SettingsHeaderSwitch));
    await tester.pumpAndSettle();

    expect(
      tester
          .widget<SettingsHeaderSwitch>(find.byType(SettingsHeaderSwitch))
          .value,
      isTrue,
    );
    expect(
      position.pixels,
      greaterThan(collapsedOffset),
      reason: 'the page must scroll to reach the opened fields',
    );
    // The routing fields end with the channel steppers; the pane keeps them
    // clear of its bottom edge instead of cropping them.
    final previewCard = find.ancestor(
      of: find.text('Preview bus'),
      matching: find.byType(SettingsSectionCard),
    );
    final rightChannelStepper = find.descendant(
      of: previewCard,
      matching: find.byType(AppButton),
    );
    expect(rightChannelStepper, findsWidgets);
    expect(
      tester.getRect(rightChannelStepper.last).bottom,
      lessThanOrEqualTo(windowHeight - 16),
    );
  }, semanticsEnabled: false);
}
