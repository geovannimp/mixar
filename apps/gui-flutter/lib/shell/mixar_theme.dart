import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/shell/mixar_colors.dart';
import 'package:gui_flutter/shell/mixar_style.dart';
import 'package:shadcn_ui/shadcn_ui.dart' show ShadTheme;

/// Mixar-owned theme tokens. Seeds [ShadTheme] at the app root.
@immutable
class MixarThemeData {
  const new({
    required this.colors,
    required this.typography,
    required this.style,
  });

  factory light() {
    const colors = MixarColors.light;
    return MixarThemeData(
      colors: colors,
      typography: MixarTypography.mixar(colors.foreground),
      style: const MixarStyle(),
    );
  }

  factory dark() {
    const colors = MixarColors.dark;
    return MixarThemeData(
      colors: colors,
      typography: MixarTypography.mixar(colors.foreground),
      style: const MixarStyle(),
    );
  }

  factory forBrightness(Brightness brightness) => brightness == Brightness.dark
      ? MixarThemeData.dark()
      : MixarThemeData.light();

  final MixarColors colors;
  final MixarTypography typography;
  final MixarStyle style;
}

/// Provides [MixarThemeData] and a body [DefaultTextStyle] (Forui `FTheme` parity).
///
/// Without [DefaultTextStyle], [Text] inherits MaterialApp's debug error style
/// (yellow underline) wherever the tree skips [Material]/[Scaffold].
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
