import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_anchor.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_body.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_group.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Sort field + direction, as one menu button next to the filter.
///
/// One click selects a field (ascending); another click on the active field
/// flips direction, like a table column header.
class TrackListSortMenu extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final (field, ascending) = ref.watch(librarySortProvider);
    final sortable = ref.watch(activeCollectionSortableProvider);
    final manual = ref.watch(playlistManualOrderProvider);

    Widget sortItem({
      required String label,
      required bool active,
      required VoidCallback onPress,
      IconData? icon,
    }) {
      return MixarMenuItem(
        // `title`, not `child`: `child` skips the row padding.
        title: Row(
          children: [
            Expanded(
              // One line per row: a wrapped label doubles the row height and
              // breaks the menu's rhythm.
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  color: active
                      ? theme.colors.primary
                      : theme.colors.foreground,
                  fontWeight: active ? FontWeight.w600 : FontWeight.w400,
                ),
              ),
            ),
            if (active && icon != null)
              Icon(icon, size: 14, color: theme.colors.primary),
          ],
        ),
        onPress: onPress,
        selected: active,
      );
    }

    return MixarMenuAnchor(
      dialogTitle: 'Sort tracks',
      // Field labels are short, but keep headroom so an arrow + accent row
      // never wraps under body.sm.
      minWidth: 160,
      menuBuilder: (context, handle) => MixarMenuBody(
        groups: [
          if (sortable)
            MixarMenuGroup(
              children: [
                sortItem(
                  label: 'Manual order',
                  active: manual,
                  icon: LucideIcons.check,
                  onPress: () {
                    handle.hide();
                    ref
                        .read(playlistManualOrderProvider.notifier)
                        .setManual(manual: true);
                  },
                ),
              ],
            ),
          MixarMenuGroup(
            children: [
              for (final option in LibrarySortField.values)
                sortItem(
                  label: option.label,
                  active: !manual && option == field,
                  // Points the way the list reads: A→Z / low→high is ↓.
                  icon: ascending ? LucideIcons.arrowDown : LucideIcons.arrowUp,
                  onPress: () {
                    handle.hide();
                    ref.read(librarySortProvider.notifier).select(option);
                  },
                ),
            ],
          ),
        ],
      ),
      childBuilder: (context, handle) => ConstrainedBox(
        // Content-sized by default, capped when the pane is narrow. A `Flexible`
        // here instead would make the button a flex child, which lets it grow
        // to a share of the toolbar instead of hugging its label.
        constraints: const BoxConstraints(maxWidth: 120),
        // `md` grows minHeight to the header (same as the 3-dots menu) so the
        // hover wash fills it. Label stays compact via explicit `body.xs` /
        // 12px arrow below, and tighter horizontal padding than `md` default —
        // `padding` only overrides contentPadding, not contentConstraints.
        child: AppButton(
          semanticsLabel: 'Sort tracks',
          onPress: handle.toggle,
          variant: .ghost,
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 11),
          // `AppButton` defaults to MainAxisSize.max, which would stretch the
          // button to the cap above instead of hugging its label.
          mainAxisSize: MainAxisSize.min,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: Text(
                  // The toolbar button is width-capped, so it keeps the short
                  // form; the menu spells out "Manual order".
                  manual ? 'Manual' : field.label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.typography.body.xs.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              const SizedBox(width: 4),
              Icon(
                // Manual order has no direction to point: the grip says the
                // order is the playlist's own.
                manual
                    ? LucideIcons.gripVertical
                    : ascending
                    ? LucideIcons.arrowDown
                    : LucideIcons.arrowUp,
                size: 12,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
