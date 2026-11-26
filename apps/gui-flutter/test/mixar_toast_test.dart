import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

/// The card wrapping [text]: the toast is an intrinsic-width box in the root
/// overlay, so layout assertions measure this instead of the whole overlay.
Finder toastCard(Finder text) =>
    find.ancestor(of: text, matching: find.byType(IntrinsicWidth));

/// The variant disc inside [card], filled with [color].
Finder toastDisc(Finder card, Color color) => find.descendant(
  of: card,
  matching: find.byWidgetPredicate(
    (widget) =>
        widget is DecoratedBox &&
        widget.decoration ==
            BoxDecoration(color: color, shape: BoxShape.circle),
  ),
);

/// One tap target the toast tests mount; the callback receives a live context.
typedef ToastTrigger = ({
  String label,
  void Function(BuildContext context) onTap,
});

void main() {
  Future<void> pumpHome(
    WidgetTester tester,
    Widget home, {
    MixarThemeData? theme,
  }) async {
    final data = theme ?? MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(data),
        builder: mixarMaterialAppBuilder(data),
        home: Scaffold(body: home),
      ),
    );
  }

  /// Mounts [triggers] as a column of tap targets inside the themed shell.
  Future<void> pumpToastTriggers(
    WidgetTester tester,
    List<ToastTrigger> triggers, {
    MixarThemeData? theme,
  }) {
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
      theme: theme,
    );
  }

  /// Mounts the "Show" trigger that opens a persistent toast about a connected
  /// controller, so the tests below only differ in what they do with it.
  Future<void> pumpShowControllerToast(
    WidgetTester tester, {
    Widget Function(BuildContext context, VoidCallback dismiss)? suffixBuilder,
  }) {
    return pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Controller connected'),
          suffixBuilder: suffixBuilder,
        ),
      ),
    ]);
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
    await pumpShowControllerToast(
      tester,
      suffixBuilder: (context, dismiss) =>
          GestureDetector(onTap: dismiss, child: const Text('Enable')),
    );

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

  testWidgets('toast copy and actions share one left edge', (tester) async {
    await pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Switch audio outputs to DDJ-400?'),
          description: const Text('Warning: Mixar will restart.'),
          // A full-width row, as the controller prompt uses: the first child
          // then measures the copy column rather than a right-aligned cluster.
          suffixBuilder: (context, dismiss) => Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            spacing: 8,
            children: [
              GestureDetector(onTap: dismiss, child: const Text('Remember')),
              GestureDetector(onTap: dismiss, child: const Text('Switch')),
            ],
          ),
        ),
      ),
    ]);

    await tester.tap(find.text('Show'));
    await tester.pumpAndSettle();

    final card = toastCard(find.text('Switch audio outputs to DDJ-400?'));
    // Card padding (14) + the copy column past the variant glyph (16) and its
    // gap (12). Measured from the card, not from the title, so a suffix that
    // drifts right cannot satisfy the assertion by moving the title with it.
    final copyColumnX = tester.getTopLeft(card).dx + 14 + 28;
    void expectInCopyColumn(Finder finder, String reason) {
      expect(
        tester.getTopLeft(finder).dx,
        moreOrLessEquals(copyColumnX, epsilon: 0.5),
        reason: reason,
      );
    }

    expectInCopyColumn(
      find.text('Switch audio outputs to DDJ-400?'),
      'title must sit in the copy column',
    );
    expectInCopyColumn(
      find.text('Warning: Mixar will restart.'),
      'description must sit in the title copy column',
    );
    expectInCopyColumn(
      find.text('Remember'),
      'actions must sit in the title copy column',
    );
  });

  testWidgets('toast width follows content inside the cap', (tester) async {
    const longTitle =
        'Switch audio outputs to this rather long controller product name';
    await pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text(longTitle),
        ),
      ),
    ]);

    await tester.tap(find.text('Show'));
    await tester.pumpAndSettle();

    final card = toastCard(find.text(longTitle));
    expect(card, findsOneWidget);
    final width = tester.getSize(card).width;
    expect(width, greaterThanOrEqualTo(300), reason: 'card keeps its minimum');
    expect(
      width,
      lessThanOrEqualTo(420),
      reason: 'long copy must not push the card past its cap',
    );
  });

  testWidgets('toast paints a gradient surface and a circled glyph', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    final radius = theme.style.borderRadius.dialog;
    final borderWidth = theme.style.borderWidth;
    final accent = theme.colors.destructive;

    await pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Rounded toast'),
          description: const Text('Short body copy.'),
          variant: MixarToastVariant.destructive,
        ),
      ),
    ]);

    await tester.tap(find.text('Show'));
    await tester.pumpAndSettle();

    final card = toastCard(find.text('Rounded toast'));
    expect(card, findsOneWidget);

    final outline = find.descendant(
      of: card,
      matching: find.byWidgetPredicate(
        (widget) =>
            widget is DecoratedBox &&
            widget.position == DecorationPosition.foreground &&
            widget.decoration ==
                BoxDecoration(
                  borderRadius: radius,
                  border: Border.all(
                    color: theme.colors.border,
                    width: borderWidth,
                  ),
                ),
      ),
    );
    expect(
      outline,
      findsOneWidget,
      reason: 'the rounded outline must paint over the clipped card content',
    );

    final clip = find.descendant(
      of: outline,
      matching: find.byWidgetPredicate(
        (widget) =>
            widget is ClipRRect &&
            widget.borderRadius == radius &&
            widget.clipBehavior == Clip.antiAlias,
      ),
    );
    expect(clip, findsOneWidget);

    final gradientBox = find.descendant(
      of: clip,
      matching: find.byWidgetPredicate(
        (widget) =>
            widget is DecoratedBox &&
            widget.decoration is BoxDecoration &&
            (widget.decoration as BoxDecoration).gradient != null,
      ),
    );
    expect(gradientBox, findsOneWidget);
    final gradient =
        (tester.widget<DecoratedBox>(gradientBox).decoration as BoxDecoration)
                .gradient!
            as LinearGradient;
    expect(gradient.begin, Alignment.centerLeft);
    expect(gradient.end, Alignment.centerRight);
    expect(
      gradient.colors.last,
      theme.colors.card,
      reason: 'the wash fades into the card surface',
    );
    expect(
      gradient.colors.first,
      Color.alphaBlend(accent.withValues(alpha: 0.16), theme.colors.card),
      reason: 'the wash is the bare variant accent over the card',
    );

    expect(
      find.descendant(
        of: clip,
        matching: find.byWidgetPredicate(
          (widget) => widget is ColoredBox && widget.color == accent,
        ),
      ),
      findsNothing,
      reason: 'the left rail is gone',
    );

    final circle = toastDisc(clip, accent);
    expect(circle, findsOneWidget);
    expect(tester.getSize(circle), const Size(22, 22));

    // The disc must overhang its 16-px slot and be allowed to paint outside.
    final slot = find.descendant(
      of: card,
      matching: find.byWidgetPredicate(
        (widget) =>
            widget is SizedBox && widget.width == 16 && widget.height == 22,
      ),
    );
    expect(slot, findsOneWidget);
    final slotRect = tester.getRect(slot);
    final discRect = tester.getRect(circle);
    expect(discRect.left - slotRect.left, closeTo(-3, 0.01));
    expect(discRect.width, greaterThan(slotRect.width));
    expect(
      tester
          .widget<Stack>(
            find.descendant(of: slot, matching: find.byType(Stack)),
          )
          .clipBehavior,
      Clip.none,
    );

    final glyph = find.descendant(of: circle, matching: find.byType(Icon));
    expect(tester.widget<Icon>(glyph).icon, LucideIcons.circleAlert);
    expect(
      tester.widget<Icon>(glyph).color,
      theme.colors.destructiveForeground,
      reason: 'the glyph uses the on-accent ink for contrast',
    );
    expect(
      tester.getCenter(glyph),
      tester.getCenter(circle),
      reason: 'the glyph must sit centred in its disc',
    );

    expect(tester.takeException(), isNull);
  });

  testWidgets('non success toasts paint in the primary colour', (tester) async {
    final theme = MixarThemeData.dark();
    await pumpToastTriggers(tester, [
      (
        label: 'Info',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Info notice'),
        ),
      ),
      (
        label: 'Warning',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          variant: MixarToastVariant.warning,
          title: const Text('Warning notice'),
        ),
      ),
    ]);

    await tester.tap(find.text('Info'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Warning'));
    await tester.pumpAndSettle();

    for (final (title, icon) in [
      ('Info notice', LucideIcons.info),
      ('Warning notice', LucideIcons.triangleAlert),
    ]) {
      final card = toastCard(find.text(title));
      final glyph = tester.widget<Icon>(
        find.descendant(
          of: card,
          matching: find.byWidgetPredicate(
            (widget) => widget is Icon && widget.icon != LucideIcons.x,
          ),
        ),
      );
      expect(glyph.icon, icon, reason: '$title keeps its own glyph');
      expect(
        glyph.color,
        theme.colors.primaryForeground,
        reason: '$title uses on-accent ink over the primary disc',
      );
      expect(toastDisc(card, theme.colors.primary), findsOneWidget);
    }
  });

  Future<void> expectSuccessToast(
    WidgetTester tester,
    MixarThemeData theme,
    Color green,
  ) async {
    await pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          variant: MixarToastVariant.success,
          title: const Text('Settings saved'),
        ),
      ),
    ], theme: theme);

    await tester.tap(find.text('Show'));
    await tester.pumpAndSettle();

    final card = toastCard(find.text('Settings saved'));
    final glyph = tester.widget<Icon>(
      find.descendant(
        of: card,
        matching: find.byWidgetPredicate(
          (widget) => widget is Icon && widget.icon != LucideIcons.x,
        ),
      ),
    );
    expect(glyph.color, theme.colors.primaryForeground);
    expect(
      toastDisc(card, green),
      findsOneWidget,
      reason: 'the success disc is the local status green',
    );
  }

  testWidgets('success toast paints the success green in dark', (tester) async {
    // The palette ships no green ladder, so success keeps the local status
    // green the library live dot and settings verdict already use.
    await expectSuccessToast(
      tester,
      MixarThemeData.dark(),
      const Color(0xFF4ADE80),
    );
  });

  testWidgets('success toast paints the success green in light', (
    tester,
  ) async {
    await expectSuccessToast(
      tester,
      MixarThemeData.light(),
      const Color(0xFF15803D),
    );
  });

  testWidgets('toast icon override replaces the variant glyph', (tester) async {
    await pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Question toast'),
          icon: const Text('?'),
        ),
      ),
    ]);

    await tester.tap(find.text('Show'));
    await tester.pumpAndSettle();

    final card = toastCard(find.text('Question toast'));
    final question = find.descendant(of: card, matching: find.text('?'));
    expect(question, findsOneWidget);
    expect(
      find.ancestor(of: question, matching: find.byType(ExcludeSemantics)),
      findsOneWidget,
      reason: 'the glyph is decorative; the title carries the message',
    );
    expect(
      find.descendant(of: card, matching: find.byIcon(LucideIcons.info)),
      findsNothing,
    );
  });

  testWidgets('toast icon override ignores text scaling', (tester) async {
    tester.platformDispatcher.textScaleFactorTestValue = 2;
    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);

    await pumpToastTriggers(tester, [
      (
        label: 'Show',
        onTap: (context) => showMixarToast(
          context: context,
          duration: null,
          title: const Text('Question toast'),
          icon: const Text('?'),
        ),
      ),
    ]);

    await tester.tap(find.text('Show'));
    await tester.pumpAndSettle();

    final glyph = find.descendant(
      of: toastCard(find.text('Question toast')),
      matching: find.text('?'),
    );
    expect(glyph, findsOneWidget);
    final size = tester.getSize(glyph);
    expect(size.height, closeTo(16, 0.5), reason: 'glyph keeps its own size');
    expect(size.height, lessThanOrEqualTo(22), reason: 'glyph fits its disc');
    expect(size.width, lessThanOrEqualTo(22));
  });

  testWidgets('toast close hover paints a circular wash', (tester) async {
    final theme = MixarThemeData.dark();
    await pumpShowControllerToast(tester);

    await tester.tap(find.text('Show'));
    await tester.pump();
    await tester.pumpAndSettle();
    expect(find.text('Controller connected'), findsOneWidget);

    final close = find.byWidgetPredicate(
      (widget) =>
          widget is MTappable &&
          widget.semanticsLabel == 'Dismiss notification',
    );
    expect(close, findsOneWidget);
    expect(tester.getSize(close), const Size(26, 26));

    final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: Offset.zero);
    addTearDown(gesture.removePointer);
    await tester.pump();
    await gesture.moveTo(tester.getCenter(close));
    await tester.pumpAndSettle();

    final wash = tester
        .widgetList<DecoratedBox>(
          find.descendant(of: close, matching: find.byType(DecoratedBox)),
        )
        .map((box) => box.decoration as BoxDecoration)
        .last;
    expect(wash.shape, BoxShape.circle);
    expect(wash.color, theme.hoverWash);

    await tester.tap(close);
    await tester.pumpAndSettle();
    expect(find.text('Controller connected'), findsNothing);
  });
}
