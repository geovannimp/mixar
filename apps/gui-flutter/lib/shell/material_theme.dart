import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

/// Build `material_ui` [ThemeData] from Mixar tokens.
ThemeData materialUiThemeFromMixar(
  MixarThemeData theme, {
  Color? scaffoldBackgroundColor,
}) {
  final c = theme.colors;
  final body = theme.typography.body;
  final display = theme.typography.display;

  final textTheme = TextTheme(
    displayLarge: display.xl5,
    displayMedium: display.xl4,
    displaySmall: display.xl3,
    headlineLarge: display.xl2,
    headlineMedium: display.xl,
    headlineSmall: display.lg,
    titleLarge: body.lg,
    titleMedium: body.md,
    titleSmall: body.sm,
    bodyLarge: body.md,
    bodyMedium: body.sm,
    bodySmall: body.xs,
    labelLarge: body.sm,
    labelMedium: body.xs,
    labelSmall: body.xs2,
  );

  return ThemeData(
    colorScheme: ColorScheme(
      brightness: c.brightness,
      primary: c.primary,
      onPrimary: c.primaryForeground,
      secondary: c.secondary,
      onSecondary: c.secondaryForeground,
      error: c.destructive,
      onError: c.destructiveForeground,
      surface: c.background,
      onSurface: c.foreground,
      secondaryContainer: c.muted,
      onSecondaryContainer: c.mutedForeground,
    ),
    fontFamily: body.fontFamily,
    fontFamilyFallback: body.fontFamilyFallback,
    textTheme: textTheme,
    splashFactory: NoSplash.splashFactory,
    useMaterial3: true,
    scaffoldBackgroundColor: scaffoldBackgroundColor,
  );
}
