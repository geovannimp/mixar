import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_providers.dart';

/// Forui tip gated by Settings → UI → Show tooltips (default on).
///
/// When off, returns [child] unchanged so press handlers stay intact.
class AppTooltip extends ConsumerWidget {
  const AppTooltip({
    required this.tip,
    required this.child,
    super.key,
  });

  final String tip;
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final enabled = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s.showTooltips, orElse: () => true);
    if (!enabled || tip.isEmpty) {
      return child;
    }
    return FTooltip(
      tipBuilder: (context, controller) => Text(tip),
      builder: (context, controller, child) => child!,
      child: child,
    );
  }
}
