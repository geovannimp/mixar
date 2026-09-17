import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  test('MixarThemeData typography applies brand font families', () {
    final typography = MixarThemeData.light().typography;

    expect(typography.body.fontFamily, MixarFonts.outfit);
    expect(typography.display.fontFamily, MixarFonts.spaceGrotesk);
    expect(typography.mono.fontFamily, MixarFonts.notoSansMono);
    expect(typography.body.sm.fontFamily, MixarFonts.outfit);
    expect(typography.display.lg.fontFamily, MixarFonts.spaceGrotesk);
    expect(typography.mono.xs.fontFamily, MixarFonts.notoSansMono);
    expect(typography.body.sm.decoration, TextDecoration.none);
  });

  testWidgets('MixarTheme installs DefaultTextStyle without underline', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    late TextStyle inherited;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        home: MixarTheme(
          data: theme,
          child: Builder(
            builder: (context) {
              inherited = DefaultTextStyle.of(context).style;
              return const Text('hello');
            },
          ),
        ),
      ),
    );

    expect(inherited.decoration, TextDecoration.none);
    expect(inherited.color, theme.colors.foreground);
    expect(inherited.fontFamily, MixarFonts.outfit);
  });
}
