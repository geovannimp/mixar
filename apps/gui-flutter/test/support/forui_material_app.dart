import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/shell/legacy_material_scope.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';

ThemeData foruiMaterialTheme(FThemeData theme, {bool touch = false}) =>
    materialUiThemeFromForui(mixarThemeData(theme, touch: touch));

/// [MaterialApp.builder] with Forui theme + [LegacyMaterialScope] for legacy deps.
TransitionBuilder foruiMaterialAppBuilder(
  FThemeData theme, {
  Widget Function(Widget child)? wrapChild,
}) {
  return (context, child) {
    final content = wrapChild != null ? wrapChild(child!) : child!;
    return LegacyMaterialScope(
      child: FTheme(data: mixarThemeData(theme, touch: false), child: content),
    );
  };
}
