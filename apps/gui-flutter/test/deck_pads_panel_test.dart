import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    required bool hasTrack,
    bool disabled = false,
  }) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: (context, child) => MaterialUiCompatibilityBridge( // ignore: deprecated_member_use
          child: FTheme(data: theme, child: child!),
        ),
        home: Scaffold(
          body: SizedBox(
            width: 360,
            height: 320,
            child: DeckPadsPanel(hasTrack: hasTrack, disabled: disabled),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('mode tabs switch grids', (tester) async {
    await pumpPanel(tester, hasTrack: true);

    expect(find.text('CUE'), findsOneWidget);
    expect(find.text('0:12.5'), findsOneWidget); // demo hot cue

    await tester.tap(find.text('JUMP'));
    await tester.pumpAndSettle();
    expect(find.text('+1'), findsOneWidget);
    expect(find.text('0:12.5'), findsNothing);

    await tester.tap(find.text('SAMPLE'));
    await tester.pumpAndSettle();
    expect(find.text('Bank 1'), findsOneWidget);
    expect(find.text('Kick'), findsOneWidget);
  });

  testWidgets('hot cue save updates pad label time', (tester) async {
    await pumpPanel(tester, hasTrack: true);

    await tester.tap(find.text('2'));
    await tester.pumpAndSettle();
    expect(find.text('0:01.0'), findsOneWidget);
  });

  testWidgets('sampler bank next cycles active bank', (tester) async {
    await pumpPanel(tester, hasTrack: true);
    await tester.tap(find.text('SAMPLE'));
    await tester.pumpAndSettle();

    await tester.tap(find.bySemanticsLabel('Next sampler bank'));
    await tester.pumpAndSettle();
    expect(find.text('Bank 2'), findsOneWidget);
    expect(find.text('hold'), findsOneWidget);
  });

  testWidgets('pad actions stay disabled without a track', (tester) async {
    await pumpPanel(tester, hasTrack: false);

    await tester.tap(find.text('2'));
    await tester.pumpAndSettle();
    expect(find.text('0:01.0'), findsNothing);

    await tester.tap(find.text('ROLL'));
    await tester.pumpAndSettle();
    expect(find.text('roll'), findsWidgets);
  });

  testWidgets('disabled panel blocks mode tabs and pad actions', (tester) async {
    await pumpPanel(tester, hasTrack: true, disabled: true);

    await tester.tap(find.text('JUMP'));
    await tester.pumpAndSettle();
    expect(find.text('+1'), findsNothing);
    expect(find.text('0:12.5'), findsOneWidget);

    await tester.tap(find.text('2'));
    await tester.pumpAndSettle();
    expect(find.text('0:01.0'), findsNothing);
  });
}
