import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/app_typography.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Select menu opens by growing height from the top edge.
///
/// `Align.heightFactor` alone under unbounded overlay width collapses to the
/// child's intrinsic width and shifts the follower. `widthFactor: 1` keeps the
/// select's laid-out width so the portal anchor stays aligned.
final _mixarSelectOpenEffects = <AnimateEffect<dynamic>>[
  CustomEffect(
    duration: const Duration(milliseconds: 160),
    curve: Curves.easeOutCubic,
    builder: (context, value, child) {
      return ClipRect(
        child: Align(
          alignment: Alignment.topCenter,
          heightFactor: value.clamp(0.0, 1.0),
          widthFactor: 1,
          child: child,
        ),
      );
    },
  ),
  const FadeEffect(
    duration: Duration(milliseconds: 120),
    curve: Curves.easeOut,
  ),
];

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
      accent: c.accent,
      accentForeground: c.selectionForeground,
      destructive: c.destructive,
      destructiveForeground: c.destructiveForeground,
      border: c.border,
      input: c.border,
      ring: c.primary,
      // Text-selection highlight must stay translucent so glyphs show through
      // — solid [MixarColors.selection] is for row/chip fills, not caret spans.
      selection: c.primary.withValues(alpha: 0.35),
    ),
    textTheme: ShadTextTheme(family: MixarFonts.outfit),
    radius: theme.style.borderRadius.md,
    // Shad's stock option theme fills a selected/hovered row with `accent`, a
    // solid teal block that glares against the dark menu. Use the quiet Mixar
    // selection wash and keep the row text in Mixar's foreground.
    optionTheme: ShadOptionTheme(
      hoveredBackgroundColor: c.selection,
      selectedBackgroundColor: c.selection,
      selectedTextStyle: theme.typography.body.sm.copyWith(
        color: c.selectionForeground,
      ),
      selectedIconColor: c.selectionForeground,
    ),
    selectTheme: ShadSelectTheme(
      effects: _mixarSelectOpenEffects,
      popoverReverseDuration: const Duration(milliseconds: 120),
    ),
  );
}
