import 'dart:typed_data';

import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
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

  testWidgets('UI panel offers the theme select, defaulting to system', (
    tester,
  ) async {
    await pumpPanel(tester);
    expect(find.text('THEME'), findsOneWidget);
    final select = find.byType(MixarSelect<ThemeSetting>);
    expect(select, findsOneWidget);
    expect(
      find.descendant(of: select, matching: find.text('System default')),
      findsOneWidget,
    );
  }, semanticsEnabled: false);

  testWidgets('picking Dark emits the theme setting', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (settings) => changed = settings);

    await tester.tap(find.byType(MixarSelect<ThemeSetting>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Dark').last);
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

    final select = find.byType(MixarSelect<ThemeSetting>);
    tester
        .widget<ShadFocusable>(
          find.descendant(of: select, matching: find.byType(ShadFocusable)),
        )
        .focusNode
        ?.requestFocus();
    await tester.pumpAndSettle();

    // The ring sits 4px outside the control and strokes 2px wide, so the
    // collapsing card body must not crop the vertical strokes at the control's
    // own edge.
    final rect = tester.getRect(select);
    final boundary =
        captureKey.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    final origin = boundary.localToGlobal(Offset.zero);
    final primary = MixarThemeData.dark().colors.primary;
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
    final rgba = raster!.rgba;
    final stride = raster.width * 4;
    final midRow = (rect.center.dy - origin.dy).round();
    final leftColumn = (rect.left - origin.dx).round() - 4;
    expect(
      _hasColor(rgba, stride, leftColumn, midRow, primary),
      isTrue,
      reason:
          'the ring paints at x=$leftColumn, 4px left of the control edge '
          '${rect.left - origin.dx}',
    );
    expect(
      _hasColor(rgba, stride, leftColumn + 1, midRow, primary),
      isTrue,
      reason: 'the ring stroke is 2px wide',
    );
  }, semanticsEnabled: false);
}

/// Whether the pixel at ([x], [y]) matches [color].
bool _hasColor(Uint8List rgba, int stride, int x, int y, Color color) {
  final argb = color.toARGB32();
  final i = y * stride + x * 4;
  return rgba[i] == ((argb >> 16) & 0xFF) &&
      rgba[i + 1] == ((argb >> 8) & 0xFF) &&
      rgba[i + 2] == (argb & 0xFF);
}
