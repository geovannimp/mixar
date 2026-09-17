import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

void main() {
  test('MixarThemeData typography applies brand font families', () {
    final typography = MixarThemeData.light().typography;

    expect(typography.body.fontFamily, MixarFonts.outfit);
    expect(typography.display.fontFamily, MixarFonts.spaceGrotesk);
    expect(typography.mono.fontFamily, MixarFonts.notoSansMono);
    expect(typography.body.sm.fontFamily, MixarFonts.outfit);
    expect(typography.display.lg.fontFamily, MixarFonts.spaceGrotesk);
    expect(typography.mono.xs.fontFamily, MixarFonts.notoSansMono);
  });
}
