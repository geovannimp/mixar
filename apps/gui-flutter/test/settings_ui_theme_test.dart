import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    AppSettings? draft,
    ValueChanged<AppSettings>? onChanged,
  }) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: draft ?? defaultAppSettings(),
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('UI panel offers the theme select, defaulting to system', (
    tester,
  ) async {
    await pumpPanel(tester);
    expect(find.text('THEME'), findsOneWidget);
    final select = find.byType(MixarSelect<ThemeSetting>);
    expect(select, findsOneWidget);
    expect(
      find.descendant(of: select, matching: find.text('System default')),
      findsOneWidget,
    );
  }, semanticsEnabled: false);

  testWidgets('picking Dark emits the theme setting', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (settings) => changed = settings);

    await tester.tap(find.byType(MixarSelect<ThemeSetting>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Dark').last);
    await tester.pumpAndSettle();

    expect(changed?.theme, ThemeSetting.dark);
  }, semanticsEnabled: false);
}
