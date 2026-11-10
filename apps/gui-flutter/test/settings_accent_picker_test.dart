import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/mixar_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_accent_picker.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_visual_choice.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
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

  testWidgets('accent picker lists all five palettes in one row, teal marked', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    await pumpPanel(tester);
    await tester.pump();

    expect(find.text('Accent color'), findsOneWidget);
    final choice = find.byType(SettingsVisualChoice<AccentPaletteSetting>);
    expect(choice, findsOneWidget);
    for (final label in ['Teal', 'Blue', 'Violet', 'Orange', 'Pink']) {
      expect(
        find.descendant(of: choice, matching: find.text(label)),
        findsOneWidget,
        reason: label,
      );
    }
    expect(find.text('Emerald'), findsNothing);

    // One row: every swatch centre shares a baseline.
    final centres = {
      for (final label in ['Teal', 'Blue', 'Violet', 'Orange', 'Pink'])
        tester.getCenter(find.bySemanticsLabel(label)).dy.round(),
    };
    expect(centres, hasLength(1));

    // One node per cell: the visible label is excluded from semantics, so the
    // node label is exactly the palette name (no duplicate announcement).
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
      of: find.byType(SettingsVisualChoice<AccentPaletteSetting>),
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

  for (final width in [200.0, 140.0, 96.0, 64.0]) {
    testWidgets('swatches shrink instead of overflowing at ${width}px', (
      tester,
    ) async {
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

      expect(tester.takeException(), isNull);
      final swatch = tester.getSize(
        find
            .descendant(
              of: find.byType(SettingsAccentPicker),
              matching: find.byType(AspectRatio),
            )
            .first,
      );
      expect(swatch.width, lessThanOrEqualTo(22));
      expect(swatch.width, swatch.height);
      expect(swatch.width, greaterThan(0));
    }, semanticsEnabled: false);
  }

  testWidgets('a focused cell shows the selection ring', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SettingsAccentPicker(
              value: AccentPaletteSetting.teal,
              onChanged: (_) {},
            ),
          ),
        ),
      ),
    );

    final cells = find.descendant(
      of: find.byType(SettingsAccentPicker),
      matching: find.byType(MTappable),
    );
    Border ringOf(int index) {
      final box = tester.widget<DecoratedBox>(
        find
            .descendant(
              of: cells.at(index),
              matching: find.byType(DecoratedBox),
            )
            .last,
      );
      return (box.decoration as BoxDecoration).border! as Border;
    }

    // Teal is selected, so it already carries the ring; the second tab lands on
    // Blue, which is unselected and must gain one from focus alone.
    expect(ringOf(0).top.width, 2);
    expect(ringOf(1).top.width, MixarStyle().borderWidth);

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();

    final focused = ringOf(1);
    expect(focused.top.width, 2);
    expect(focused.top.color, MixarThemeData.dark().colors.foreground);
  }, semanticsEnabled: false);

  testWidgets('hovering a cell paints the shared hover wash', (tester) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: theme,
          home: Scaffold(
            body: SettingsAccentPicker(
              value: AccentPaletteSetting.teal,
              onChanged: (_) {},
            ),
          ),
        ),
      ),
    );

    final cell = find
        .descendant(
          of: find.byType(SettingsAccentPicker),
          matching: find.byType(MTappable),
        )
        .first;
    final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: Offset.zero);
    addTearDown(gesture.removePointer);
    await tester.pump();

    await gesture.moveTo(tester.getCenter(cell));
    await tester.pumpAndSettle();

    final box = tester.widget<DecoratedBox>(
      find.descendant(of: cell, matching: find.byType(DecoratedBox)).first,
    );
    expect((box.decoration as BoxDecoration).color, theme.hoverWash);
  });
}
