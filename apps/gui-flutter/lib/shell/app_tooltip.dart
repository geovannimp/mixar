import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
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

  /// Classic white tip on dark booth chrome — square corners, dark ink.
  static const _tipBackground = Color(0xFFFFFFFF);
  static const _tipForeground = Color(0xFF18181B);
  static const _tipMuted = Color(0xFF71717A);

  static const _theme = HintThemeData(
    preset: HintPreset.minimal,
    maxWidth: 280,
    borderWidth: 0,
    borderColor: Color(0x00000000),
    borderRadius: BorderRadius.zero,
    backgroundColor: _tipBackground,
    foregroundColor: _tipForeground,
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
    return Hint(
      triggers: _triggers,
      waitDuration: _wait,
      theme: _theme,
      contentBuilder: (context) {
        final typography = context.theme.typography;
        if (detail == null || detail.isEmpty) {
          return Text(
            tip,
            style: typography.body.sm.copyWith(color: _tipForeground),
          );
        }
        return Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              tip,
              style: typography.body.sm.copyWith(color: _tipForeground),
            ),
            const SizedBox(height: 2),
            Text(detail, style: typography.body.xs.copyWith(color: _tipMuted)),
          ],
        );
      },
      child: child,
    );
  }
}
