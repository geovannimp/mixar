import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_labels.dart';
import 'package:gui_flutter/widgets/settings/theme_swatch.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Theme option with a miniature of the app painted in that brightness.
///
/// Sized by its parent: the swatch takes a 104px band and the caption sits
/// under it. Onboarding and Settings place the same tile side by side.
class ThemeOptionTile extends StatelessWidget {
  const new({
    required this.value,
    required this.palette,
    required this.selected,
    required this.onPress,
    super.key,
  });

  final ThemeSetting value;
  final MixarAccentPalette palette;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    return MTappable(
      onPress: onPress,
      selected: selected,
      semanticsLabel: themeSettingLabel(l10n, value),
      builder: (_, state) {
        final colors = theme.colors;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Foreground: the preview fills its box, so a background border
            // would be painted over by it.
            DecoratedBox(
              position: DecorationPosition.foreground,
              decoration: BoxDecoration(
                border: Border.all(
                  color: selected
                      ? colors.primary
                      : state.hovered || state.focused
                      ? colors.accent
                      : colors.border,
                  width: selected ? 2 : theme.style.borderWidth,
                ),
              ),
              child: AspectRatio(
                aspectRatio: 16 / 9,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    ThemeSwatch(value: value, palette: palette),
                    if (selected)
                      const Positioned(
                        top: 0,
                        right: 0,
                        child: _SelectedBadge(),
                      ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 8),
            // The tile's own semantics label carries the name; the visible
            // caption must not be read twice.
            ExcludeSemantics(
              child: Text(
                themeSettingLabel(l10n, value),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.typography.body.sm.copyWith(
                  color: selected ? colors.foreground : colors.mutedForeground,
                  fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}

/// Corner tick on the selected theme tile.
class _SelectedBadge extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return SizedBox.square(
      dimension: 16,
      child: ColoredBox(
        color: theme.colors.primary,
        child: Icon(
          LucideIcons.check,
          size: 11,
          color: theme.colors.primaryForeground,
        ),
      ),
    );
  }
}
