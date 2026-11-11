import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// One row in a vertical menu: icon + label, accent-highlighted while it is
/// the open page.
///
/// Shared by the settings sidebar and the app menu behind the header's
/// hamburger, so both lists read as the same navigation chrome. Full-bleed and
/// square: the selected row's fill runs to the panel's edges and its 2px accent
/// rule pins it to the leading edge.
class NavListItem extends StatelessWidget {
  const new({
    required this.label,
    required this.icon,
    required this.selected,
    required this.onPress,
    super.key,
  });

  final String label;
  final IconData icon;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;

    return MTappable(
      selected: selected,
      semanticsLabel: label,
      onPress: onPress,
      builder: (context, state) {
        final fill = selected
            ? colors.selection
            : state.hovered
            ? colors.card
            : null;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: fill,
            border: Border(
              left: BorderSide(
                width: 2,
                color: selected ? colors.primary : const Color(0x00000000),
              ),
            ),
          ),
          child: Padding(
            // A roomier row than the default chrome: the nav is a long list
            // read top to bottom, so taller rows and a wider icon gap keep the
            // labels from crowding each other and the pane's edge.
            padding: const EdgeInsets.fromLTRB(12, 12, 8, 12),
            child: Row(
              children: [
                Icon(
                  icon,
                  size: 16,
                  // Selected rows carry the brand accent on the icon only; the
                  // label stays selection-foreground so it reads as chrome.
                  color: selected ? colors.primary : colors.mutedForeground,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.typography.body.sm.copyWith(
                      color: selected
                          ? colors.selectionForeground
                          : colors.foreground,
                      fontWeight: FontWeight.w500,
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
