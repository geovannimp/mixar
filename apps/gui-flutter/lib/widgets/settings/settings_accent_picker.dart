import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Accent palette chooser — two rows of three labelled swatches.
///
/// Swatches paint each palette's own `s500` step, so the hue reads the same in
/// light and dark; selection is carried by the ring and the label weight.
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

  static String _label(AppLocalizations l10n, AccentPaletteSetting option) =>
      switch (option) {
        AccentPaletteSetting.teal => l10n.settingsUiAccentTeal,
        AccentPaletteSetting.blue => l10n.settingsUiAccentBlue,
        AccentPaletteSetting.violet => l10n.settingsUiAccentViolet,
        AccentPaletteSetting.orange => l10n.settingsUiAccentOrange,
        AccentPaletteSetting.pink => l10n.settingsUiAccentPink,
      };

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Row(
      spacing: 12,
      children: [
        for (final option in _options)
          Expanded(child: _cell(context, l10n, option)),
      ],
    );
  }

  Widget _cell(
    BuildContext context,
    AppLocalizations l10n,
    AccentPaletteSetting option,
  ) {
    final theme = context.theme;
    final selected = option == value;
    final label = _label(l10n, option);
    return MTappable(
      semanticsLabel: label,
      selected: selected,
      onPress: () => onChanged(option),
      builder: (context, state) {
        // Keyboard focus paints the same ring as selection, so tabbing through
        // the row is visible without a mouse.
        final ringed = selected || state.focused;
        final highlighted = ringed || state.hovered;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: state.hovered ? theme.colors.muted : null,
            borderRadius: theme.style.borderRadius.md,
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              spacing: 6,
              children: [
                // AspectRatio instead of a fixed 22px box: a narrow settings
                // card shrinks the swatch rather than overflowing the cell.
                ConstrainedBox(
                  constraints: const BoxConstraints(
                    maxWidth: 22,
                    maxHeight: 22,
                  ),
                  child: AspectRatio(
                    aspectRatio: 1,
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: accentPaletteFromSettings(option).scale.s500,
                        border: Border.all(
                          color: ringed
                              ? theme.colors.foreground
                              : theme.colors.border,
                          width: ringed ? 2 : theme.style.borderWidth,
                        ),
                      ),
                    ),
                  ),
                ),
                // The cell's Semantics already carries the name; the visible
                // label stays out of the tree so it is not read twice.
                ExcludeSemantics(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    textAlign: TextAlign.center,
                    style: theme.typography.body.xs.copyWith(
                      color: highlighted
                          ? theme.colors.foreground
                          : theme.colors.mutedForeground,
                      fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
