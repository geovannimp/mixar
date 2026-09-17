import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/shell/mixar_colors.dart';
import 'package:gui_flutter/shell/mixar_style.dart';

/// Mixar-owned theme tokens. Seeds [ShadTheme] at the app root.
@immutable
class MixarThemeData {
  const MixarThemeData({
    required this.colors,
    required this.typography,
    required this.style,
  });

  factory MixarThemeData.light() {
    const colors = MixarColors.light;
    return MixarThemeData(
      colors: colors,
      typography: MixarTypography.mixar(colors.foreground),
      style: const MixarStyle(),
    );
  }

  factory MixarThemeData.dark() {
    const colors = MixarColors.dark;
    return MixarThemeData(
      colors: colors,
      typography: MixarTypography.mixar(colors.foreground),
      style: const MixarStyle(),
    );
  }

  factory MixarThemeData.forBrightness(Brightness brightness) =>
      brightness == Brightness.dark
      ? MixarThemeData.dark()
      : MixarThemeData.light();

  final MixarColors colors;
  final MixarTypography typography;
  final MixarStyle style;
}

/// Provides [MixarThemeData] to descendants. Prefer [context.theme].
class MixarTheme extends InheritedWidget {
  const MixarTheme({required this.data, required super.child, super.key});

  final MixarThemeData data;

  static MixarThemeData of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<MixarTheme>();
    assert(scope != null, 'No MixarTheme found in context');
    return scope!.data;
  }

  static MixarThemeData? maybeOf(BuildContext context) {
    return context.dependOnInheritedWidgetOfExactType<MixarTheme>()?.data;
  }

  @override
  bool updateShouldNotify(MixarTheme oldWidget) => data != oldWidget.data;
}

extension MixarThemeContext on BuildContext {
  /// Mixar theme tokens (replaces Forui `context.theme`).
  MixarThemeData get theme => MixarTheme.of(this);
}
