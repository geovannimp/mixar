import 'package:flutter/material.dart';
import 'package:forui/forui.dart';

/// Vendored font families (see `pubspec.yaml` / `fonts/`).
abstract final class MixarFonts {
  static const outfit = 'Outfit';
  static const spaceGrotesk = 'Space Grotesk';
  static const notoSansMono = 'Noto Sans Mono';
}

/// Stylistic features mirrored from the Tauri-era CSS (`apps/gui-app/src/index.css`).
final _brandFeatures = [
  FontFeature.stylisticSet(1), // ss01
  FontFeature.enable('case'),
  FontFeature.tabularFigures(),
  FontFeature.slashedZero(),
];

final _displayFeatures = [
  ..._brandFeatures,
  FontFeature.stylisticSet(4), // ss04 on Space Grotesk
];

final _monoFeatures = [FontFeature.tabularFigures(), FontFeature.slashedZero()];

FTypeface _remapTypeface(
  FTypeface base,
  String fontFamily, {
  List<FontFeature>? fontFeatures,
}) {
  TextStyle remap(TextStyle style) => style.copyWith(
    fontFamily: fontFamily,
    fontFamilyFallback: const <String>[],
    fontFeatures: fontFeatures ?? style.fontFeatures,
  );

  return FTypeface(
    fontFamily: fontFamily,
    xs3: remap(base.xs3),
    xs2: remap(base.xs2),
    xs: remap(base.xs),
    sm: remap(base.sm),
    md: remap(base.md),
    lg: remap(base.lg),
    xl: remap(base.xl),
    xl2: remap(base.xl2),
    xl3: remap(base.xl3),
    xl4: remap(base.xl4),
    xl5: remap(base.xl5),
    xl6: remap(base.xl6),
    xl7: remap(base.xl7),
    xl8: remap(base.xl8),
  );
}

/// Outfit body, Space Grotesk display, Noto Sans Mono extension — Tauri-era stack.
FTypography mixarTypography(FTypography base) {
  final mono = _remapTypeface(
    base.body,
    MixarFonts.notoSansMono,
    fontFeatures: _monoFeatures,
  );
  return base.copyWith(
    display: _remapTypeface(
      base.display,
      MixarFonts.spaceGrotesk,
      fontFeatures: _displayFeatures,
    ),
    body: _remapTypeface(
      base.body,
      MixarFonts.outfit,
      fontFeatures: _brandFeatures,
    ),
    extensions: [mono],
  );
}

/// Forui theme with Mixar typography; colors and widget styles unchanged.
FThemeData mixarThemeData(FThemeData base, {required bool touch}) {
  return FThemeData(
    colors: base.colors,
    touch: touch,
    debugLabel: base.debugLabel,
    typography: mixarTypography(base.typography),
    icons: base.icons,
    style: base.style,
    hapticFeedback: base.hapticFeedback,
  );
}

extension MixarTypography on FTypography {
  /// Mono numerics (BPM, timers, pad slot numbers).
  FTypeface get mono => extension<FTypeface>();
}
