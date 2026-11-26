import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpHome(
    WidgetTester tester,
    Widget home, {
    Locale locale = const Locale('en', 'US'),
    TextDirection textDirection = TextDirection.ltr,
  }) async {
    final theme = MixarThemeData.dark();
    final appBuilder = mixarMaterialAppBuilder(theme);
    await tester.pumpWidget(
      MaterialApp(
        locale: locale,
        theme: materialUiThemeFromMixar(theme),
        builder: (context, child) => Directionality(
          textDirection: textDirection,
          child: appBuilder(context, child),
        ),
        home: Scaffold(body: home),
      ),
    );
  }

  /// Text surface that opens [show] when tapped, so every dialog test shares
  /// one opener instead of its own Builder/GestureDetector copy.
  Widget openOnTap(Future<void> Function(BuildContext pageContext) show) {
    return Builder(
      builder: (pageContext) => GestureDetector(
        onTap: () => show(pageContext),
        child: const Text('Open'),
      ),
    );
  }

  /// Opener for the placeholder dialog the localization and barrier tests use.
  Widget exportDialogOpener() {
    return openOnTap(
      (context) => showMixarDialog<String?>(
        context: context,
        builder: (_) => const Padding(
          padding: EdgeInsets.all(16),
          child: Text('Export format'),
        ),
      ),
    );
  }

  testWidgets('dialogs dim the body but leave the titlebar bright', (
    tester,
  ) async {
    await pumpHome(tester, exportDialogOpener());

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();

    // The body-only barrier is the dialog's dimming layer; the titlebar is
    // above it and stays clickable.
    final barrier = tester
        .widgetList<ModalBarrier>(find.byType(ModalBarrier))
        .firstWhere((b) => b.color == kMixarDialogScrimColor);
    final rect = tester.getRect(find.byWidget(barrier));
    expect(rect.top, kAppHeaderHeight);
    expect(rect.height, 600 - kAppHeaderHeight);

    // A titlebar press must not dismiss the dialog.
    await tester.tapAt(const Offset(400, 20));
    await tester.pumpAndSettle();
    expect(find.text('Export format'), findsOneWidget);

    // A press on the dimmed body still closes it.
    await tester.tapAt(const Offset(20, 300));
    await tester.pumpAndSettle();
    expect(find.text('Export format'), findsNothing);
  });

  testWidgets('showMixarDialog under pt_BR does not crash', (tester) async {
    await pumpHome(
      tester,
      exportDialogOpener(),
      locale: const Locale('pt', 'BR'),
    );

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(find.text('Export format'), findsOneWidget);
  });

  testWidgets('showMixarDialog builds content without localization crash', (
    tester,
  ) async {
    await pumpHome(tester, exportDialogOpener());

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(find.text('Export format'), findsOneWidget);
  });

  testWidgets('showMixarConfirm returns chosen action value', (tester) async {
    String? result;
    await pumpHome(
      tester,
      openOnTap((context) async {
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
      }),
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
      openOnTap(
        (context) => showMixarConfirm<bool>(
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

  testWidgets('dialog footer ends on the close button column', (tester) async {
    const title =
        'Delete these tracks from every playlist and the library collection?';
    for (final direction in [TextDirection.ltr, TextDirection.rtl]) {
      await pumpHome(
        tester,
        openOnTap(
          (context) => showMixarConfirm<String>(
            context: context,
            title: title,
            actions: const [
              MixarDialogAction(label: 'Delete', value: 'delete'),
              MixarDialogAction(
                label: 'Cancel',
                value: 'cancel',
                variant: MixarButtonVariant.outline,
              ),
            ],
          ),
        ),
        textDirection: direction,
      );

      await tester.tap(find.text('Open'));
      await tester.pumpAndSettle();

      // The close glyph's box and the footer's last pill must end on the same
      // column, on whichever side the direction puts the close button.
      final rtl = direction == TextDirection.rtl;
      final closeIcon = tester.getRect(find.byIcon(LucideIcons.x));
      final titleRect = tester.getRect(find.text(title));
      final pill = tester.getRect(find.widgetWithText(AppButton, 'Cancel'));
      final card = tester.getRect(find.byType(MixarDialog));

      final closeTrailing = rtl ? closeIcon.left : closeIcon.right;
      final pillTrailing = rtl ? pill.left : pill.right;
      final cardTrailing = rtl ? card.left : card.right;
      final inset = rtl
          ? pillTrailing - cardTrailing
          : cardTrailing - pillTrailing;

      expect(pillTrailing, closeTrailing, reason: '$direction pill vs close');
      expect(
        inset,
        mixarDialogCloseInset,
        reason: '$direction card edge to pill',
      );
      // The header reserves its close-button space on the same side, so a long
      // title wraps clear of the icon instead of running under it.
      expect(
        titleRect.overlaps(closeIcon),
        isFalse,
        reason: '$direction header vs close icon',
      );

      // Dismiss the card so the next pass starts from a clean navigator.
      await tester.tapAt(const Offset(12, 560));
      await tester.pumpAndSettle();
    }
  });

  testWidgets('showMixarDialog scrolls tall content without overflow', (
    tester,
  ) async {
    // Compact viewport so a long column exceeds the dialog height.
    await tester.binding.setSurfaceSize(const Size(400, 320));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    await pumpHome(
      tester,
      openOnTap(
        (context) => showMixarDialog<void>(
          context: context,
          builder: (_) => Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [for (var i = 0; i < 40; i++) Text('Row $i')],
            ),
          ),
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
      openOnTap((context) async {
        result = await showMixarConfirm<String>(
          context: context,
          title: 'Hello',
          actions: const [MixarDialogAction(label: 'Ok', value: 'ok')],
        );
      }),
    );

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    // The body barrier, below the titlebar, is the dismiss target.
    await tester.tapAt(const Offset(8, kAppHeaderHeight + 10));
    await tester.pumpAndSettle();
    expect(result, isNull);
  });

  testWidgets('dialog close icon is larger and centered on the title', (
    tester,
  ) async {
    await pumpHome(
      tester,
      openOnTap(
        (context) => showMixarDialog<void>(
          context: context,
          builder: (dialogContext) {
            final theme = dialogContext.theme;
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
