import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_performance_panel.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

void main() {
  Future<void> pumpPanel(WidgetTester tester) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: MaterialApp(
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
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('Pads / Loop / Grid / Jog tabs switch exclusive content', (
    tester,
  ) async {
    await pumpPanel(tester);

    expect(find.byIcon(LucideIcons.layoutGrid), findsOneWidget);
    expect(find.byIcon(LucideIcons.repeat2), findsOneWidget);
    expect(find.byIcon(LucideIcons.audioLines), findsOneWidget);
    expect(find.byIcon(LucideIcons.disc3), findsOneWidget);
    expect(
      tester.getCenter(find.byIcon(LucideIcons.layoutGrid)).dy <
          tester.getCenter(find.byIcon(LucideIcons.repeat2)).dy,
      isTrue,
    );
    expect(
      tester.getCenter(find.byIcon(LucideIcons.repeat2)).dy <
          tester.getCenter(find.byIcon(LucideIcons.audioLines)).dy,
      isTrue,
    );
    expect(
      tester.getCenter(find.byIcon(LucideIcons.audioLines)).dy <
          tester.getCenter(find.byIcon(LucideIcons.disc3)).dy,
      isTrue,
    );
    expect(find.text('CUE'), findsOneWidget);
    expect(find.text('IN'), findsNothing);
    expect(find.text('Now'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);

    await tester.tap(find.byIcon(LucideIcons.repeat2));
    await tester.pumpAndSettle();
    expect(find.text('IN'), findsOneWidget);
    expect(find.text('OUT'), findsOneWidget);
    expect(find.text('CUE'), findsNothing);
    expect(find.text('Now'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);

    await tester.tap(find.byIcon(LucideIcons.audioLines));
    await tester.pumpAndSettle();
    expect(find.text('Now'), findsOneWidget);
    expect(find.text('CUE'), findsNothing);
    expect(find.text('IN'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);

    await tester.tap(find.byIcon(LucideIcons.disc3));
    await tester.pumpAndSettle();
    expect(find.bySemanticsLabel('Jog wheel'), findsOneWidget);
    expect(find.text('CUE'), findsNothing);
    expect(find.text('IN'), findsNothing);
    expect(find.text('Now'), findsNothing);

    await tester.tap(find.byIcon(LucideIcons.layoutGrid));
    await tester.pumpAndSettle();
    expect(find.text('CUE'), findsOneWidget);
    expect(find.text('IN'), findsNothing);
    expect(find.text('Now'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);
  });
}
