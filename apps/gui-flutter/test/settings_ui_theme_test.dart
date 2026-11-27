import 'dart:typed_data';

import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_theme_picker.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    AppSettings? draft,
    ValueChanged<AppSettings>? onChanged,
    Key? captureKey,
  }) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: RepaintBoundary(
                key: captureKey,
                child: SettingsUiPanel(
                  draft: draft ?? defaultAppSettings(),
                  onChanged: onChanged ?? (_) {},
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('UI panel offers the theme choice, defaulting to system', (
    tester,
  ) async {
    await pumpPanel(tester);
    expect(find.text('THEME'), findsOneWidget);
    final choice = find.byType(SettingsThemePicker);
    expect(choice, findsOneWidget);
    final widget = tester.widget<SettingsThemePicker>(choice);
    expect(widget.value, ThemeSetting.system);
    for (final label in ['System default', 'Light', 'Dark']) {
      expect(
        find.descendant(of: choice, matching: find.text(label)),
        findsOneWidget,
      );
    }

    // The three options sit in one rounded pill, like the accent picker.
    final track = tester.widget<DecoratedBox>(
      find.descendant(of: choice, matching: find.byType(DecoratedBox)).first,
    );
    final decoration = track.decoration as BoxDecoration;
    expect(
      decoration.borderRadius,
      MixarThemeData.dark().style.borderRadius.pill,
    );
    final centres = {
      for (final label in ['System default', 'Light', 'Dark'])
        tester
            .getCenter(find.descendant(of: choice, matching: find.text(label)))
            .dy
            .round(),
    };
    expect(centres, hasLength(1));
  }, semanticsEnabled: false);

  testWidgets('picking Dark emits the theme setting', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (settings) => changed = settings);

    final darkOption = find.descendant(
      of: find.byType(SettingsThemePicker),
      matching: find.text('Dark'),
    );
    await tester.ensureVisible(darkOption);
    await tester.pumpAndSettle();
    await tester.tap(darkOption);
    await tester.pumpAndSettle();

    expect(changed?.theme, ThemeSetting.dark);
  }, semanticsEnabled: false);

  testWidgets('focused select keeps its whole ring inside the card body', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final captureKey = GlobalKey();
    await pumpPanel(tester, captureKey: captureKey);

    final select = find.byType(MixarSelect<UiLanguageSetting>);
    await _focusSelect(tester, select);

    // The ring sits 4px outside the control and strokes 2px wide, so the
    // collapsing card body must not crop the vertical strokes at the control's
    // own edge.
    final rect = tester.getRect(select);
    final capture = await _capturePanel(tester, captureKey);
    final primary = MixarThemeData.dark().colors.primary;
    final midRow = (rect.center.dy - capture.origin.dy).round();
    final leftColumn = (rect.left - capture.origin.dx).round() - 4;
    // A stadium's widest point is tangent, so its outermost raster column is
    // antialiased; allow a few levels of channel drift there.
    expect(
      _hasColorNear(
        capture.rgba,
        capture.stride,
        leftColumn,
        midRow,
        primary,
        tolerance: 12,
      ),
      isTrue,
      reason:
          'the ring paints at x=$leftColumn, 4px left of the control edge '
          '${rect.left - capture.origin.dx}',
    );
    expect(
      _hasColorNear(
        capture.rgba,
        capture.stride,
        leftColumn + 1,
        midRow,
        primary,
      ),
      isTrue,
      reason: 'the ring stroke is 2px wide',
    );
  }, semanticsEnabled: false);
}

/// Focuses the shad control inside [select] and lets the focus ring settle.
Future<void> _focusSelect(WidgetTester tester, Finder select) async {
  tester
      .widget<ShadFocusable>(
        find.descendant(of: select, matching: find.byType(ShadFocusable)),
      )
      .focusNode
      ?.requestFocus();
  await tester.pumpAndSettle();
}

/// RGBA pixels of the panel captured under [captureKey], with its global
/// origin so test rects can be turned into raster coordinates.
Future<({Uint8List rgba, int stride, Offset origin})> _capturePanel(
  WidgetTester tester,
  GlobalKey captureKey,
) async {
  final boundary =
      captureKey.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  final origin = boundary.localToGlobal(Offset.zero);
  // Rasterizing needs the real event loop; toImage deadlocks inside the
  // fake-async zone tester.runAsync would otherwise keep.
  final raster = await tester.runAsync(() async {
    final image = await boundary.toImage();
    try {
      final data = await image.toByteData();
      return (rgba: data!.buffer.asUint8List(), width: image.width);
    } finally {
      image.dispose();
    }
  });
  return (rgba: raster!.rgba, stride: raster.width * 4, origin: origin);
}

/// Whether the pixel at ([x], [y]) approximately matches [color] within
/// [tolerance] levels per channel for antialiased edges.
///
/// [tolerance] of 0 requires an exact match; 12 covers the few levels of
/// channel drift a stadium's tangent edge picks up from antialiasing.
bool _hasColorNear(
  Uint8List rgba,
  int stride,
  int x,
  int y,
  Color color, {
  int tolerance = 0,
}) {
  final argb = color.toARGB32();
  final i = y * stride + x * 4;
  return (rgba[i] - ((argb >> 16) & 0xFF)).abs() <= tolerance &&
      (rgba[i + 1] - ((argb >> 8) & 0xFF)).abs() <= tolerance &&
      (rgba[i + 2] - (argb & 0xFF)).abs() <= tolerance;
}
