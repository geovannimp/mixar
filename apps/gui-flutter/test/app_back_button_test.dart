import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/shell/app_back_button.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

import 'support/mixar_material_app.dart';

void main() {
  /// Pumps the button in a left-aligned slot and hands back a setter, so a test
  /// can flip [visible] the way the shell does on a page change.
  Future<({void Function(bool) setVisible, int Function() presses})> pumpSlot(
    WidgetTester tester, {
    required bool visible,
    bool reduceMotion = false,
  }) async {
    late StateSetter setHost;
    var presses = 0;
    var shown = visible;
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: MediaQuery(
          data: MediaQueryData(disableAnimations: reduceMotion),
          child: Align(
            alignment: Alignment.centerLeft,
            child: StatefulBuilder(
              builder: (context, setState) {
                setHost = setState;
                return AppBackButton(visible: shown, onPress: () => presses++);
              },
            ),
          ),
        ),
      ),
    );
    return (
      setVisible: (bool value) => setHost(() => shown = value),
      presses: () => presses,
    );
  }

  Finder slotOpacity() => find
      .descendant(
        of: find.byType(AppBackButton),
        matching: find.byType(Opacity),
      )
      .first;

  testWidgets('it names the booth and reports a press', (tester) async {
    final host = await pumpSlot(tester, visible: true);

    expect(find.text('Booth'), findsOneWidget);
    expect(find.byIcon(LucideIcons.arrowLeft), findsOneWidget);

    await tester.tap(find.byType(AppButton));
    await tester.pump();

    expect(host.presses(), 1);
    expect(
      find.byType(AppButton),
      findsOneWidget,
      reason: 'a press alone does not take the control away',
    );
  });

  testWidgets('it keeps the flat titlebar radius', (tester) async {
    await pumpSlot(tester, visible: true);

    final surface =
        tester
                .widgetList<DecoratedBox>(
                  find.descendant(
                    of: find.byType(AppButton),
                    matching: find.byType(DecoratedBox),
                  ),
                )
                .last
                .decoration
            as BoxDecoration;
    expect(surface.borderRadius, MixarThemeData.dark().style.borderRadius.md);
  });

  testWidgets('it slides left and fades out when the page changes', (
    tester,
  ) async {
    final host = await pumpSlot(tester, visible: true);
    final before = tester.getTopLeft(find.byType(AppButton)).dx;

    host.setVisible(false);
    await tester.pump();
    await tester.pump(kAppBackExitDuration ~/ 3 * 2);

    final opacity = tester.widget<Opacity>(slotOpacity());
    expect(opacity.opacity, lessThan(0.9));
    expect(opacity.opacity, greaterThan(0.2));
    expect(
      tester.getTopLeft(find.byType(AppButton)).dx,
      lessThan(before - 2),
      reason: 'the exit travels left, out of the leading corner',
    );

    await tester.pumpAndSettle();
    expect(find.byType(AppButton), findsNothing);
  });

  testWidgets('it returns at once when the page comes back', (tester) async {
    final host = await pumpSlot(tester, visible: false);
    expect(find.byType(AppButton), findsNothing);

    host.setVisible(true);
    await tester.pump();

    expect(find.byType(AppButton), findsOneWidget);
    expect(tester.widget<Opacity>(slotOpacity()).opacity, 1);
  });

  testWidgets('an exiting button takes no press', (tester) async {
    final host = await pumpSlot(tester, visible: true);

    host.setVisible(false);
    await tester.pump();

    // The exit starts on this frame; the painted button must already be inert.
    await tester.tap(find.byType(AppButton), warnIfMissed: false);
    expect(host.presses(), 0);

    await tester.pump(kAppBackExitDuration ~/ 2);
    await tester.tap(find.byType(AppButton), warnIfMissed: false);
    expect(host.presses(), 0);
  });

  testWidgets('coming back mid-exit leaves the button up', (tester) async {
    final host = await pumpSlot(tester, visible: true);

    host.setVisible(false);
    await tester.pump();
    await tester.pump(kAppBackExitDuration ~/ 2);

    host.setVisible(true);
    await tester.pump();

    // The cancelled exit completes its future; it must not clear the row the
    // re-show just put back.
    await tester.pump(kAppBackExitDuration);
    expect(find.byType(AppButton), findsOneWidget);
    expect(tester.widget<Opacity>(slotOpacity()).opacity, 1);
  });

  testWidgets('reduced motion empties the slot in the same frame', (
    tester,
  ) async {
    final host = await pumpSlot(tester, visible: true, reduceMotion: true);

    host.setVisible(false);
    await tester.pump();

    expect(find.byType(AppButton), findsNothing);
  });
}
