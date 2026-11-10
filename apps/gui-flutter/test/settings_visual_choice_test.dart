import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/settings/settings_visual_choice.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpChoice(
    WidgetTester tester, {
    required String value,
    required ValueChanged<String> onChanged,
    Axis axis = Axis.vertical,
    double? previewAspectRatio,
  }) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: SettingsVisualChoice<String>(
            value: value,
            axis: axis,
            previewAspectRatio: previewAspectRatio,
            options: const [
              SettingsVisualOption<String>(
                value: 'alpha',
                label: 'Alpha',
                subtitle: 'First option',
                preview: SizedBox(key: ValueKey('preview-alpha')),
              ),
              SettingsVisualOption<String>(
                value: 'beta',
                label: 'Beta',
                preview: SizedBox(key: ValueKey('preview-beta')),
              ),
            ],
            onChanged: onChanged,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('renders every label, subtitle, and preview', (tester) async {
    await pumpChoice(tester, value: 'alpha', onChanged: (_) {});

    expect(find.text('Alpha'), findsOneWidget);
    expect(find.text('First option'), findsOneWidget);
    expect(find.text('Beta'), findsOneWidget);
    expect(find.byKey(const ValueKey('preview-alpha')), findsOneWidget);
    expect(find.byKey(const ValueKey('preview-beta')), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('marks only the row whose value is selected', (tester) async {
    await pumpChoice(tester, value: 'beta', onChanged: (_) {});

    final rows = tester.widgetList<MTappable>(find.byType(MTappable)).toList();
    expect(rows, hasLength(2));
    expect(rows[0].selected, isFalse);
    expect(rows[1].selected, isTrue);
  }, semanticsEnabled: false);

  testWidgets('tapping an unselected row reports its value', (tester) async {
    String? changed;
    await pumpChoice(tester, value: 'alpha', onChanged: (v) => changed = v);

    await tester.tap(
      find.descendant(
        of: find.byType(SettingsVisualChoice<String>),
        matching: find.text('Beta'),
      ),
    );
    await tester.pumpAndSettle();

    expect(changed, 'beta');
  }, semanticsEnabled: false);

  testWidgets('horizontal tiles put the preview above the label', (
    tester,
  ) async {
    await pumpChoice(
      tester,
      value: 'alpha',
      onChanged: (_) {},
      axis: Axis.horizontal,
    );

    final alphaPreview = tester.getRect(
      find.byKey(const ValueKey('preview-alpha')),
    );
    final alphaLabel = tester.getRect(find.text('Alpha'));
    final betaLabel = tester.getRect(find.text('Beta'));

    expect(alphaPreview.bottom, lessThanOrEqualTo(alphaLabel.top));
    expect(alphaLabel.center.dx, lessThan(betaLabel.center.dx));
  }, semanticsEnabled: false);

  testWidgets('an explicit preview ratio sizes the preview box', (
    tester,
  ) async {
    await pumpChoice(
      tester,
      value: 'alpha',
      onChanged: (_) {},
      previewAspectRatio: 16 / 9,
    );

    final preview = tester.getRect(find.byKey(const ValueKey('preview-alpha')));
    expect(preview.height, 28);
    expect(preview.width, closeTo(28 * 16 / 9, 0.01));
  }, semanticsEnabled: false);
}
