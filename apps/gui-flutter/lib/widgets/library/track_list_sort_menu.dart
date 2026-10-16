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
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final (field, ascending) = ref.watch(librarySortProvider);
    return MixarMenuAnchor(
      dialogTitle: 'Sort tracks',
      // Field labels are short, but keep headroom so an arrow + accent row
      // never wraps under body.sm.
      minWidth: 160,
      menuBuilder: (context, handle) => MixarMenuBody(
        groups: [
          MixarMenuGroup(
            children: [
              for (final option in LibrarySortField.values)
                MixarMenuItem(
                  // `title`, not `child`: `child` skips the row padding.
                  title: Row(
                    children: [
                      Expanded(
                        // One line per row: a wrapped label doubles the row
                        // height and breaks the menu's rhythm.
                        child: Text(
                          option.label,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            color: option == field
                                ? theme.colors.primary
                                : theme.colors.foreground,
                            fontWeight: option == field
                                ? FontWeight.w600
                                : FontWeight.w400,
                          ),
                        ),
                      ),
                      if (option == field)
                        Icon(
                          // Points the way the list reads: A→Z / low→high is ↓.
                          ascending
                              ? LucideIcons.arrowDown
                              : LucideIcons.arrowUp,
                          size: 14,
                          color: theme.colors.primary,
                        ),
                    ],
                  ),
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
          size: .md,
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 11),
          // `AppButton` defaults to MainAxisSize.max, which would stretch the
          // button to the cap above instead of hugging its label.
          mainAxisSize: MainAxisSize.min,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: Text(
                  field.label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.typography.body.xs.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              const SizedBox(width: 4),
              Icon(
                ascending ? LucideIcons.arrowDown : LucideIcons.arrowUp,
                size: 12,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
