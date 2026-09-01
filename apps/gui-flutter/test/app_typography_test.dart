import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_typography.dart';

void main() {
  test('mixarTypography applies brand font families', () {
    final typography = mixarTypography(FTheme.neutral.light.desktop.typography);

    expect(typography.body.fontFamily, MixarFonts.outfit);
    expect(typography.display.fontFamily, MixarFonts.spaceGrotesk);
    expect(typography.mono.fontFamily, MixarFonts.notoSansMono);
    expect(typography.body.sm.fontFamily, MixarFonts.outfit);
    expect(typography.display.lg.fontFamily, MixarFonts.spaceGrotesk);
    expect(typography.mono.xs.fontFamily, MixarFonts.notoSansMono);
  });
}
