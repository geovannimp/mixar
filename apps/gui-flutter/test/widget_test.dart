import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_shell.dart';
import 'package:gui_flutter/shell/desktop.dart';

void main() {
  testWidgets('mixer shell shows core regions', (tester) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);

    final theme = FTheme.neutral.light.desktop;
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      MaterialApp(
        theme: theme.toApproximateMaterialTheme(),
        builder: (context, child) => FTheme(data: theme, child: child!),
        home: const AppShell(),
      ),
    );

    expect(find.text('RUST DJ'), findsOneWidget);
    expect(find.text('Deck A'), findsWidgets);
    expect(find.text('Deck B'), findsWidgets);
    expect(find.text('Load tracks to see waveforms.'), findsOneWidget);
    expect(find.textContaining('Filter tracks'), findsOneWidget);
  });
}
