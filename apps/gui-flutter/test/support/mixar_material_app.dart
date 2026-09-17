import 'package:gui_flutter/shell/legacy_material_scope.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/shad_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

ThemeData mixarMaterialTheme(MixarThemeData theme) =>
    materialUiThemeFromMixar(theme);

/// [MaterialApp.builder] with Mixar + Shad themes + [LegacyMaterialScope].
TransitionBuilder mixarMaterialAppBuilder(
  MixarThemeData theme, {
  Widget Function(Widget child)? wrapChild,
}) {
  return (context, child) {
    final content = wrapChild != null ? wrapChild(child!) : child!;
    return LegacyMaterialScope(
      child: MixarTheme(
        data: theme,
        child: ShadTheme(data: shadThemeFromMixar(theme), child: content),
      ),
    );
  };
}
