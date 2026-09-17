import 'package:flutter/material.dart' as flutter_material;
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_button_style.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:material_ui/material_ui.dart';

import 'support/forui_material_app.dart';

void main() {
  Future<void> pumpHome(
    WidgetTester tester,
    Widget home, {
    Locale locale = const Locale('en', 'US'),
  }) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        locale: locale,
        theme: materialUiThemeFromForui(theme),
        builder: foruiMaterialAppBuilder(theme),
        home: Scaffold(body: home),
      ),
    );
  }

  testWidgets('flutter MaterialLocalizations resolve under pt_BR', (
    tester,
  ) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) {
          final label = flutter_material.MaterialLocalizations.of(
            context,
          ).dialogLabel;
          return Text(label);
        },
      ),
      locale: const Locale('pt', 'BR'),
    );
    expect(find.text('Dialog'), findsOneWidget);
  });

  testWidgets('showMixarDialog under pt_BR does not crash', (tester) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () async {
            await showMixarDialog<String?>(
              context: context,
              builder: (context) => const Padding(
                padding: EdgeInsets.all(16),
                child: Text('Export format'),
              ),
            );
          },
          child: const Text('Open'),
        ),
      ),
      locale: const Locale('pt', 'BR'),
    );

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(find.text('Export format'), findsOneWidget);
  });

  testWidgets('LegacyMaterialScope exposes flutter MaterialLocalizations', (
    tester,
  ) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) {
          final label = flutter_material.MaterialLocalizations.of(
            context,
          ).dialogLabel;
          return Text(label);
        },
      ),
    );
    expect(find.text('Dialog'), findsOneWidget);
  });

  testWidgets('showMixarDialog builds content without localization crash', (
    tester,
  ) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () async {
            await showMixarDialog<String?>(
              context: context,
              builder: (context) => const Padding(
                padding: EdgeInsets.all(16),
                child: Text('Export format'),
              ),
            );
          },
          child: const Text('Open'),
        ),
      ),
    );

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(find.text('Export format'), findsOneWidget);
  });

  testWidgets('showMixarConfirm returns chosen action value', (tester) async {
    String? result;
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () async {
            result = await showMixarConfirm<String>(
              context: context,
              title: 'Delete?',
              body: 'Cannot undo.',
              actions: const [
                MixarDialogAction(
                  label: 'Cancel',
                  value: 'cancel',
                  variant: MixarButtonVariant.outline,
                ),
                MixarDialogAction(
                  label: 'Delete',
                  value: 'delete',
                  variant: MixarButtonVariant.destructive,
                ),
              ],
            );
          },
          child: const Text('Open'),
        ),
      ),
    );

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(find.text('Delete?'), findsWidgets);
    await tester.tap(find.text('Delete'));
    await tester.pumpAndSettle();
    expect(result, 'delete');
  });

  testWidgets('showMixarConfirm barrier dismiss returns null', (tester) async {
    String? result = 'unset';
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () async {
            result = await showMixarConfirm<String>(
              context: context,
              title: 'Hello',
              actions: const [MixarDialogAction(label: 'Ok', value: 'ok')],
            );
          },
          child: const Text('Open'),
        ),
      ),
    );

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    await tester.tapAt(const Offset(8, 8));
    await tester.pumpAndSettle();
    expect(result, isNull);
  });
}
