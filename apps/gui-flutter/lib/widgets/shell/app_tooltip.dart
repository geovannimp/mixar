import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:hint_kit/hint_kit.dart';

/// Tip gated by Settings → UI → Show tooltips (default on).
///
/// When off, returns [child] unchanged so press handlers stay intact.
/// Optional [description] shows muted secondary copy under [tip].
///
/// Uses [hint_kit](https://pub.dev/packages/hint_kit) instead of Forui tooltips.
class AppTooltip extends ConsumerWidget {
  const new({
    required this.tip,
    required this.child,
    this.description,
    super.key,
  });

  final String tip;
  final String? description;
  final Widget child;

  static const Set<HintTrigger> _triggers = {HintTrigger.hover};
  static const _wait = Duration(milliseconds: 400);

  /// Raised-panel tip: `card` fill, hairline `border` rim, square corners.
  /// Tones come from the active theme so the bubble sits on the page in both
  /// light and dark instead of being a fixed white chip.
  ///
  /// hint_kit strokes the bubble silhouette centred on its path, so half of a
  /// `borderWidth` stroke falls outside the fill and blends with the page.
  /// Doubling the chrome hairline leaves one full-strength token-width rim
  /// inside the bubble, matching `Border.all(width: style.borderWidth)`
  /// elsewhere.
  static HintThemeData _themeFor(MixarThemeData theme) => HintThemeData(
    preset: HintPreset.minimal,
    maxWidth: 280,
    borderWidth: theme.style.borderWidth * 2,
    borderColor: theme.colors.border,
    borderRadius: BorderRadius.zero,
    backgroundColor: theme.colors.card,
    foregroundColor: theme.colors.foreground,
    transition: HintTransition.fade,
  );

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final enabled = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s.showTooltips, orElse: () => true);
    if (!enabled || tip.isEmpty) {
      return child;
    }
    final detail = description?.trim();
    final theme = context.theme;
    return Hint(
      triggers: _triggers,
      waitDuration: _wait,
      theme: _themeFor(theme),
      contentBuilder: (context) {
        final typography = context.theme.typography;
        if (detail == null || detail.isEmpty) {
          return Text(
            tip,
            style: typography.body.sm.copyWith(color: theme.colors.foreground),
          );
        }
        return Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              tip,
              style: typography.body.sm.copyWith(
                color: theme.colors.foreground,
              ),
            ),
            const SizedBox(height: 2),
            Text(
              detail,
              style: typography.body.xs.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
          ],
        );
      },
      child: child,
    );
  }
}
