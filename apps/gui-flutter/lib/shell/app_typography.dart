import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';

/// Vendored font families (see `pubspec.yaml` / `fonts/`).
abstract final class MixarFonts {
  static const outfit = 'Outfit';
  static const spaceGrotesk = 'Space Grotesk';
  static const notoSansMono = 'Noto Sans Mono';
}

/// Shared numeric / case features. Do **not** put Outfit `ss01` here — that set
/// swaps the default horizontal-`e` glyphs for angled Kabel-style alternates
/// (what Google Fonts shows without stylistic sets).
final _numericCaseFeatures = [
  FontFeature.enable('case'),
  FontFeature.tabularFigures(),
  FontFeature.slashedZero(),
];

/// Space Grotesk display: keep ss01/ss04 from the marketing stack.
final _displayFeatures = [
  FontFeature.stylisticSet(1), // ss01
  FontFeature.stylisticSet(4), // ss04
  ..._numericCaseFeatures,
];

final _monoFeatures = [FontFeature.tabularFigures(), FontFeature.slashedZero()];

/// Size scale for one face (Forui desktop non-touch sizes).
@immutable
class MixarTypeface {
  const MixarTypeface({
    required this.fontFamily,
    required this.xs3,
    required this.xs2,
    required this.xs,
    required this.sm,
    required this.md,
    required this.lg,
    required this.xl,
    required this.xl2,
    required this.xl3,
    required this.xl4,
    required this.xl5,
    required this.xl6,
    required this.xl7,
    required this.xl8,
    this.fontFamilyFallback = const [],
  });

  factory MixarTypeface.desktop({
    required Color color,
    required String fontFamily,
    List<String> fontFamilyFallback = const [],
    List<FontFeature>? fontFeatures,
  }) {
    TextStyle style(double size, double height) => TextStyle(
      color: color,
      fontFamily: fontFamily,
      fontFamilyFallback: fontFamilyFallback,
      fontSize: size,
      height: height,
      leadingDistribution: TextLeadingDistribution.even,
      fontFeatures: fontFeatures,
      // Avoid inheriting MaterialApp's debug error underline when no
      // DefaultTextStyle/Material ancestor is present.
      decoration: TextDecoration.none,
    );
    return MixarTypeface(
      fontFamily: fontFamily,
      fontFamilyFallback: fontFamilyFallback,
      xs3: style(8, 1),
      xs2: style(10, 1),
      xs: style(12, 1),
      sm: style(14, 1.25),
      md: style(16, 1.5),
      lg: style(18, 1.75),
      xl: style(20, 1.75),
      xl2: style(22, 2),
      xl3: style(30, 2.25),
      xl4: style(36, 2.5),
      xl5: style(48, 1),
      xl6: style(60, 1),
      xl7: style(72, 1),
      xl8: style(96, 1),
    );
  }

  final String fontFamily;
  final List<String> fontFamilyFallback;
  final TextStyle xs3;
  final TextStyle xs2;
  final TextStyle xs;
  final TextStyle sm;
  final TextStyle md;
  final TextStyle lg;
  final TextStyle xl;
  final TextStyle xl2;
  final TextStyle xl3;
  final TextStyle xl4;
  final TextStyle xl5;
  final TextStyle xl6;
  final TextStyle xl7;
  final TextStyle xl8;
}

/// Outfit body, Space Grotesk display, Noto Sans Mono — Tauri-era stack.
@immutable
class MixarTypography {
  const MixarTypography({
    required this.display,
    required this.body,
    required this.mono,
  });

  factory MixarTypography.mixar(Color foreground) {
    return MixarTypography(
      display: MixarTypeface.desktop(
        color: foreground,
        fontFamily: MixarFonts.spaceGrotesk,
        fontFeatures: _displayFeatures,
      ),
      body: MixarTypeface.desktop(
        color: foreground,
        fontFamily: MixarFonts.outfit,
        fontFeatures: _numericCaseFeatures,
      ),
      mono: MixarTypeface.desktop(
        color: foreground,
        fontFamily: MixarFonts.notoSansMono,
        fontFeatures: _monoFeatures,
      ),
    );
  }

  final MixarTypeface display;
  final MixarTypeface body;
  final MixarTypeface mono;
}
