import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/app_typography.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/core/theme/mixar_colors.dart';
import 'package:gui_flutter/core/theme/mixar_palette.dart';
import 'package:gui_flutter/core/theme/mixar_style.dart';
import 'package:shadcn_ui/shadcn_ui.dart' show ShadTheme;

/// Mixar-owned theme tokens. Seeds [ShadTheme] at the app root.
///
/// Factories return stable singletons so inherited MixarTheme notify stays
/// false across MaterialApp.builder frames that keep the same brightness.
@immutable
class MixarThemeData {
  const new({
    required this.colors,
    required this.typography,
    required this.style,
  });

  factory light() => _light;
  factory dark() => _dark;

  factory forBrightness(Brightness brightness) =>
      brightness == Brightness.dark ? _dark : _light;

  /// Palette-seeded theme. The teal palette returns the stock singleton, so
  /// existing `light()`/`dark()` call sites are unchanged.
  factory of(MixarAccentPalette palette, Brightness brightness) {
    final base = MixarThemeData.forBrightness(brightness);
    return palette == MixarAccentPalette.teal
        ? base
        : _accented.putIfAbsent(
            (palette, brightness),
            () => MixarThemeData(
              colors: base.colors.withAccentScale(palette.scale),
              // Neutrals and text colour are unchanged, so reuse the graph.
              typography: base.typography,
              style: base.style,
            ),
          );
  }

  static final _accented = <(MixarAccentPalette, Brightness), MixarThemeData>{};

  static final _light = MixarThemeData(
    colors: MixarColors.light,
    typography: MixarTypography.mixar(MixarColors.light.foreground),
    style: const MixarStyle(),
  );

  static final _dark = MixarThemeData(
    colors: MixarColors.dark,
    typography: MixarTypography.mixar(MixarColors.dark.foreground),
    style: const MixarStyle(),
  );

  final MixarColors colors;
  final MixarTypography typography;
  final MixarStyle style;

  /// Translucent neutral hover wash shared by the accent picker and ghost
  /// buttons. Alpha (not a solid step) so the same wash reads on card,
  /// secondary, and background surfaces.
  Color get hoverWash => colors.foreground.withValues(alpha: 0.05);

  /// Dark-theme knob face shared by the rotary and fader controls: half a
  /// palette step darker than zinc-700.
  Color get _darkControlFace =>
      Color.lerp(MixarZinc.s700, MixarZinc.s800, 0.5)!;

  /// Fader grip face. The light theme keeps the lighter mid-grey plate so the
  /// control does not read as a heavy block on pale chrome.
  Color get faderFace =>
      colors.brightness == Brightness.dark ? _darkControlFace : MixarZinc.s400;

  /// Rotary face. The light theme keeps the warm secondary→foreground blend,
  /// at a lighter ratio than the original 0.28 so the disc sits close to the
  /// pale chrome while staying distinct from the track halo.
  Color get rotaryFace => colors.brightness == Brightness.dark
      ? _darkControlFace
      : Color.lerp(colors.secondary, colors.foreground, 0.12)!;
}

/// Provides [MixarThemeData] and a body [DefaultTextStyle] (Forui `FTheme` parity).
///
/// Without [DefaultTextStyle], [Text] inherits MaterialApp's debug error style
/// (yellow underline) wherever the tree skips `Material`/`Scaffold`.
class MixarTheme extends StatelessWidget {
  const new({required this.data, required this.child, super.key});

  final MixarThemeData data;
  final Widget child;

  static MixarThemeData of(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<_InheritedMixarTheme>();
    assert(scope != null, 'No MixarTheme found in context');
    return scope!.data;
  }

  static MixarThemeData? maybeOf(BuildContext context) {
    return context
        .dependOnInheritedWidgetOfExactType<_InheritedMixarTheme>()
        ?.data;
  }

  @override
  Widget build(BuildContext context) {
    return _InheritedMixarTheme(
      data: data,
      child: DefaultTextStyle(
        style: data.typography.body.sm.copyWith(
          color: data.colors.foreground,
          decoration: TextDecoration.none,
        ),
        child: child,
      ),
    );
  }
}

class _InheritedMixarTheme extends InheritedWidget {
  const new({required this.data, required super.child});

  final MixarThemeData data;

  @override
  bool updateShouldNotify(_InheritedMixarTheme oldWidget) =>
      data != oldWidget.data;
}

extension MixarThemeContext on BuildContext {
  /// Mixar theme tokens (replaces Forui `context.theme`).
  MixarThemeData get theme => MixarTheme.of(this);
}
