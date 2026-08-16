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

  testWidgets('Pads / Loop tabs switch exclusive content', (tester) async {
    await pumpPanel(tester);

    expect(find.byIcon(FLucideIcons.layoutGrid), findsOneWidget);
    expect(find.byIcon(FLucideIcons.repeat2), findsOneWidget);
    expect(
      tester.getCenter(find.byIcon(FLucideIcons.layoutGrid)).dy <
          tester.getCenter(find.byIcon(FLucideIcons.repeat2)).dy,
      isTrue,
    );
    expect(find.text('CUE'), findsOneWidget);
    expect(find.text('IN'), findsNothing);

    await tester.tap(find.byIcon(FLucideIcons.repeat2));
    await tester.pumpAndSettle();
    expect(find.text('IN'), findsOneWidget);
    expect(find.text('OUT'), findsOneWidget);
    expect(find.text('CUE'), findsNothing);

    await tester.tap(find.byIcon(FLucideIcons.layoutGrid));
    await tester.pumpAndSettle();
    expect(find.text('CUE'), findsOneWidget);
    expect(find.text('IN'), findsNothing);
  });
}
