import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Narrow-layout back button: returns to the library sidebar page.
///
/// Sits left of the filter field on the track list and history detail pages.
class LibraryPaneBackButton extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return AppTooltip(
      tip: 'Back to library list',
      child: AppButton.icon(
        variant: .ghost,
        size: .xs,
        semanticsLabel: 'Back to library list',
        onPress: () {
          ref
              .read(libraryFocusZoneProvider.notifier)
              .set(LibraryFocusZone.sidebar);
          ref
              .read(libraryNarrowPageProvider.notifier)
              .set(LibraryNarrowPage.list);
        },
        child: const Icon(LucideIcons.arrowLeft, size: 16),
      ),
    );
  }
}
