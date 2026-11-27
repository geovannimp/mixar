import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_labels.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart' show Theme;

/// Accent palette chooser — one segmented pill, like the deck selector.
///
/// A single bordered track split by hairlines holds the five palettes. Each
/// segment paints its own tone: the dot is the palette's `s500` step and the
/// label takes [textColorStep], so the hue reads the same in light and dark.
/// Selection is carried by the palette's own tint behind the segment plus the
/// heavier label.
class SettingsAccentPicker extends StatelessWidget {
  const new({required this.value, required this.onChanged, super.key});

  final AccentPaletteSetting value;
  final ValueChanged<AccentPaletteSetting> onChanged;

  static const _options = <AccentPaletteSetting>[
    AccentPaletteSetting.teal,
    AccentPaletteSetting.blue,
    AccentPaletteSetting.violet,
    AccentPaletteSetting.orange,
    AccentPaletteSetting.pink,
  ];

  /// Below this width a segment cannot hold the dot without crushing the
  /// label, so the dot drops out and the label keeps the space.
  static const _dotBreakpoint = 56.0;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final radius = theme.style.borderRadius.pill;
    return LayoutBuilder(
      builder: (context, constraints) {
        final showDots =
            !constraints.hasBoundedWidth ||
            constraints.maxWidth / _options.length >= _dotBreakpoint;
        return DecoratedBox(
          decoration: BoxDecoration(
            border: Border.all(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
            borderRadius: radius,
          ),
          child: ClipRRect(
            borderRadius: radius,
            child: IntrinsicHeight(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  for (var i = 0; i < _options.length; i++) ...[
                    if (i > 0)
                      MDivider(
                        axis: Axis.vertical,
                        padding: EdgeInsets.zero,
                        color: theme.colors.border,
                      ),
                    Expanded(
                      child: _AccentSegment(
                        palette: _options[i],
                        label: accentPaletteLabel(l10n, _options[i]),
                        selected: _options[i] == value,
                        showDot: showDots,
                        onPress: () => onChanged(_options[i]),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

class _AccentSegment extends StatelessWidget {
  const new({
    required this.palette,
    required this.label,
    required this.selected,
    required this.showDot,
    required this.onPress,
  });

  final AccentPaletteSetting palette;
  final String label;
  final bool selected;
  final bool showDot;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final brightness = Theme.brightnessOf(context);
    final scale = accentPaletteFromSettings(palette).scale;
    return MTappable(
      onPress: onPress,
      selected: selected,
      semanticsLabel: label,
      builder: (context, state) {
        return DecoratedBox(
          // Focus paints over the fill: the ring is always readable, and the
          // track's hairlines stay underneath it.
          position: DecorationPosition.foreground,
          decoration: BoxDecoration(
            border: state.focused
                ? Border.all(color: theme.colors.foreground, width: 1.5)
                : null,
          ),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: selected
                  ? (brightness == Brightness.dark ? scale.s950 : scale.s100)
                  : state.hovered || state.pressed
                  ? theme.hoverWash
                  : const Color(0x00000000),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 10),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  if (showDot) ...[
                    SizedBox.square(
                      dimension: 10,
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          shape: BoxShape.circle,
                          color: scale.s500,
                          border: Border.all(
                            color: theme.colors.border,
                            width: theme.style.borderWidth,
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                  ],
                  Flexible(
                    // The cell's Semantics already carries the name; the
                    // visible label stays out of the tree so it is not read
                    // twice.
                    child: ExcludeSemantics(
                      child: Text(
                        label,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        textAlign: TextAlign.center,
                        style: theme.typography.body.sm.copyWith(
                          color: textColorStep(scale, brightness),
                          fontWeight: selected
                              ? FontWeight.w600
                              : FontWeight.w500,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}
