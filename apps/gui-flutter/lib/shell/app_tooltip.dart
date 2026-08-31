import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_providers.dart';

/// Forui tip gated by Settings → UI → Show tooltips (default on).
///
/// When off, returns [child] unchanged so press handlers stay intact.
/// Optional [description] shows muted secondary copy under [tip].
class AppTooltip extends ConsumerWidget {
  const AppTooltip({
    required this.tip,
    required this.child,
    this.description,
    super.key,
  });

  final String tip;
  final String? description;
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final enabled = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s.showTooltips, orElse: () => true);
    if (!enabled || tip.isEmpty) {
      return child;
    }
    final detail = description?.trim();
    return FTooltip(
      tipBuilder: (context, controller) {
        if (detail == null || detail.isEmpty) {
          return Text(tip);
        }
        final theme = context.theme;
        return Column(
          mainAxisSize: .min,
          crossAxisAlignment: .start,
          children: [
            Text(tip),
            const SizedBox(height: 2),
            Text(
              detail,
              style: theme.typography.body.xs.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
          ],
        );
      },
      builder: (context, controller, child) => child!,
      child: child,
    );
  }
}
