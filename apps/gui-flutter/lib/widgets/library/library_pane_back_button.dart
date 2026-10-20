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
      // `md` (the default) matches the density button and the trailing menu, so
      // the hover wash fills the header height; `xs` left a centred chip with
      // gaps above and below.
      child: AppButton.icon(
        variant: .ghost,
        semanticsLabel: 'Back to library list',
        onPress: () {
          ref
              .read(libraryFocusZoneProvider.notifier)
              .set(LibraryFocusZone.sidebar);
          ref
              .read(libraryNarrowPageProvider.notifier)
              .set(LibraryNarrowPage.list);
        },
        child: const Icon(LucideIcons.arrowLeft),
      ),
    );
  }
}
