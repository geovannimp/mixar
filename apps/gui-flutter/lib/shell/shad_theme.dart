import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Map Mixar tokens into [ShadThemeData] so shadcn widgets match chrome.
///
/// Uses Mixar's Forui-parity palette (not stock [ShadNeutralColorScheme]).
ShadThemeData shadThemeFromMixar(MixarThemeData theme) {
  final c = theme.colors;
  return ShadThemeData(
    brightness: c.brightness,
    colorScheme: ShadColorScheme(
      background: c.background,
      foreground: c.foreground,
      card: c.card,
      cardForeground: c.foreground,
      popover: c.card,
      popoverForeground: c.foreground,
      primary: c.primary,
      primaryForeground: c.primaryForeground,
      secondary: c.secondary,
      secondaryForeground: c.secondaryForeground,
      muted: c.muted,
      mutedForeground: c.mutedForeground,
      accent: c.secondary,
      accentForeground: c.secondaryForeground,
      destructive: c.destructive,
      destructiveForeground: c.destructiveForeground,
      border: c.border,
      input: c.border,
      ring: c.primary,
      selection: c.primary.withValues(alpha: 0.35),
    ),
    textTheme: ShadTextTheme(family: MixarFonts.outfit),
    radius: theme.style.borderRadius.md,
  );
}
