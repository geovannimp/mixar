import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// A tip row: accent icon, title, muted body — no card chrome.
///
/// Closing tips read as guidance rather than as settings, so they stay
/// undecorated on the stage.
class OnboardingTipRow extends StatelessWidget {
  const new({
    required this.icon,
    required this.title,
    required this.subtitle,
    super.key,
  });

  final IconData icon;
  final String title;
  final String subtitle;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        spacing: 16,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(icon, size: 20, color: theme.colors.accent),
          ),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 3,
              children: [
                Text(
                  title,
                  style: theme.typography.body.md.copyWith(
                    fontWeight: FontWeight.w600,
                    height: 1.25,
                  ),
                ),
                Text(
                  subtitle,
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                    height: 1.5,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
