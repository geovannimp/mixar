import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

/// One tap target the toast tests mount; the callback receives a live context.
typedef ToastTrigger = ({
  String label,
  void Function(BuildContext context) onTap,
});

void main() {
  Future<void> pumpHome(WidgetTester tester, Widget home) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(body: home),
      ),
    );
  }

  /// Mounts [triggers] as a column of tap targets inside the themed shell.
  Future<void> pumpToastTriggers(
    WidgetTester tester,
    List<ToastTrigger> triggers,
  ) {
    return pumpHome(
      tester,
      Builder(
        builder: (context) => Column(
          children: [
            for (final trigger in triggers)
              GestureDetector(
                onTap: () => trigger.onTap(context),
                child: Text(trigger.label),
              ),
          ],
        ),
      ),
    );
  }

  testWidgets('showMixarToast displays title', (tester) async {
    await pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          title: const Text('Engine failed'),
          duration: const Duration(milliseconds: 200),
        ),
      ),
    ]);

    await tester.tap(find.text('Show'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));
    expect(find.text('Engine failed'), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pumpAndSettle();
  });

  testWidgets('showMixarToast suffix dismisses persistent toast', (
    tester,
  ) async {
    await pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Controller connected'),
          suffixBuilder: (context, dismiss) =>
              GestureDetector(onTap: dismiss, child: const Text('Enable')),
        ),
      ),
    ]);

    await tester.tap(find.text('Show'));
    await tester.pump();
    await tester.pumpAndSettle();
    expect(find.text('Controller connected'), findsOneWidget);

    await tester.tap(find.text('Enable'));
    await tester.pumpAndSettle();
    expect(find.text('Controller connected'), findsNothing);
  });

  testWidgets('dismissMixarToast is a no-op for unknown id', (tester) async {
    await pumpToastTriggers(tester, [
      (
        label: 'Dismiss',
        onTap: (context) => dismissMixarToast(context, id: 'missing-toast'),
      ),
    ]);

    await tester.tap(find.text('Dismiss'));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('dismissMixarToast closes toast by id', (tester) async {
    await pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          id: 'controller-offer:TestDev Port',
          duration: null,
          title: const Text('Pioneer DDJ-400 connected'),
        ),
      ),
      (
        label: 'Disconnect',
        onTap: (context) =>
            dismissMixarToast(context, id: 'controller-offer:TestDev Port'),
      ),
    ]);

    await tester.tap(find.text('Show'));
    await tester.pump();
    await tester.pumpAndSettle();
    expect(find.text('Pioneer DDJ-400 connected'), findsOneWidget);

    await tester.tap(find.text('Disconnect'));
    await tester.pumpAndSettle();
    expect(find.text('Pioneer DDJ-400 connected'), findsNothing);
  });

  testWidgets('showMixarToast stacks suffix below description', (tester) async {
    await pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Pioneer DDJ-400 connected'),
          description: const Text('Do you want to use this controller?'),
          suffixBuilder: (context, dismiss) => Row(
            mainAxisSize: MainAxisSize.min,
            spacing: 8,
            children: [
              GestureDetector(onTap: dismiss, child: const Text('Cancel')),
              GestureDetector(onTap: dismiss, child: const Text('Enable')),
            ],
          ),
        ),
      ),
    ]);

    await tester.tap(find.text('Show'));
    await tester.pump();
    await tester.pumpAndSettle();

    expect(find.text('Cancel'), findsOneWidget);
    expect(find.text('Enable'), findsOneWidget);
    final promptBottom = tester
        .getBottomLeft(find.text('Do you want to use this controller?'))
        .dy;
    final enableTop = tester.getTopLeft(find.text('Enable')).dy;
    expect(
      enableTop,
      greaterThanOrEqualTo(promptBottom),
      reason: 'Enable must sit under the prompt, not beside it',
    );

    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(
      find.text('Pioneer DDJ-400 connected'),
      findsNothing,
      reason: 'Cancel dismiss callback must close the toast',
    );
  });

  testWidgets('showMixarToast stacks newer toasts above the corner', (
    tester,
  ) async {
    await pumpToastTriggers(tester, [
      (
        label: 'Show first',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('First notice'),
        ),
      ),
      (
        label: 'Show second',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Second notice'),
        ),
      ),
    ]);

    await tester.tap(find.text('Show first'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Show second'));
    await tester.pumpAndSettle();

    final firstTop = tester.getTopLeft(find.text('First notice')).dy;
    final secondTop = tester.getTopLeft(find.text('Second notice')).dy;
    expect(firstTop, lessThan(secondTop));
    expect(
      tester.getBottomLeft(find.text('First notice')).dy,
      lessThan(secondTop),
      reason: 'toasts must stack, not overlap',
    );
  });

  testWidgets('toast variant picks its glyph', (tester) async {
    await pumpToastTriggers(tester, [
      (
        label: 'Show success',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Saved'),
          variant: MixarToastVariant.success,
        ),
      ),
      (
        label: 'Show error',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Failed'),
          variant: MixarToastVariant.destructive,
        ),
      ),
    ]);

    await tester.tap(find.text('Show success'));
    await tester.pumpAndSettle();
    expect(find.byIcon(LucideIcons.check), findsOneWidget);

    await tester.tap(find.text('Show error'));
    await tester.pumpAndSettle();
    expect(find.byIcon(LucideIcons.circleAlert), findsOneWidget);
  });
}
