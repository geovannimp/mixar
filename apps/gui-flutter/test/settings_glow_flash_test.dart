import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('glow overlay appears then fades when active', (tester) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: SettingsGlowFlash(
            active: true,
            child: SizedBox(width: 120, height: 80, child: Text('card')),
          ),
        ),
      ),
    );
    // Post-frame starts the controller; first tick shows overlay.
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));
    expect(find.byType(DecoratedBox), findsWidgets);
    expect(find.text('card'), findsOneWidget);

    await tester.pump(const Duration(milliseconds: 1000));
    // Completed flash drops the overlay Stack and returns the bare child.
    expect(find.text('card'), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(SettingsGlowFlash),
        matching: find.byType(Stack),
      ),
      findsNothing,
    );
  }, semanticsEnabled: false);

  testWidgets('replays on false→true and stops on true→false', (tester) async {
    var active = false;
    late StateSetter setActive;
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              setActive = setState;
              return SettingsGlowFlash(
                active: active,
                child: const SizedBox(
                  width: 120,
                  height: 80,
                  child: Text('card'),
                ),
              );
            },
          ),
        ),
      ),
    );
    await tester.pump();
    expect(
      find.descendant(
        of: find.byType(SettingsGlowFlash),
        matching: find.byType(Stack),
      ),
      findsNothing,
    );

    setActive(() => active = true);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));
    expect(
      find.descendant(
        of: find.byType(SettingsGlowFlash),
        matching: find.byType(Stack),
      ),
      findsOneWidget,
    );

    setActive(() => active = false);
    await tester.pump();
    expect(
      find.descendant(
        of: find.byType(SettingsGlowFlash),
        matching: find.byType(Stack),
      ),
      findsNothing,
    );
  }, semanticsEnabled: false);
}
