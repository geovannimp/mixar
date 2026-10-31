import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/mixar_colors.dart';
import 'package:gui_flutter/core/theme/mixar_palette.dart';

/// The light theme is warm bone paper: surfaces climb a tinted ladder instead
/// of reaching pure white, and the raised panel stays distinguishable from the
/// page. These are the invariants behind "light, not blinding white".
void main() {
  const light = MixarColors.light;

  final surfaces = <String, Color>{
    'background': light.background,
    'card': light.card,
    'secondary': light.secondary,
    'muted': light.muted,
    'waveformBackground': light.waveformBackground,
  };

  test('light surfaces never reach pure white', () {
    for (final entry in surfaces.entries) {
      expect(
        entry.value,
        isNot(const Color(0xFFFFFFFF)),
        reason: '${entry.key} must stay off pure white',
      );
    }
  });

  test('light surfaces live on the warm ladder', () {
    expect(light.background, MixarBone.s100);
    expect(light.card, MixarBone.s50);
    expect(light.secondary, MixarBone.s200);
    expect(light.muted, MixarBone.s200);
    expect(light.waveformBackground, MixarBone.s200);
    expect(light.border, MixarBone.s300);
    expect(light.foreground, MixarBone.s900);
  });

  test('light card reads as a raised plane', () {
    final card = light.card.computeLuminance();
    final page = light.background.computeLuminance();
    final chip = light.secondary.computeLuminance();
    // Nine points of luminance, not the 4 that a near-white page gives.
    expect((card - page) * 100, greaterThan(6));
    expect(page, greaterThan(chip));
  });

  test('light text keeps WCAG AA on the new surfaces', () {
    double contrast(Color a, Color b) {
      final la = a.computeLuminance();
      final lb = b.computeLuminance();
      final hi = la > lb ? la : lb;
      final lo = la > lb ? lb : la;
      return (hi + 0.05) / (lo + 0.05);
    }

    expect(contrast(light.foreground, light.card), greaterThan(7));
    expect(contrast(light.foreground, light.background), greaterThan(7));
    expect(contrast(light.mutedForeground, light.card), greaterThan(4.5));
    expect(contrast(light.mutedForeground, light.background), greaterThan(4.5));
  });
}
