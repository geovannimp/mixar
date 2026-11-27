import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_accent_picker.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  const labels = ['Teal', 'Blue', 'Violet', 'Orange', 'Pink'];

  Future<void> pumpPanel(
    WidgetTester tester, {
    AppSettings? draft,
    ValueChanged<AppSettings>? onChanged,
  }) async {
    final settings = draft ?? defaultAppSettings();
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: settings,
                savedAccentPalette: settings.accentPalette,
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
  }

  Future<void> pumpPicker(WidgetTester tester, {double? width}) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: width,
                child: SettingsAccentPicker(
                  value: AccentPaletteSetting.teal,
                  onChanged: (_) {},
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('accent picker is one rounded row of five palettes', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    await pumpPanel(tester);
    await tester.pump();

    expect(find.text('Accent color'), findsOneWidget);
    final picker = find.byType(SettingsAccentPicker);
    expect(picker, findsOneWidget);
    for (final label in labels) {
      expect(
        find.descendant(of: picker, matching: find.text(label)),
        findsOneWidget,
        reason: label,
      );
    }
    expect(find.text('Emerald'), findsNothing);

    // The whole group is one track: a pill border around a single row.
    final track = tester.widget<DecoratedBox>(
      find.descendant(of: picker, matching: find.byType(DecoratedBox)).first,
    );
    final decoration = track.decoration as BoxDecoration;
    expect(
      decoration.borderRadius,
      MixarThemeData.dark().style.borderRadius.pill,
    );
    expect(decoration.border, isNotNull);

    // One row: every segment centre shares a baseline.
    final centres = {
      for (final label in labels)
        tester.getCenter(find.bySemanticsLabel(label)).dy.round(),
    };
    expect(centres, hasLength(1));

    // One node per segment: the visible label is excluded from semantics, so
    // the node label is exactly the palette name (no duplicate announcement).
    expect(
      tester.getSemantics(find.bySemanticsLabel('Teal')),
      isSemantics(label: 'Teal', isSelected: true, isButton: true),
    );
    expect(
      tester.getSemantics(find.bySemanticsLabel('Violet')),
      isSemantics(label: 'Violet', isSelected: false, isButton: true),
    );
    handle.dispose();
  });

  testWidgets('tapping Violet emits the palette and leaves other fields', (
    tester,
  ) async {
    final baseline = defaultAppSettings();
    AppSettings? changed;
    await pumpPanel(
      tester,
      draft: baseline,
      onChanged: (settings) => changed = settings,
    );

    final violet = find.descendant(
      of: find.byType(SettingsAccentPicker),
      matching: find.text('Violet'),
    );
    await tester.ensureVisible(violet);
    await tester.pumpAndSettle();
    await tester.tap(violet);
    await tester.pump();

    expect(changed?.accentPalette, AccentPaletteSetting.violet);
    expect(changed?.theme, baseline.theme);
    expect(changed?.libraryRowFields, baseline.libraryRowFields);
    expect(changed?.showTooltips, baseline.showTooltips);
  }, semanticsEnabled: false);

  for (final width in [560.0, 240.0, 96.0, 64.0]) {
    testWidgets('segments stay inside ${width}px without overflowing', (
      tester,
    ) async {
      await pumpPicker(tester, width: width);

      expect(tester.takeException(), isNull);
      expect(tester.getSize(find.byType(SettingsAccentPicker)).width, width);
      for (final label in labels) {
        expect(find.text(label), findsOneWidget, reason: label);
      }
    }, semanticsEnabled: false);
  }

  testWidgets('a focused segment shows the focus ring', (tester) async {
    await pumpPicker(tester);

    final cells = find.descendant(
      of: find.byType(SettingsAccentPicker),
      matching: find.byType(MTappable),
    );
    Border? ringOf(int index) {
      final box = tester.widget<DecoratedBox>(
        find
            .descendant(
              of: cells.at(index),
              matching: find.byType(DecoratedBox),
            )
            .first,
      );
      return (box.decoration as BoxDecoration).border as Border?;
    }

    expect(ringOf(0), isNull);

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();

    final focused = ringOf(0);
    expect(focused, isNotNull);
    expect(focused!.top.width, 1.5);
    expect(focused.top.color, MixarThemeData.dark().colors.foreground);
  }, semanticsEnabled: false);

  testWidgets('hovering a segment paints the shared hover wash', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpPicker(tester);

    // Blue is unselected, so the wash reads on its own rather than under the
    // selected palette's tint.
    final cell = find
        .descendant(
          of: find.byType(SettingsAccentPicker),
          matching: find.byType(MTappable),
        )
        .at(1);
    final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: Offset.zero);
    addTearDown(gesture.removePointer);
    await tester.pump();

    await gesture.moveTo(tester.getCenter(cell));
    await tester.pumpAndSettle();

    final box = tester.widget<DecoratedBox>(
      find.descendant(of: cell, matching: find.byType(DecoratedBox)).at(1),
    );
    expect((box.decoration as BoxDecoration).color, theme.hoverWash);
  });
}
