import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/settings_page.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('opening the preview bus scrolls its last fields into view', (
    tester,
  ) async {
    // Short window: the collapsed card sits at the end of the page, so the
    // body opens entirely below the fold.
    const windowHeight = 430.0;
    await tester.binding.setSurfaceSize(const Size(1200, windowHeight));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    final baseline = copyAppSettings(
      defaultAppSettings(),
      previewEnabled: false,
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => baseline),
          audioDevicesProvider.overrideWith(
            (ref) async => const <OutputDevice>[],
          ),
          samplerBanksProvider.overrideWith((ref) async => const []),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(body: SettingsPage(onClose: () {})),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final scrollable = find.descendant(
      of: find.byType(SingleChildScrollView),
      matching: find.byType(Scrollable),
    );
    final position = tester.state<ScrollableState>(scrollable.first).position;
    position.jumpTo(position.maxScrollExtent);
    await tester.pumpAndSettle();
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
