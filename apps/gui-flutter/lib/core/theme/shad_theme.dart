import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/app_typography.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Select menu reveals by clipping a full-size panel, not by shrinking its
/// layout height.
///
/// The portal anchors the overlay using the panel's measured size. Animating
/// the layout height makes an upward-opening menu anchor to a half-grown size
/// and overlap the trigger. A clip keeps the full size from the first frame,
/// so the fallback anchor can seat the panel flush against the trigger.
List<AnimateEffect<dynamic>> mixarSelectOpenEffects({
  Alignment alignment = Alignment.topCenter,
}) {
  return <AnimateEffect<dynamic>>[
    CustomEffect(
      duration: const Duration(milliseconds: 160),
      curve: Curves.easeOutCubic,
      builder: (context, value, child) {
        return ClipRect(
          clipper: _MixarSelectRevealClipper(
            alignment: alignment,
            factor: value.clamp(0.0, 1.0),
          ),
          child: child,
        );
      },
    ),
    const FadeEffect(
      duration: Duration(milliseconds: 120),
      curve: Curves.easeOut,
    ),
  ];
}

/// Reveals a panel from its top ([Alignment.topCenter]) or bottom
/// ([Alignment.bottomCenter]) edge without changing its layout size.
class _MixarSelectRevealClipper extends CustomClipper<Rect> {
  const new({required this.alignment, required this.factor});

  final Alignment alignment;
  final double factor;

  @override
  Rect getClip(Size size) {
    final height = size.height * factor;
    return alignment == Alignment.bottomCenter
        ? Rect.fromLTWH(0, size.height - height, size.width, height)
        : Rect.fromLTWH(0, 0, size.width, height);
  }

  @override
  bool shouldReclip(_MixarSelectRevealClipper oldClipper) =>
      oldClipper.alignment != alignment || oldClipper.factor != factor;
}

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
    // Fields wear the full pill like select controls and labelled app buttons.
    inputTheme: ShadInputTheme(decoration: _pillDecoration(theme)),
    // Options read as MixarMenuItem rows: full-bleed bands, 44px tall, a
    // hairline between choices, and a dark-grey hover. Selection is ink-only —
    // the label and trailing check take the accent; the row keeps the card.
    // The hover paint lives in `_SelectOptionRow` because Shad treats keyboard
    // focus as hover; this theme only supplies the row typography.
    optionTheme: ShadOptionTheme(
      hoveredBackgroundColor: const Color(0x00000000),
      selectedBackgroundColor: const Color(0x00000000),
      backgroundColor: const Color(0x00000000),
      textStyle: theme.typography.body.sm.copyWith(color: c.foreground),
      // The trigger reads this style for its selected value; the accent is for
      // the open list's active row only, supplied per ShadOption.
      selectedTextStyle: theme.typography.body.sm.copyWith(color: c.foreground),
      padding: EdgeInsets.zero,
      radius: BorderRadius.zero,
    ),
    selectTheme: ShadSelectTheme(
      effects: mixarSelectOpenEffects(),
      popoverReverseDuration: const Duration(milliseconds: 120),
      // The rows and their rules span the panel like a Mixar menu body.
      optionsPadding: EdgeInsets.zero,
      // Sit flush against the trigger so the open select reads as one card.
      // The fallback stays above the trigger when there is no room below.
      anchor: const ShadAnchorAuto(
        fallback: ShadAnchorAuto(
          followerAnchor: Alignment.topCenter,
          targetAnchor: Alignment.topCenter,
        ),
      ),
      // Selects share the input pill, focus ring included.
      decoration: _pillDecoration(theme),
    ),
  );
}

/// Border and focus ring shared by pill-shaped controls: text fields and
/// selects.
///
/// Only the radii are set, so the Shad default strokes keep their colour,
/// width, and focus offset; the error state falls back to the base border. The
/// secondary ring is explicit because the global decoration theme draws it on
/// the square radius.
ShadDecoration _pillDecoration(MixarThemeData theme) {
  final pill = ShadBorder(radius: theme.style.borderRadius.pill);
  return ShadDecoration(border: pill, secondaryFocusedBorder: pill);
}
