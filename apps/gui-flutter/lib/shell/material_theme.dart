import 'package:flutter/material.dart' as flutter_material;
import 'package:forui/forui.dart';
import 'package:material_ui/material_ui.dart';

/// Forui's [FThemeData.toApproximateMaterialTheme] still returns SDK
/// `package:flutter/material` [ThemeData]. Rebuild as `material_ui` [ThemeData].
ThemeData materialUiThemeFromForui(
  FThemeData theme, {
  Color? scaffoldBackgroundColor,
}) {
  final flutter_material.ThemeData legacy = theme.toApproximateMaterialTheme();
  final flutter_material.ColorScheme c = legacy.colorScheme;
  final flutter_material.TextTheme t = legacy.textTheme;

  final textTheme = TextTheme(
    displayLarge: t.displayLarge,
    displayMedium: t.displayMedium,
    displaySmall: t.displaySmall,
    headlineLarge: t.headlineLarge,
    headlineMedium: t.headlineMedium,
    headlineSmall: t.headlineSmall,
    titleLarge: t.titleLarge,
    titleMedium: t.titleMedium,
    titleSmall: t.titleSmall,
    bodyLarge: t.bodyLarge,
    bodyMedium: t.bodyMedium,
    bodySmall: t.bodySmall,
    labelLarge: t.labelLarge,
    labelMedium: t.labelMedium,
    labelSmall: t.labelSmall,
  );

  return ThemeData(
    colorScheme: ColorScheme(
      brightness: c.brightness,
      primary: c.primary,
      onPrimary: c.onPrimary,
      secondary: c.secondary,
      onSecondary: c.onSecondary,
      error: c.error,
      onError: c.onError,
      surface: c.surface,
      onSurface: c.onSurface,
      secondaryContainer: c.secondaryContainer,
      onSecondaryContainer: c.onSecondaryContainer,
    ),
    fontFamily: t.bodyMedium?.fontFamily ?? theme.typography.body.fontFamily,
    fontFamilyFallback:
        t.bodyMedium?.fontFamilyFallback ?? theme.typography.body.fontFamilyFallback,
    textTheme: textTheme,
    splashFactory: NoSplash.splashFactory,
    useMaterial3: true,
    scaffoldBackgroundColor: scaffoldBackgroundColor,
  );
}
