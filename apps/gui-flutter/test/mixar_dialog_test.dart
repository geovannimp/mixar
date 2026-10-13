import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_button_style.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpHome(
    WidgetTester tester,
    Widget home, {
    Locale locale = const Locale('en', 'US'),
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        locale: locale,
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(body: home),
      ),
    );
  }

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
    expect(find.text('Delete?'), findsOneWidget);
    await tester.tap(find.text('Delete'));
    await tester.pumpAndSettle();
    expect(result, 'delete');
  });

  testWidgets('showMixarConfirm actions sit below body text', (tester) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () async {
            await showMixarConfirm<bool>(
              context: context,
              title: 'Clear stem cache?',
              body:
                  'Deletes generated stem files for all tracks. '
                  'Original library audio is not touched.',
              actions: const [
                MixarDialogAction(
                  label: 'Cancel',
                  value: false,
                  variant: MixarButtonVariant.outline,
                ),
                MixarDialogAction(
                  label: 'Clear',
                  value: true,
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

    final bodyRect = tester.getRect(
      find.text(
        'Deletes generated stem files for all tracks. '
        'Original library audio is not touched.',
      ),
    );
    final cancelRect = tester.getRect(find.text('Cancel'));
    expect(cancelRect.top, greaterThanOrEqualTo(bodyRect.bottom));
  });

  testWidgets('showMixarDialog scrolls tall content without overflow', (
    tester,
  ) async {
    // Compact viewport so a long column exceeds the dialog height.
    await tester.binding.setSurfaceSize(const Size(400, 320));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () async {
            await showMixarDialog<void>(
              context: context,
              builder: (context) => Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [for (var i = 0; i < 40; i++) Text('Row $i')],
                ),
              ),
            );
          },
          child: const Text('Open'),
        ),
      ),
    );

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(find.text('Row 0'), findsOneWidget);
    await tester.scrollUntilVisible(
      find.text('Row 39'),
      80,
      scrollable: find.byType(Scrollable).first,
    );
    expect(find.text('Row 39'), findsOneWidget);
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

  testWidgets('dialog close icon is larger and centered on the title', (
    tester,
  ) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () async {
            await showMixarDialog<void>(
              context: context,
              builder: (context) {
                final theme = context.theme;
                return Padding(
                  padding: const EdgeInsets.all(16),
                  child: Text(
                    'Pad mode',
                    style: theme.typography.body.md.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                );
              },
            );
          },
          child: const Text('Open'),
        ),
      ),
    );

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();

    final closeIcon = tester.widget<Icon>(find.byIcon(LucideIcons.x));
    expect(closeIcon.size, 20);

    final titleRect = tester.getRect(find.text('Pad mode'));
    final closeRect = tester.getRect(find.byIcon(LucideIcons.x));
    expect(
      closeRect.center.dy,
      moreOrLessEquals(titleRect.center.dy, epsilon: 1),
    );
  });
}
