import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_performance_panel.dart';

void main() {
  Future<void> pumpPanel(WidgetTester tester) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: theme.toApproximateMaterialTheme(),
        builder: (context, child) => FTheme(data: theme, child: child!),
        home: Scaffold(
          body: const SizedBox(
            width: 360,
            height: 320,
            child: DeckPerformancePanel(hasTrack: true),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('Pads / Loop / Grid / Jog tabs switch exclusive content', (
    tester,
  ) async {
    await pumpPanel(tester);

    expect(find.byIcon(FLucideIcons.layoutGrid), findsOneWidget);
    expect(find.byIcon(FLucideIcons.repeat2), findsOneWidget);
    expect(find.byIcon(FLucideIcons.audioLines), findsOneWidget);
    expect(find.byIcon(FLucideIcons.disc3), findsOneWidget);
    expect(
      tester.getCenter(find.byIcon(FLucideIcons.layoutGrid)).dy <
          tester.getCenter(find.byIcon(FLucideIcons.repeat2)).dy,
      isTrue,
    );
    expect(
      tester.getCenter(find.byIcon(FLucideIcons.repeat2)).dy <
          tester.getCenter(find.byIcon(FLucideIcons.audioLines)).dy,
      isTrue,
    );
    expect(
      tester.getCenter(find.byIcon(FLucideIcons.audioLines)).dy <
          tester.getCenter(find.byIcon(FLucideIcons.disc3)).dy,
      isTrue,
    );
    expect(find.text('CUE'), findsOneWidget);
    expect(find.text('IN'), findsNothing);
    expect(find.text('Beat 1'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);

    await tester.tap(find.byIcon(FLucideIcons.repeat2));
    await tester.pumpAndSettle();
    expect(find.text('IN'), findsOneWidget);
    expect(find.text('OUT'), findsOneWidget);
    expect(find.text('CUE'), findsNothing);
    expect(find.text('Beat 1'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);

    await tester.tap(find.byIcon(FLucideIcons.audioLines));
    await tester.pumpAndSettle();
    expect(find.text('Beat 1'), findsOneWidget);
    expect(find.text('CUE'), findsNothing);
    expect(find.text('IN'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);

    await tester.tap(find.byIcon(FLucideIcons.disc3));
    await tester.pumpAndSettle();
    expect(find.bySemanticsLabel('Jog wheel'), findsOneWidget);
    expect(find.text('CUE'), findsNothing);
    expect(find.text('IN'), findsNothing);
    expect(find.text('Beat 1'), findsNothing);

    await tester.tap(find.byIcon(FLucideIcons.layoutGrid));
    await tester.pumpAndSettle();
    expect(find.text('CUE'), findsOneWidget);
    expect(find.text('IN'), findsNothing);
    expect(find.text('Beat 1'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);
  });
}
