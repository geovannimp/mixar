import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_session_panel.dart';
import 'package:gui_flutter/shell/mixar_switch.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('history fields show when enabled and hide when disabled', (
    tester,
  ) async {
    var draft = defaultAppSettings();
    expect(draft.historyEnabled, isTrue);

    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: SingleChildScrollView(
            child: StatefulBuilder(
              builder: (context, setState) {
                return SettingsSessionPanel(
                  draft: draft,
                  onChanged: (next) => setState(() => draft = next),
                );
              },
            ),
          ),
        ),
      ),
    );

    expect(find.text('Performance history'), findsOneWidget);
    expect(find.text('SESSION IDLE TIMEOUT'), findsOneWidget);
    expect(find.text('MINIMUM PLAY DURATION'), findsOneWidget);
    expect(find.text('MINIMUM EFFECTIVE DECK VOLUME'), findsOneWidget);

    await tester.tap(find.byType(MixarSwitch));
    await tester.pumpAndSettle();

    expect(draft.historyEnabled, isFalse);
    expect(find.text('SESSION IDLE TIMEOUT'), findsNothing);
    expect(find.text('MINIMUM PLAY DURATION'), findsNothing);
    expect(find.text('MINIMUM EFFECTIVE DECK VOLUME'), findsNothing);

    await tester.tap(find.byType(MixarSwitch));
    await tester.pumpAndSettle();

    expect(draft.historyEnabled, isTrue);
    expect(find.text('SESSION IDLE TIMEOUT'), findsOneWidget);
    expect(find.text('MINIMUM PLAY DURATION'), findsOneWidget);
    expect(find.text('MINIMUM EFFECTIVE DECK VOLUME'), findsOneWidget);
  }, semanticsEnabled: false);
}
