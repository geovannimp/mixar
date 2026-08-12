import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';

void main() {
  Future<void> pumpPanel(WidgetTester tester, {required bool hasTrack}) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: theme.toApproximateMaterialTheme(),
        builder: (context, child) => FTheme(data: theme, child: child!),
        home: Scaffold(
          body: SizedBox(
            width: 360,
            height: 320,
            child: DeckPadsPanel(hasTrack: hasTrack),
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
}
