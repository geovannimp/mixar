import 'package:flutter/material.dart' as legacy;
import 'package:flutter/widgets.dart';
import 'package:material_ui/material_ui.dart' as modern;

/// Maps [modern.ThemeData] into a legacy `package:flutter/material` theme for
/// dependencies (e.g. trina_grid) that still read `Theme.of` from material.dart.
///
/// ponytail: local copy of material_ui's deprecated bridge until a supported
/// replacement ships; delete when trina_grid migrates to package:material_ui.
class LegacyMaterialScope extends StatelessWidget {
  const LegacyMaterialScope({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final modern.ThemeData modernTheme = modern.Theme.of(context);

    return legacy.Theme(
      data: _mapToLegacy(modernTheme),
      child: legacy.Localizations.override(
        context: context,
        delegates: modern.GlobalMaterialLocalizations.delegates,
        child: child,
      ),
    );
  }

  legacy.ThemeData _mapToLegacy(modern.ThemeData modernTheme) {
    final modern.ColorScheme scheme = modernTheme.colorScheme;
    final modern.TextTheme textTheme = modernTheme.textTheme;

    return legacy.ThemeData(
      platform: modernTheme.platform,
      visualDensity: legacy.VisualDensity(
        horizontal: modernTheme.visualDensity.horizontal,
        vertical: modernTheme.visualDensity.vertical,
      ),
      colorScheme: legacy.ColorScheme(
        brightness: scheme.brightness,
        primary: scheme.primary,
        onPrimary: scheme.onPrimary,
        primaryContainer: scheme.primaryContainer,
        onPrimaryContainer: scheme.onPrimaryContainer,
        secondary: scheme.secondary,
        onSecondary: scheme.onSecondary,
        secondaryContainer: scheme.secondaryContainer,
        onSecondaryContainer: scheme.onSecondaryContainer,
        tertiary: scheme.tertiary,
        onTertiary: scheme.onTertiary,
        tertiaryContainer: scheme.tertiaryContainer,
        onTertiaryContainer: scheme.onTertiaryContainer,
        error: scheme.error,
        onError: scheme.onError,
        errorContainer: scheme.errorContainer,
        onErrorContainer: scheme.onErrorContainer,
        surface: scheme.surface,
        onSurface: scheme.onSurface,
        surfaceDim: scheme.surfaceDim,
        surfaceBright: scheme.surfaceBright,
        surfaceContainerLowest: scheme.surfaceContainerLowest,
        surfaceContainerLow: scheme.surfaceContainerLow,
        surfaceContainer: scheme.surfaceContainer,
        surfaceContainerHigh: scheme.surfaceContainerHigh,
        surfaceContainerHighest: scheme.surfaceContainerHighest,
        onSurfaceVariant: scheme.onSurfaceVariant,
        outline: scheme.outline,
        outlineVariant: scheme.outlineVariant,
        shadow: scheme.shadow,
        scrim: scheme.scrim,
        inverseSurface: scheme.inverseSurface,
        onInverseSurface: scheme.onInverseSurface,
        inversePrimary: scheme.inversePrimary,
        surfaceTint: scheme.surfaceTint,
      ),
      textTheme: legacy.TextTheme(
        displayLarge: textTheme.displayLarge,
        displayMedium: textTheme.displayMedium,
        displaySmall: textTheme.displaySmall,
        headlineLarge: textTheme.headlineLarge,
        headlineMedium: textTheme.headlineMedium,
        headlineSmall: textTheme.headlineSmall,
        titleLarge: textTheme.titleLarge,
        titleMedium: textTheme.titleMedium,
        titleSmall: textTheme.titleSmall,
        bodyLarge: textTheme.bodyLarge,
        bodyMedium: textTheme.bodyMedium,
        bodySmall: textTheme.bodySmall,
        labelLarge: textTheme.labelLarge,
        labelMedium: textTheme.labelMedium,
        labelSmall: textTheme.labelSmall,
      ),
      scaffoldBackgroundColor: modernTheme.scaffoldBackgroundColor,
    );
  }
}
