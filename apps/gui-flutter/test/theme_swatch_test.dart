import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/theme_swatch.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpSwatch(
    WidgetTester tester,
    ThemeSetting value, {
    required double width,
    required double height,
  }) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Center(
          child: SizedBox(
            width: width,
            height: height,
            child: ThemeSwatch(value: value, palette: MixarAccentPalette.teal),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<Uint8List> capture(
    WidgetTester tester,
    ThemeSetting value, {
    required double width,
    required double height,
  }) async {
    final key = GlobalKey();
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: RepaintBoundary(
          key: key,
          child: Center(
            child: SizedBox(
              width: width,
              height: height,
              child: ThemeSwatch(
                value: value,
                palette: MixarAccentPalette.teal,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final boundary =
        key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    // Rasterizing needs the real event loop; toImage deadlocks inside the
    // fake-async zone tester.runAsync would otherwise keep.
    final raster = await tester.runAsync(() async {
      final image = await boundary.toImage();
      try {
        final data = await image.toByteData();
        return data!.buffer.asUint8List();
      } finally {
        image.dispose();
      }
    });
    return raster!;
  }

  testWidgets('paints every theme at the Settings preview size', (
    tester,
  ) async {
    for (final theme in ThemeSetting.values) {
      await pumpSwatch(tester, theme, width: 64, height: 28);
      expect(tester.takeException(), isNull);
      expect(find.byType(ThemeSwatch), findsOneWidget);
    }
  }, semanticsEnabled: false);

  testWidgets('light and dark swatches paint different pixels', (tester) async {
    final light = await capture(
      tester,
      ThemeSetting.light,
      width: 64,
      height: 28,
    );
    final dark = await capture(
      tester,
      ThemeSetting.dark,
      width: 64,
      height: 28,
    );

    expect(light.length, dark.length);
    expect(listEquals(light, dark), isFalse);
  }, semanticsEnabled: false);

  testWidgets('paints at the theme tile 16:9 size', (tester) async {
    await pumpSwatch(tester, ThemeSetting.system, width: 240, height: 135);
    expect(tester.takeException(), isNull);
  }, semanticsEnabled: false);
}
