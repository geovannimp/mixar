import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Tauri `HOT_CUE_ACCENTS` slot colors (border / fill / text).
({Color border, Color fill, Color text}) hotCueAccent(int slot) {
  const accents = <(int, int, int)>[
    (239, 68, 68), // red-500
    (249, 115, 22), // orange-500
    (234, 179, 8), // yellow-500
    (34, 197, 94), // green-500
    (6, 182, 212), // cyan-500
    (59, 130, 246), // blue-500
    (139, 92, 246), // violet-500
    (236, 72, 153), // pink-500
  ];
  final rgb = accents[((slot % 8) + 8) % 8];
  return (
    border: Color.fromRGBO(rgb.$1, rgb.$2, rgb.$3, 0.55),
    fill: Color.fromRGBO(rgb.$1, rgb.$2, rgb.$3, 0.20),
    text: Color.fromRGBO(
      (rgb.$1 + 255) ~/ 2,
      (rgb.$2 + 255) ~/ 2,
      (rgb.$3 + 255) ~/ 2,
      1,
    ),
  );
}

/// Content colour on a saturated pad fill (cue/stem accents).
///
/// Fixed like `kStemColors`: these fills are hardware palette colours, not
/// theme surfaces, so the theme's on-primary token (near-black in dark) would
/// be unreadable on them.
const kPadOnAccentColor = Color(0xFFFFFFFF);

/// Resting chrome for pads that own an accent colour (hot cue, sampler).
PadChrome accentPadChrome(
  MixarThemeData theme,
  ({Color border, Color fill, Color text}) accent,
) {
  // The accent fill is a 20% tint. Mix from its opaque RGB, or the lerp would
  // carry that alpha into the hairline and reintroduce the surface-dependent
  // shade this token exists to remove.
  final accentBase = accent.fill.withValues(alpha: 1);
  return PadChrome(
    // color-mix(in srgb, accent 40%, border), resolved to a solid.
    border: Color.lerp(theme.colors.border, accentBase, 0.4)!,
    fill: theme.colors.card,
    foreground: theme.colors.foreground,
    hoverBorder: accent.border,
    pressedFill: accent.fill.withValues(alpha: 1),
    pressedForeground: kPadOnAccentColor,
  );
}

/// Explicit per-pad chrome for modes that own their palette (Stems).
///
/// [border] / [fill] / [foreground] paint the resting pad; [hoverBorder]
/// replaces the border while the pointer is inside; [pressedFill] fills the pad
/// while it is held (and forces white content). Disabled pads ignore it.
@immutable
class PadChrome {
  const new({
    required this.border,
    required this.fill,
    required this.foreground,
    this.hoverBorder,
    this.pressedFill,
    this.pressedForeground,
  });

  final Color border;
  final Color fill;
  final Color foreground;
  final Color? hoverBorder;
  final Color? pressedFill;

  /// Content colour while [pressedFill] is active; defaults to
  /// [kPadOnAccentColor] for pad-owned fills.
  final Color? pressedForeground;
}

/// Pad-sized button matching Tauri `DeckButton` size=`pad`.
class PadButton extends StatefulWidget {
  const new({
    required this.child,
    this.onPress,
    this.onPointerDown,
    this.onPointerUp,
    this.onPointerCancel,
    this.onPointerExit,
    this.disabled = false,
    this.active = false,
    this.chrome,
    this.tooltip,
    this.fillChild = false,
    super.key,
  });

  final Widget child;

  /// Lays [child] out with the pad's tight constraints instead of centering
  /// and scaling it down. Use for children that position overlays themselves.
  final bool fillChild;
  final VoidCallback? onPress;
  final VoidCallback? onPointerDown;
  final VoidCallback? onPointerUp;
  final VoidCallback? onPointerCancel;
  final VoidCallback? onPointerExit;
  final bool disabled;

  /// Latched single-select state (e.g. the active semitone): the pad keeps its
  /// card fill and wears a primary-colour border.
  final bool active;
  final PadChrome? chrome;
  final String? tooltip;

  @override
  State<PadButton> createState() => _PadButtonState();
}

class _PadButtonState extends State<PadButton> {
  bool _hovered = false;
  bool _pressed = false;

  @override
  void didUpdateWidget(covariant PadButton oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Disabling the pad removes its pointer handlers, so a held pad would
    // otherwise re-enable looking permanently lit.
    if (widget.disabled) {
      _hovered = false;
      _pressed = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final disabled = widget.disabled;
    final chrome = widget.chrome;
    final hoverable = !disabled && chrome?.hoverBorder != null;
    // Pressing any pad lights it with the shared primary; pads that own a
    // colour (cue/sampler/stems) highlight with it instead.
    final pressedFill = !disabled && _pressed
        ? (chrome?.pressedFill ?? theme.colors.primary)
        : null;
    final activeBorder = !disabled && widget.active
        ? theme.colors.primary
        : null;
    final border = disabled
        ? theme.colors.border
        : pressedFill ??
              (_hovered && hoverable
                  ? chrome!.hoverBorder!
                  : (chrome?.border ?? activeBorder ?? theme.colors.border));
    // Single-select / latched accents only stroke the border; they never fill
    // the pad at rest.
    final fill = disabled
        ? theme.colors.secondary.withValues(alpha: 0.35)
        : pressedFill ?? chrome?.fill ?? theme.colors.card;
    final fg = disabled
        ? theme.colors.mutedForeground
        : pressedFill != null
        ? (chrome?.pressedForeground ??
              chrome?.foreground ??
              theme.colors.primaryForeground)
        : (chrome?.foreground ?? theme.colors.foreground);

    Widget pad = MouseRegion(
      onEnter: hoverable ? (_) => setState(() => _hovered = true) : null,
      onExit: hoverable || (!disabled && widget.onPointerExit != null)
          ? (_) {
              if (_hovered || _pressed) {
                setState(() {
                  _hovered = false;
                  _pressed = false;
                });
              }
              if (!disabled) {
                widget.onPointerExit?.call();
              }
            }
          : null,
      child: Listener(
        behavior: HitTestBehavior.opaque,
        onPointerDown: disabled
            ? null
            : (_) {
                setState(() => _pressed = true);
                widget.onPointerDown?.call();
              },
        onPointerUp: disabled
            ? null
            : (_) {
                setState(() => _pressed = false);
                widget.onPointerUp?.call();
              },
        onPointerCancel: disabled
            ? null
            : (_) {
                setState(() => _pressed = false);
                widget.onPointerCancel?.call();
              },
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: disabled ? null : widget.onPress,
          child: SizedBox.expand(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: fill,
                border: Border.all(color: border),
                borderRadius: theme.style.borderRadius.md,
              ),
              child: DefaultTextStyle.merge(
                style: TextStyle(color: fg),
                // shortcut: Inter metrics + two-line cue labels can exceed
                // the square cell by <1px on Flutter 3.47; scale down.
                child: widget.fillChild
                    ? widget.child
                    : Center(
                        child: FittedBox(fit: .scaleDown, child: widget.child),
                      ),
              ),
            ),
          ),
        ),
      ),
    );

    if (widget.tooltip != null && widget.tooltip!.isNotEmpty) {
      // One Semantics node; ExcludeSemantics keeps the tooltip chrome quiet.
      // onTap restores activation after ExcludeSemantics hides GestureDetector.
      final onActivate = disabled
          ? null
          : (widget.onPress == null && widget.onPointerDown == null)
          ? null
          : () {
              widget.onPointerDown?.call();
              widget.onPress?.call();
              widget.onPointerUp?.call();
            };
      pad = Semantics(
        label: widget.tooltip,
        button: true,
        enabled: !disabled,
        onTap: onActivate,
        child: AppTooltip(
          tip: widget.tooltip!,
          child: ExcludeSemantics(child: pad),
        ),
      );
    }
    return pad;
  }
}
