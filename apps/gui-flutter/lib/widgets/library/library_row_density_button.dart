import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Session-only density switch. Deliberately does not write settings.
class LibraryRowDensityButton extends ConsumerWidget {
  const new({required this.density, super.key});

  final LibraryRowDensity density;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final target = density.other;
    return AppTooltip(
      tip: target == LibraryRowDensity.compact
          ? 'Compact rows'
          : 'Comfortable rows',
      // `md` matches the trailing 3-dots menu so the hover wash fills the
      // header height; `xs` left a centred chip with gaps above and below.
      child: AppButton.icon(
        semanticsLabel: 'Toggle row layout',
        onPress: () => ref
            .read(libraryRowDensityOverrideProvider.notifier)
            .toggle(density),
        variant: .ghost,
        child: Icon(density.isCompact ? LucideIcons.list : LucideIcons.rows3),
      ),
    );
  }
}
